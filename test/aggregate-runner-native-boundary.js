import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const runner = readFileSync(new URL('../tools/run-all-tests.mjs', import.meta.url), 'utf8');
const workflow = readFileSync(
  new URL('../.github/workflows/onp-sdd-evidence.yml', import.meta.url),
  'utf8',
);

test('aggregate runner exposes an explicit native-test boundary', () => {
  assert.match(runner, /HANK_SKIP_TAURI/);
  assert.match(runner, /Tauri acceptance tests/);
  assert.match(runner, /process\.env\.HANK_SKIP_TAURI/);
});

test('ONP workflow runs native Tauri coverage explicitly', () => {
  assert.match(workflow, /HANK_SKIP_TAURI:\s*['"]?1/);
  assert.match(workflow, /Verify Tauri desktop[\s\S]*?HANK_SKIP_TAURI:\s*['"]?['"]?[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify tauri-desktop/);
});

test('ONP workflow runs repository workspace verification explicitly', () => {
  assert.match(
    workflow,
    /Verify repository workspace[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify repository-workspace/,
  );
});

test('ONP workflow runs Git worktree verification explicitly', () => {
  assert.match(
    workflow,
    /Verify Git worktree manager[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify git-worktree-manager/,
  );
});

test('ONP workflow runs branch policy verification explicitly', () => {
  assert.match(
    workflow,
    /Verify branch policy[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify branch-policy/,
  );
});

test('ONP workflow runs task-to-branch mapping verification explicitly', () => {
  assert.match(
    workflow,
    /Verify task-to-branch mapping[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify task-branch-mapping/,
  );
});

test('ONP workflow runs reviewer agent profile verification explicitly', () => {
  assert.match(
    workflow,
    /Verify reviewer agent profile[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify reviewer-agent-profile/,
  );
});
test('ONP workflow runs QA agent profile verification explicitly', () => {
  assert.match(
    workflow,
    /Verify QA agent profile[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify qa-agent-profile/,
  );
});

test('ONP workflow runs security agent profile verification explicitly', () => {
  assert.match(
    workflow,
    /Verify security agent profile[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify security-agent-profile/,
  );
});

test('ONP workflow runs coding agent profile verification explicitly', () => {
  assert.match(
    workflow,
    /Verify coding agent profile[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify coding-agent-profile/,
  );
});
