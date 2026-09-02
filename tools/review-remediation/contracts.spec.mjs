import assert from 'node:assert/strict';
import test from 'node:test';

import {
  MAX_FINDING_DETAIL_BYTES,
  MAX_FINDING_TITLE_BYTES,
  POLICY_REVISION,
  findingFingerprint,
  isDuplicateMarker,
  normalizeFinding,
  redactSecrets,
  remediationBranchName,
} from './contracts.mjs';

const repository = 'stoltembergg-png/hank';
const headSha = 'a'.repeat(40);

function validFinding(overrides = {}) {
  return {
    source: 'coderabbit',
    repository,
    pullRequest: 401,
    sourceBranch: 'feature/fix-review',
    headSha,
    reviewer: 'coderabbitai[bot]',
    title: 'Handle the error path',
    detail: 'The error is swallowed before it reaches the caller.',
    path: 'crates/agent-core/src/lib.rs',
    line: 42,
    evidenceUrl: `https://github.com/${repository}/pull/401#discussion_r1`,
    ...overrides,
  };
}

test('normalizes a CodeRabbit finding and binds it to the repository head', () => {
  const result = normalizeFinding(validFinding(), repository);

  assert.equal(result.status, 'READY');
  assert.equal(result.finding.policyRevision, POLICY_REVISION);
  assert.match(result.finding.fingerprint, /^[0-9a-f]{64}$/);
  assert.equal(result.finding.path, 'crates/agent-core/src/lib.rs');
});

test('normalizes an Aikido finding from the accepted check identity', () => {
  const result = normalizeFinding(validFinding({
    source: 'aikido',
    reviewer: 'Aikido Security',
    title: 'Unchecked input',
    detail: 'Validate the input before using it.',
    evidenceUrl: `https://github.com/${repository}/pull/401/checks`,
  }), repository);

  assert.equal(result.status, 'READY');
  assert.equal(result.finding.source, 'aikido');
});

test('rejects foreign, stale, generic, and unsupported findings', () => {
  const cases = [
    validFinding({ repository: 'other/repo' }),
    validFinding({ headSha: 'not-a-sha' }),
    validFinding({ path: undefined }),
    validFinding({ detail: '' }),
    validFinding({ source: 'human', reviewer: 'maintainer' }),
    validFinding({ reviewer: 'unknown-bot' }),
    validFinding({ evidenceUrl: 'http://example.test/finding' }),
  ];

  for (const input of cases) {
    assert.equal(normalizeFinding(input, repository).status, 'HUMAN_REQUIRED');
  }
});

test('rejects unsafe branches and paths before model processing', () => {
  for (const sourceBranch of ['../main', 'feature\\secret', 'feature..name', 'feature name']) {
    assert.equal(normalizeFinding(validFinding({ sourceBranch }), repository).status, 'HUMAN_REQUIRED');
  }
  for (const path of ['../secret.txt', '/absolute.txt', '.github/workflows/build.yml', 'config/credentials.json', 'config/.env.local', 'target\\file.rs']) {
    assert.equal(normalizeFinding(validFinding({ path }), repository).status, 'HUMAN_REQUIRED');
  }
});

test('redacts credentials and secret-like reviewer text', () => {
  const value = redactSecrets([
    'Authorization: Bearer sk-example-value',
    'token=abc123 password=hunter2 api_key=long-value',
    'github_pat_1234567890abcdef',
  ].join('\n'));

  assert.doesNotMatch(value, /sk-example-value|abc123|hunter2|long-value|github_pat_/);
  assert.match(value, /\[REDACTED\]/g);
});

test('bounds and redacts title and detail', () => {
  const longTitle = 'x'.repeat(MAX_FINDING_TITLE_BYTES + 1);
  const longDetail = 'x'.repeat(MAX_FINDING_DETAIL_BYTES + 1);

  assert.equal(normalizeFinding(validFinding({ title: longTitle }), repository).status, 'HUMAN_REQUIRED');
  assert.equal(normalizeFinding(validFinding({ detail: longDetail }), repository).status, 'HUMAN_REQUIRED');
  const redacted = normalizeFinding(validFinding({ detail: 'token=secret-value' }), repository);
  assert.equal(redacted.status, 'READY');
  assert.doesNotMatch(redacted.finding.detail, /secret-value/);
});

test('fingerprints are deterministic and independent of object key order', () => {
  const first = validFinding();
  const firstResult = normalizeFinding(first, repository);
  assert.equal(firstResult.status, 'READY');

  const second = {
    detail: first.detail,
    title: first.title,
    line: first.line,
    path: first.path,
    reviewer: first.reviewer,
    headSha: first.headSha,
    sourceBranch: first.sourceBranch,
    pullRequest: first.pullRequest,
    repository: first.repository,
    source: first.source,
    evidenceUrl: first.evidenceUrl,
  };
  assert.equal(findingFingerprint(first), findingFingerprint(second));
  assert.equal(firstResult.finding.fingerprint, findingFingerprint(first));
});

test('branch names use only identity fields and never reviewer text', () => {
  const result = normalizeFinding(validFinding({ reviewer: 'coderabbitai[bot]' }), repository);
  assert.equal(result.status, 'READY');
  const branch = remediationBranchName(result.finding);

  assert.match(branch, /^review-remediation\/pr-401\/aaaaaaaaaaaa-[0-9a-f]{12}$/);
  assert.doesNotMatch(branch, /Handle|coderabbit|error/);
});

test('duplicate markers require the exact fingerprint and marker shape', () => {
  const fingerprint = 'b'.repeat(64);
  const marker = `<!-- hank-review-remediation: fingerprint=${fingerprint} -->`;

  assert.equal(isDuplicateMarker(marker, fingerprint), true);
  assert.equal(isDuplicateMarker(`${marker}\nother text`, fingerprint), true);
  assert.equal(isDuplicateMarker(marker, 'c'.repeat(64)), false);
  assert.equal(isDuplicateMarker(`<!-- hank-review-remediation: fingerprint=${fingerprint.slice(0, 63)} -->`, fingerprint), false);
  assert.equal(isDuplicateMarker(`hank-review-remediation: fingerprint=${fingerprint}`, fingerprint), false);
});
