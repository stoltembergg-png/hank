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
