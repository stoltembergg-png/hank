import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import test from 'node:test';

import { createGithubApi, GithubApiError } from './github-api.mjs';
import { findingFingerprint, findingLineage } from './contracts.mjs';
import {
  MAX_REMEDIATION_CYCLES,
  buildEvidenceDescriptor,
  buildProposalInput,
  collectFinding,
  proposeFinding,
  publishValidated,
  validateProposal,
} from './orchestrator.mjs';

const repository = 'stoltembergg-png/hank';
const headSha = 'a'.repeat(40);
const findingPath = 'crates/agent-core/src/lib.rs';
const remediationPatch = [
  'diff --git a/src/value.txt b/src/value.txt',
  '--- a/src/value.txt',
  '+++ b/src/value.txt',
  '@@ -1,1 +1,1 @@',
  '-before',
  '+after',
  '',
].join('\n');

function pullRequest(overrides = {}) {
  return {
    number: 401,
    state: 'open',
    draft: false,
    head: { ref: 'feature/fix-review', sha: headSha, repo: { full_name: repository } },
    base: { ref: 'main', repo: { full_name: repository } },
    ...overrides,
  };
}

function fakeApi(overrides = {}) {
  return {
    getPullRequest: async () => pullRequest(),
    getReviewComments: async () => [],
    getIssueComments: async () => [],
    getCheckAnnotations: async () => [],
    getPullRequestFiles: async () => [],
    getBranch: async () => null,
    ...overrides,
  };
}

function codeRabbitEvent(overrides = {}) {
  return {
    eventName: 'pull_request_review',
    action: 'submitted',
    repository: { full_name: repository },
    pull_request: pullRequest(),
    review: {
      id: 77,
      state: 'changes_requested',
      user: { login: 'coderabbitai[bot]' },
      body: 'Inline review',
      commit_id: headSha,
      html_url: `https://github.com/${repository}/pull/401#pullrequestreview-77`,
    },
    ...overrides,
  };
}

function codeRabbitComment(overrides = {}) {
  return {
    id: 88,
    pull_request_review_id: 77,
    user: { login: 'coderabbitai[bot]' },
    body: 'Handle the error path\n\nThe error is swallowed before it reaches the caller.',
    path: findingPath,
    line: 42,
    commit_id: headSha,
    html_url: `https://github.com/${repository}/pull/401#discussion_r88`,
    ...overrides,
  };
}

function aikidoEvent(overrides = {}) {
  return {
    eventName: 'check_run',
    action: 'completed',
    repository: { full_name: repository },
    check_run: {
      id: 99,
      name: 'Aikido Security: Deep Review',
      conclusion: 'failure',
      head_sha: headSha,
      details_url: `https://github.com/${repository}/actions/runs/99`,
      pull_requests: [{ number: 401 }],
      app: { slug: 'aikido' },
    },
    ...overrides,
  };
}

test('collects one concrete CodeRabbit finding and binds it to the current head', async () => {
  const result = await collectFinding({
    event: codeRabbitEvent(),
    repository,
    api: fakeApi({ getReviewComments: async () => [codeRabbitComment()] }),
  });

  assert.equal(result.status, 'READY', JSON.stringify(result));
  assert.equal(result.finding.source, 'coderabbit');
  assert.equal(result.finding.path, findingPath);
  assert.equal(result.finding.headSha, headSha);
  assert.equal(result.finding.baseBranch, 'main');
  assert.match(result.finding.fingerprint, /^[0-9a-f]{64}$/);
});

test('collects one concrete Aikido annotation and rejects ambiguous links', async () => {
  const api = fakeApi({
    getCheckAnnotations: async () => [{
      path: findingPath,
      start_line: 7,
      message: 'Validate the input before using it.',
      title: 'Unchecked input',
      blob_href: `https://github.com/${repository}/blob/${headSha}/${findingPath}#L7`,
    }],
  });
  const result = await collectFinding({ event: aikidoEvent(), repository, api });
  assert.equal(result.status, 'READY');
  assert.equal(result.finding.source, 'aikido');
  assert.equal(result.finding.line, 7);

  const ambiguous = await collectFinding({
    event: aikidoEvent({ check_run: { ...aikidoEvent().check_run, pull_requests: [{ number: 401 }, { number: 402 }] } }),
    repository,
    api,
  });
  assert.equal(ambiguous.status, 'HUMAN_REQUIRED');
});

test('fails closed for foreign/fork/stale/closed/generic and non-reviewer events', async () => {
  const baseApi = fakeApi({ getReviewComments: async () => [codeRabbitComment()] });
  const cases = [
    codeRabbitEvent({ repository: { full_name: 'other/repo' } }),
    codeRabbitEvent({ pull_request: pullRequest({ head: { ref: 'fork', sha: headSha, repo: { full_name: 'other/repo' } } }) }),
    codeRabbitEvent({ pull_request: pullRequest({ head: { ref: 'feature/fix-review', sha: 'b'.repeat(40), repo: { full_name: repository } } }) }),
    codeRabbitEvent({ pull_request: pullRequest({ state: 'closed' }) }),
    codeRabbitEvent({ review: { ...codeRabbitEvent().review, user: { login: 'human-user' } } }),
    codeRabbitEvent({ review: { ...codeRabbitEvent().review, state: 'approved' } }),
  ];
  for (const [index, event] of cases.entries()) {
    const result = await collectFinding({ event, repository, api: baseApi });
    assert.notEqual(result.status, 'READY', `case ${index}: ${JSON.stringify(result)}`);
  }

  const generic = await collectFinding({
    event: codeRabbitEvent(),
    repository,
    api: fakeApi({ getReviewComments: async () => [codeRabbitComment({ path: undefined })] }),
  });
  assert.equal(generic.status, 'HUMAN_REQUIRED');

  const staleAikido = await collectFinding({
    event: aikidoEvent({ check_run: { ...aikidoEvent().check_run, head_sha: 'b'.repeat(40) } }),
    repository,
    api: fakeApi({ getCheckAnnotations: async () => [{ path: findingPath, message: 'stale check' }] }),
  });
  assert.equal(staleAikido.status, 'HUMAN_REQUIRED');
});

test('does not create duplicate or remediation-branch work and enforces cycle cap', async () => {
  const duplicate = await collectFinding({
    event: codeRabbitEvent(),
    repository,
    api: fakeApi({
      getReviewComments: async () => [codeRabbitComment()],
      getIssueComments: async () => [{ body: '<!-- hank-review-remediation: fingerprint=0000000000000000000000000000000000000000000000000000000000000000 -->' }],
    }),
  });
  assert.equal(duplicate.status, 'READY', JSON.stringify(duplicate));

  const finding = duplicate.finding;
  const exactDuplicate = await collectFinding({
    event: codeRabbitEvent(),
    repository,
    api: fakeApi({
      getReviewComments: async () => [codeRabbitComment()],
      getIssueComments: async () => [{ user: { login: 'github-actions[bot]' }, body: `<!-- hank-review-remediation: fingerprint=${finding.fingerprint} -->` }],
    }),
  });
  assert.equal(exactDuplicate.status, 'NOOP');

  const remediationBranch = await collectFinding({
    event: codeRabbitEvent({ pull_request: pullRequest({ head: { ref: 'review-remediation/pr-401/aaaaaaaaaaaa-bbbbbbbbbbbb', sha: headSha, repo: { full_name: repository } } }) }),
    repository,
    api: fakeApi({ getPullRequest: async () => pullRequest({ head: { ref: 'review-remediation/pr-401/aaaaaaaaaaaa-bbbbbbbbbbbb', sha: headSha, repo: { full_name: repository } } }) }),
  });
  assert.equal(remediationBranch.status, 'NOOP');

  const capped = await collectFinding({
    event: codeRabbitEvent(),
    repository,
    api: fakeApi({ getReviewComments: async () => [codeRabbitComment()], getIssueComments: async () => [
      { user: { login: 'github-actions[bot]' }, body: '<!-- hank-review-remediation: lineage=401 cycle=1 -->' },
      { user: { login: 'github-actions[bot]' }, body: '<!-- hank-review-remediation: lineage=401 cycle=2 -->' },
    ] }),
  });
  assert.equal(capped.status, 'HUMAN_REQUIRED');
  assert.equal(MAX_REMEDIATION_CYCLES, 2);
});

test('keeps the remediation cycle cap across source head changes', async () => {
  const first = await collectFinding({
    event: codeRabbitEvent(),
    repository,
    api: fakeApi({ getReviewComments: async () => [codeRabbitComment()] }),
  });
  assert.equal(first.status, 'READY');
  const lineage = findingLineage(first.finding);
  const nextHead = 'b'.repeat(40);
  const nextEvent = codeRabbitEvent({
    pull_request: pullRequest({ head: { ref: 'feature/fix-review', sha: nextHead, repo: { full_name: repository } } }),
    review: { ...codeRabbitEvent().review, commit_id: nextHead },
  });
  const capped = await collectFinding({
    event: nextEvent,
    repository,
    api: fakeApi({
      getReviewComments: async () => [codeRabbitComment({ commit_id: nextHead })],
      getIssueComments: async () => [{ user: { login: 'github-actions[bot]' }, body: `<!-- hank-review-remediation: lineage=${lineage} cycle=2 -->` }],
    }),
  });

  assert.equal(capped.status, 'HUMAN_REQUIRED');
});

test('ignores cycle and duplicate markers authored by an untrusted commenter', async () => {
  const collected = await collectFinding({
    event: codeRabbitEvent(),
    repository,
    api: fakeApi({
      getReviewComments: async () => [codeRabbitComment()],
      getIssueComments: async () => [{
        user: { login: 'untrusted-user' },
        body: '<!-- hank-review-remediation: lineage=401 cycle=2 -->',
      }],
    }),
  });

  assert.equal(collected.status, 'READY');
  assert.equal(collected.cycle, 1);
});

test('rejects multiple CodeRabbit inline findings instead of selecting one silently', async () => {
  const result = await collectFinding({
    event: codeRabbitEvent(),
    repository,
    api: fakeApi({ getReviewComments: async () => [codeRabbitComment(), codeRabbitComment({ id: 89, path: 'src/other.rs' })] }),
  });
  assert.equal(result.status, 'HUMAN_REQUIRED');
});

test('rejects a model patch that targets a file outside the reviewer finding', async () => {
  const collected = await collectFinding({
    event: codeRabbitEvent(),
    repository,
    api: fakeApi({ getReviewComments: async () => [codeRabbitComment()] }),
  });
  const wrongPathPatch = remediationPatch.replaceAll('src/value.txt', 'src/other.txt');
  const result = await proposeFinding({
    collected,
    api: { getPullRequestFiles: async () => [{ filename: findingPath, patch: 'source diff' }] },
    apiKey: 'provider-secret-fixture',
    fetchImpl: async () => new Response(JSON.stringify({ choices: [{ message: { content: wrongPathPatch } }] }), { status: 200 }),
  });

  assert.equal(result.status, 'HUMAN_REQUIRED');
  assert.equal(result.reason, 'PROPOSAL_PATCH_PATH_MISMATCH');
});

test('builds a bounded proposal input and redacted evidence descriptor', () => {
  const finding = {
    source: 'coderabbit', repository, pullRequest: 401, sourceBranch: 'feature/fix-review', baseBranch: 'main', headSha,
    reviewer: 'coderabbitai[bot]', title: 'Fix error', detail: 'token=secret-value', path: findingPath, line: 42,
    evidenceUrl: `https://github.com/${repository}/pull/401#discussion_r88`, fingerprint: 'c'.repeat(64), policyRevision: 'review-remediation-v1',
  };
  const proposal = buildProposalInput({ finding, files: [{ filename: findingPath, patch: 'diff --git a/x b/x\n' }, { filename: 'src/other.rs', patch: 'ignored' }] });
  assert.equal(proposal.path, findingPath);
  assert.equal(proposal.patch, 'diff --git a/x b/x\n');
  const evidence = buildEvidenceDescriptor({
    finding,
    patch: 'patch secret=secret-value',
    tests: ['git diff --check'],
    tree: {
      digest: 'd'.repeat(64),
      files: [findingPath],
      gates: [
        { name: 'source-head', status: 'PASS' },
        { name: 'patch-applicability', status: 'PASS' },
        { name: 'patch-boundaries', status: 'PASS' },
        { name: 'whitespace', status: 'PASS' },
        { name: 'semantic-syntax', status: 'PASS' },
      ],
    },
  });
  assert.equal(evidence.status, 'VALIDATED');
  assert.doesNotMatch(JSON.stringify(evidence), /secret-value/);
  assert.equal(evidence.sourceSha, headSha);
});

test('rejects validation evidence without the trusted semantic gate', () => {
  const finding = {
    source: 'coderabbit', repository, pullRequest: 401, sourceBranch: 'feature/fix-review', baseBranch: 'main', headSha,
    reviewer: 'coderabbitai[bot]', title: 'Fix error', detail: 'The value is stale.', path: findingPath, line: 42,
  };
  assert.throws(
    () => buildEvidenceDescriptor({
      finding,
      patch: 'patch',
      tree: {
        files: [findingPath],
        treeDigest: 'd'.repeat(64),
        gates: [
          { name: 'source-head', status: 'PASS' },
          { name: 'patch-applicability', status: 'PASS' },
          { name: 'patch-boundaries', status: 'PASS' },
          { name: 'whitespace', status: 'PASS' },
        ],
      },
    }),
    (error) => error.code === 'EVIDENCE_GATES_INVALID',
  );
});

test('GitHub adapter uses bounded read-only requests and redacts errors', async () => {
  let request;
  const api = createGithubApi({
    token: 'github-secret-fixture',
    repository,
    fetchImpl: async (url, init) => {
      request = { url, init };
      return new Response(JSON.stringify({ number: 401 }), { status: 200, headers: { 'content-type': 'application/json' } });
    },
  });
  const pull = await api.getPullRequest(401);
  assert.equal(pull.number, 401);
  assert.match(request.url, /repos\/stoltembergg-png\/hank\/pulls\/401$/);
  assert.equal(request.init.headers.Authorization, 'Bearer github-secret-fixture');

  const failingApi = createGithubApi({ token: 'github-secret-fixture', repository, fetchImpl: async () => new Response('token=github-secret-fixture', { status: 500 }) });
  await assert.rejects(failingApi.getPullRequest(401), (error) => error instanceof GithubApiError && !error.message.includes('github-secret-fixture'));
});

test('GitHub adapter rejects repository path segments and unbounded first pages', async () => {
  for (const invalidRepository of ['../repo', 'owner/..', './repo', 'owner/.']) {
    assert.throws(
      () => createGithubApi({ token: 'fixture', repository: invalidRepository, fetchImpl: async () => new Response('[]', { status: 200 }) }),
      (error) => error.code === 'GITHUB_REPOSITORY_INVALID',
    );
  }

  const response = JSON.stringify(Array.from({ length: 100 }, (_, index) => ({ id: index + 1 })));
  const api = createGithubApi({
    token: 'fixture',
    repository,
    fetchImpl: async () => new Response(response, { status: 200 }),
  });
  for (const method of [
    () => api.getReviewComments(401),
    () => api.getIssueComments(401),
    () => api.getPullRequestFiles(401),
    () => api.getCheckAnnotations(99),
  ]) {
    await assert.rejects(method(), (error) => error.code === 'GITHUB_PAGINATION_UNBOUNDED');
  }
});

function fixtureWorkspace(prefix) {
  const workspace = mkdtempSync(join(process.cwd(), `.hank-review-orchestrator-${prefix}`));
  const gitEnvironment = {
    ...process.env,
    GIT_AUTHOR_DATE: '2000-01-01T00:00:00Z',
    GIT_COMMITTER_DATE: '2000-01-01T00:00:00Z',
  };
  mkdirSync(join(workspace, 'src'));
  writeFileSync(join(workspace, 'src', 'value.txt'), 'before\n');
  execFileSync('git', ['init', '-q'], { cwd: workspace, windowsHide: true });
  execFileSync('git', ['config', 'core.autocrlf', 'false'], { cwd: workspace, windowsHide: true });
  execFileSync('git', ['config', 'user.email', 'test@example.invalid'], { cwd: workspace, windowsHide: true });
  execFileSync('git', ['config', 'user.name', 'Test Fixture'], { cwd: workspace, windowsHide: true });
  execFileSync('git', ['add', 'src/value.txt'], { cwd: workspace, windowsHide: true });
  execFileSync('git', ['commit', '-qm', 'fixture'], { cwd: workspace, env: gitEnvironment, windowsHide: true });
  return workspace;
}

function workspaceHead(workspace) {
  return execFileSync('git', ['rev-parse', 'HEAD'], { cwd: workspace, encoding: 'utf8', windowsHide: true }).trim();
}

function workspaceInput(workspace) {
  return relative(process.cwd(), workspace);
}

function patchInput(patchFile) {
  return relative(process.cwd(), patchFile);
}

test('rejects validation when the workspace is not at the finding SHA', async () => {
  const workspace = fixtureWorkspace('hank-review-head-mismatch-');
  const patchFile = join(workspace, 'remediation.patch');
  writeFileSync(patchFile, remediationPatch);
  try {
    const result = await validateProposal({
      proposed: {
        status: 'PROPOSED',
        patchDigest: 'd'.repeat(64),
        finding: { headSha: headSha },
      },
      patchFile: patchInput(patchFile),
      workspace: workspaceInput(workspace),
    });

    assert.equal(result.status, 'HUMAN_REQUIRED');
    assert.equal(result.reason, 'PATCH_WORKSPACE_HEAD_MISMATCH');
  } finally {
    rmSync(workspace, { recursive: true, force: true });
  }
});

test('runs collect-to-propose-to-validate-to-publish with no provider network dependency', async () => {
  const workspace = fixtureWorkspace('hank-review-orchestrator-');
  try {
    const sourceSha = workspaceHead(workspace);
    const collectedFinding = {
      source: 'coderabbit', repository, pullRequest: 401, sourceBranch: 'feature/fix-review', baseBranch: 'main', headSha: sourceSha,
      reviewer: 'coderabbitai[bot]', title: 'Fix value', detail: 'The value is stale.', path: 'src/value.txt', line: 1,
      evidenceUrl: `https://github.com/${repository}/pull/401#discussion_r1`, policyRevision: 'review-remediation-v1',
    };
    collectedFinding.fingerprint = findingFingerprint(collectedFinding);
    const collected = {
      status: 'READY',
      cycle: 1,
      finding: collectedFinding,
    };
    const proposal = await proposeFinding({
      collected,
      api: { getPullRequestFiles: async () => [{ filename: 'src/value.txt', patch: remediationPatch }] },
      apiKey: 'provider-secret-fixture',
      fetchImpl: async () => new Response(JSON.stringify({ choices: [{ message: { content: remediationPatch } }] }), { status: 200 }),
    });
    assert.equal(proposal.status, 'PROPOSED');
    assert.equal(proposal.viability, 'VIABLE_PATCH');

    const patchFile = join(workspace, 'remediation.patch');
    writeFileSync(patchFile, proposal.patch);
    const validated = await validateProposal({ proposed: proposal, patchFile: patchInput(patchFile), workspace: workspaceInput(workspace), tests: ['git diff --check'] });
    assert.equal(validated.status, 'VALIDATED');
    assert.equal(validated.finding.pullRequest, 401);
    assert.equal(validated.sourceSha, sourceSha);
    assert.deepEqual(validated.gates.map((gate) => gate.status), ['PASS', 'PASS', 'PASS', 'PASS', 'PASS']);
    assert.equal(readFileSync(join(workspace, 'src', 'value.txt'), 'utf8'), 'after\n');

    const publishWorkspace = fixtureWorkspace('hank-review-publish-');
    const publishPatchFile = join(publishWorkspace, 'remediation.patch');
    writeFileSync(publishPatchFile, proposal.patch);
    try {
      const published = await publishValidated({
        validated,
        proposal,
        finding: collected.finding,
        patchFile: patchInput(publishPatchFile),
        workspace: workspaceInput(publishWorkspace),
        cycle: 1,
        api: fakeApi({
          getPullRequest: async () => pullRequest({ head: { ref: 'feature/fix-review', sha: sourceSha, repo: { full_name: repository } } }),
          getBranch: async () => ({ name: 'feature/fix-review', commit: { sha: sourceSha } }),
        }),
      });
      assert.equal(published.status, 'PUBLISH_READY');
      assert.match(published.branch, /^review-remediation\/pr-401\/[0-9a-f]{12}-[0-9a-f]{12}$/);
      assert.equal(published.base, 'feature/fix-review');
      assert.equal(published.baseBranch, 'main');
      assert.match(published.marker, /^<!-- hank-review-remediation: fingerprint=[0-9a-f]{64} -->$/);
      assert.match(published.lineageMarker, /^<!-- hank-review-remediation: lineage=[0-9a-f]{64} cycle=1 -->$/);
      assert.equal(published.lineage, findingLineage(collected.finding));
      assert.equal(published.noApproval, true);
      assert.equal(published.noMerge, true);

      const tampered = await publishValidated({
        validated,
        proposal: { ...proposal, finding: { ...proposal.finding, baseBranch: 'release' } },
        finding: collected.finding,
        patchFile: patchInput(publishPatchFile),
        workspace: workspaceInput(publishWorkspace),
        cycle: 1,
        api: fakeApi({ getBranch: async () => ({ name: 'feature/fix-review', commit: { sha: headSha } }) }),
      });
      assert.equal(tampered.status, 'HUMAN_REQUIRED');

      const movedWorkspace = fixtureWorkspace('hank-review-moved-');
      const movedPatchFile = join(movedWorkspace, 'remediation.patch');
      writeFileSync(movedPatchFile, proposal.patch);
      try {
        const moved = await publishValidated({
          validated,
          proposal,
          finding: collected.finding,
          patchFile: patchInput(movedPatchFile),
          workspace: workspaceInput(movedWorkspace),
          cycle: 1,
          api: fakeApi({
            getPullRequest: async () => pullRequest({ head: { ref: 'feature/fix-review', sha: 'b'.repeat(40), repo: { full_name: repository } } }),
          }),
        });
        assert.equal(moved.status, 'HUMAN_REQUIRED');
      } finally {
        rmSync(movedWorkspace, { recursive: true, force: true });
      }
    } finally {
      rmSync(publishWorkspace, { recursive: true, force: true });
    }
  } finally {
    rmSync(workspace, { recursive: true, force: true });
  }
});
