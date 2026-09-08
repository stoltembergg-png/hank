import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = new URL('../', import.meta.url);
const read = (file) => readFileSync(fileURLToPath(new URL(file, root)), 'utf8');

test('desktop updater E2E exercises signed activation and rollback', () => {
  const lifecycle = read('desktop-e2e/specs/project-lifecycle.e2e.mjs');
  assert.match(lifecycle, /stage_update/);
  assert.match(lifecycle, /activate_update/);
  assert.match(lifecycle, /rollback_update/);
  assert.match(lifecycle, /updater-rollback-report\.json/);
});

test('desktop runners provision an ephemeral updater key only when opted in', () => {
  for (const file of ['desktop-e2e/run-linux.sh', 'desktop-e2e/run-macos.sh', 'desktop-e2e/run-windows.ps1']) {
    const runner = read(file);
    assert.match(runner, /HANK_UPDATER_E2E/);
    assert.match(runner, /HANK_UPDATER_PUBLIC_KEY_DER_B64/);
    assert.match(runner, /updater-fixture\.mjs/);
  }
});

test('release install smoke runners promote only a real updater report', () => {
  const linux = read('desktop-e2e/install-smoke-linux.sh');
  const windows = read('desktop-e2e/install-smoke-windows.ps1');
  const lifecycle = read('desktop-e2e/specs/project-lifecycle.e2e.mjs');
  assert.match(lifecycle, /native-synthetic-signed-fixture/);
  assert.match(linux, /HANK_UPDATER_E2E/);
  assert.match(linux, /upgradeRollback/);
  assert.match(linux, /PASS_LIMITED/);
  assert.match(windows, /HANK_UPDATER_E2E/);
  assert.match(windows, /updater-rollback-report\.json/);
  assert.match(windows, /upgradeRollback/);
  assert.match(windows, /PASS_LIMITED/);
});
