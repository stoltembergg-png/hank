import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const config = JSON.parse(readFileSync('onpspec.config.json', 'utf8'));
const verify = readFileSync('tools/onp-spec/scripts/lib/src/core/verify.js', 'utf8');
const runner = readFileSync('tools/tauri-desktop-onp.test.mjs', 'utf8');

test('tauri-desktop uses a granular TAP reporter without changing the default', () => {
  assert.equal(config.reporter, 'exitcode');
  assert.equal(config.reporters['tauri-desktop'], 'tap');
  assert.equal(config.testCommands['tauri-desktop'], 'node --test tools/tauri-desktop-onp.test.mjs');
  assert.match(verify, /config\.reporters\?\.\[featureName\] \|\| config\.reporter/);
});

test('the Tauri ONP runner maps every desktop AC to a real Rust test', () => {
  for (const [ac, , rustTest] of [
    ['AC-101', '', 'ac_101_janela_abre_fecha_deterministico'],
    ['AC-102', '', 'ac_102_manifest_sem_capacidades_perigosas'],
    ['AC-103', '', 'ac_103_csp_bloqueia_origem_remota'],
    ['AC-104', '', 'ac_104_bridge_registra_somente_commands_tipados'],
    ['AC-105', '', 'ac_105_logs_estruturados'],
  ]) {
    assert.match(runner, new RegExp(`['"]${ac}['"]`));
    assert.match(runner, new RegExp(`['"]${rustTest}['"]`));
  }
  assert.match(runner, /@spec:\$\{ac\}/);
  assert.match(runner, /cargo test/);
  assert.match(runner, /--test', 'tauri_ac_tests'/);
  assert.match(runner, /rustTest\} \.\.\. ok/);
});
