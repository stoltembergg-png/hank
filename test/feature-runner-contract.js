import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';

test('ONP config selects a feature-scoped command', () => {
  const config = JSON.parse(readFileSync('onpspec.config.json', 'utf8'));
  assert.equal(config.testCommands['*'], 'node tools/run-feature-tests.mjs {feature}');
  assert.equal(existsSync('tools/run-feature-tests.mjs'), true);
});

test('feature runner has explicit native and frontend boundaries', () => {
  const runner = readFileSync('tools/run-feature-tests.mjs', 'utf8');
  assert.match(runner, /const args = file\.startsWith\('apps\/'\)/);
  assert.match(runner, /npm/);
  assert.match(runner, /args: \['--test'/);
  assert.match(runner, /skipTauri = process\.env\.HANK_SKIP_TAURI === '1'/);
  assert.match(runner, /file\.startsWith\('apps\/desktop\/'\)/);
});
