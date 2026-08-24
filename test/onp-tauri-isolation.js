import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const runner = readFileSync(new URL('../tools/run-all-tests.mjs', import.meta.url), 'utf8');
const workflow = readFileSync(
  new URL('../.github/workflows/onp-sdd-evidence.yml', import.meta.url),
  'utf8',
);

test('runner has an explicit tauri-only mode', () => {
  assert.match(runner, /HANK_ONLY_TAURI/);
  assert.match(runner, /commands\.splice\(0, commands\.length\)/);
});

test('ONP invokes tauri verification in isolated mode', () => {
  assert.match(workflow, /HANK_ONLY_TAURI:\s*['"]?1/);
  assert.match(workflow, /node tools\/ci\/run-onp-spec\.mjs verify tauri-desktop/);
});
