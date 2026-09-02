import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import test from 'node:test';

const workflowRoot = new URL('../.github/workflows/', import.meta.url);
const workflowFiles = readdirSync(workflowRoot)
  .filter((name) => name.endsWith('.yml') || name.endsWith('.yaml'))
  .sort();
const fullSha = /^[0-9a-f]{40}$/;

function workflowText(name) {
  return readFileSync(new URL(name, workflowRoot), 'utf8');
}

function workflowStepBlock(workflow, stepName) {
  const lines = workflow.split(/\r?\n/);
  const matches = lines.flatMap((line, index) => {
    const match = /^(\s*)-\s+name:\s*(.*?)\s*$/.exec(line);
    if (!match || match[2].replace(/^(['"])(.*)\1$/, '$2') !== stepName) return [];
    return [{ index, indentation: match[1].length }];
  });
  if (matches.length !== 1) return undefined;

  const { index: start, indentation } = matches[0];
  let end = lines.length;
  for (let index = start + 1; index < lines.length; index += 1) {
    const line = lines[index];
    if (line.trim() === '' || line.trimStart().startsWith('#')) continue;
    const lineIndentation = line.length - line.trimStart().length;
    if (lineIndentation === indentation && /^\s*-(?:\s|$)/.test(line)) {
      end = index;
      break;
    }
  }

  return {
    lines: lines.slice(start, end),
    indentation,
  };
}

function workflowStepRun(workflow, stepName) {
  const step = workflowStepBlock(workflow, stepName);
  if (!step) return undefined;

  const runIndentation = step.indentation + 2;
  const runIndex = step.lines.findIndex((line) => {
    const indentation = line.length - line.trimStart().length;
    return indentation === runIndentation && /^\s*run:\s*/.test(line);
  });
  if (runIndex === -1) return undefined;

  const runLine = step.lines[runIndex];
  const runValue = runLine.slice(runLine.indexOf(':') + 1).trim();
  if (!/^[|>]/.test(runValue)) return runValue;
  if (runValue.startsWith('>')) return undefined;

  const body = [];
  for (let index = runIndex + 1; index < step.lines.length; index += 1) {
    const line = step.lines[index];
    if (line.trim() !== '' && line.length - line.trimStart().length <= runIndentation) break;
    body.push(line);
  }

  const nonEmptyIndentations = body
    .filter((line) => line.trim() !== '')
    .map((line) => line.length - line.trimStart().length);
  const bodyIndentation = Math.min(...nonEmptyIndentations);
  return body
    .map((line) => (line.trim() === '' ? '' : line.slice(bodyIndentation)))
    .join('\n');
}

test('all workflows declare fail-closed execution controls', () => {
  assert.ok(workflowFiles.length > 0);
  for (const name of workflowFiles) {
    const text = workflowText(name);
    assert.match(text, /^permissions:\s*$/m, `${name}: missing top-level permissions`);
    assert.match(text, /^concurrency:\s*$/m, `${name}: missing top-level concurrency`);
    assert.match(text, /^\s+timeout-minutes:\s*[1-9][0-9]*\s*$/m, `${name}: missing timeout-minutes`);
    assert.doesNotMatch(text, /continue-on-error\s*:\s*true/, `${name}: continue-on-error is not fail-closed`);
    assert.doesNotMatch(text, /pull_request_target\s*:/, `${name}: pull_request_target is forbidden`);
  }
});

test('ONP evidence checks out the pull request head, not a synthetic merge commit', () => {
  const text = workflowText('onp-sdd-evidence.yml');
  assert.match(
    text,
    /ref:\s*\$\{\{\s*github\.event\.pull_request\.head\.sha\s*\|\|\s*github\.sha\s*\}\}/,
    'ONP evidence must bind to the PR head SHA',
  );
});

test('quality integrity binds and verifies the exact pull request head', () => {
  const text = workflowText('quality-integrity.yml');
  const checkoutStep = workflowStepBlock(text, 'Checkout exact revision');
  assert.ok(checkoutStep, 'quality integrity must have one exact checkout step');
  assert.match(
    checkoutStep.lines.join('\n'),
    /ref:\s*\$\{\{\s*github\.event\.pull_request\.head\.sha\s*\|\|\s*github\.sha\s*\}\}/,
    'quality integrity must bind to the PR head SHA',
  );
  const verificationStep = workflowStepBlock(text, 'Verify checked revision');
  assert.ok(verificationStep, 'quality integrity must have one revision verification step');
  assert.match(verificationStep.lines.join('\n'), /EXPECTED_SHA:/);
  const verificationRun = workflowStepRun(text, 'Verify checked revision');
  assert.ok(verificationRun, 'revision identity checks must be in the named step run block');
  assert.match(verificationRun, /^set -euo pipefail$/m);
  assert.match(verificationRun, /^actual_sha="\$\(git rev-parse HEAD\)"$/m);
  assert.match(verificationRun, /^test "\$actual_sha" = "\$EXPECTED_SHA"$/m);
  assert.match(
    verificationRun,
    /expected_tree=.*git ls-tree "\$EXPECTED_SHA"[\s\S]*actual_tree=.*git ls-tree HEAD[\s\S]*test "\$actual_tree" = "\$expected_tree"/,
    'quality integrity must compare the checked tree with the expected revision',
  );
});

test('quality integrity ignores revision fragments outside the named step', () => {
  const decoyWorkflow = `jobs:
  integrity:
    steps:
      - name: Verify checked revision
        run: echo "verification omitted"
      - name: Another step
        run: |
          set -euo pipefail
          actual_sha="$(git rev-parse HEAD)"
          test "$actual_sha" = "$EXPECTED_SHA"
`;

  const verificationRun = workflowStepRun(decoyWorkflow, 'Verify checked revision');
  assert.equal(verificationRun, 'echo "verification omitted"');
  assert.doesNotMatch(verificationRun, /git rev-parse HEAD/);
});

test('quality integrity rejects folded revision verification blocks', () => {
  const foldedWorkflow = `jobs:
  integrity:
    steps:
      - name: Verify checked revision
        run: >
          set -euo pipefail
          actual_sha="$(git rev-parse HEAD)"
          test "$actual_sha" = "$EXPECTED_SHA"
`;

  assert.equal(workflowStepRun(foldedWorkflow, 'Verify checked revision'), undefined);
});

test('review remediation keeps MiMo and publication boundaries explicit', () => {
  const text = workflowText('review-remediation-agent.yml');
  assert.match(text, /XIAOMI_MIMO_API_KEY:\s*\$\{\{\s*secrets\.XIAOMI_MIMO_API_KEY\s*\}\}/);
  assert.match(text, /gh pr create --draft/);
  assert.doesNotMatch(text, /gh pr (?:merge|review|approve)/i);
  assert.doesNotMatch(text, /pull_request_target/);
  assert.match(text, /ref:\s*\$\{\{\s*needs\.collect\.outputs\.source_sha\s*\}\}/);
  assert.match(text, /git -C target rev-parse HEAD/);
  assert.match(text, /group:\s*review-remediation-\$\{\{\s*github\.repository\s*\}\}-\$\{\{\s*github\.event\.pull_request\.number\s*\|\|\s*github\.event\.check_run\.pull_requests\[0\]\.number\s*\|\|\s*github\.event\.check_run\.id\s*\}\}/);
  assert.doesNotMatch(text, /awk '\{print \\$1\}'/);
  const exactTreeBlocks = [...text.matchAll(/- name: Verify exact source tree[\s\S]*?(?=\n      - name:|\n  [a-zA-Z0-9_-]+:|$)/g)].map((match) => match[0]);
  assert.equal(exactTreeBlocks.length, 2);
  for (const block of exactTreeBlocks) {
    assert.match(block, /EXPECTED_SHA:/);
    assert.match(block, /test "\$actual_sha" = "\$EXPECTED_SHA"/);
    assert.match(block, /test "\$actual_tree" = "\$expected_tree"/);
  }
});

test('all external actions are pinned and checkout does not persist credentials', () => {
  for (const name of workflowFiles) {
    const text = workflowText(name);
    const lines = text.split(/\r?\n/);
    lines.forEach((line, index) => {
      const match = line.match(/^\s*(?:-\s*)?uses:\s*([^\s#]+)/);
      if (!match || match[1].startsWith('./')) return;
      const [action, revision] = match[1].split('@');
      assert.ok(action && fullSha.test(revision ?? ''), `${name}:${index + 1}: action is not SHA-pinned`);
      if (action === 'actions/checkout') {
        const block = lines.slice(index, index + 8).join('\n');
        assert.match(block, /persist-credentials:\s*false/, `${name}:${index + 1}: checkout credentials persist`);
      }
    });
  }
});
