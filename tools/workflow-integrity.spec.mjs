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
  assert.match(
    text,
    /ref:\s*\$\{\{\s*github\.event\.pull_request\.head\.sha\s*\|\|\s*github\.sha\s*\}\}/,
    'quality integrity must bind to the PR head SHA',
  );
  assert.match(text, /- name: Verify checked revision[\s\S]*EXPECTED_SHA:/);
  assert.match(text, /actual_sha=.*git rev-parse HEAD[\s\S]*test "\$actual_sha" = "\$EXPECTED_SHA"/);
  assert.match(
    text,
    /expected_tree=.*git ls-tree "\$EXPECTED_SHA"[\s\S]*actual_tree=.*git ls-tree HEAD[\s\S]*test "\$actual_tree" = "\$expected_tree"/,
    'quality integrity must compare the checked tree with the expected revision',
  );
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
