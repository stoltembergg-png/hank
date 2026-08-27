import assert from 'node:assert/strict';
import { before, test } from 'node:test';
import { spawnSync } from 'node:child_process';

const manifest = 'apps/desktop/src-tauri/Cargo.toml';
const cases = [
  ['AC-101', 'window lifecycle contract', 'ac_101_janela_abre_fecha_deterministico'],
  ['AC-102', 'minimal capability manifest contract', 'ac_102_manifest_sem_capacidades_perigosas'],
  ['AC-103', 'remote origin CSP contract', 'ac_103_csp_bloqueia_origem_remota'],
  ['AC-104', 'typed bridge command contract', 'ac_104_bridge_registra_somente_commands_tipados'],
  ['AC-105', 'structured startup log contract', 'ac_105_logs_estruturados'],
];

let cargoResult;
let cargoOutput = '';
before(() => {
  cargoResult = spawnSync(
    'cargo',
    ['test', '--manifest-path', manifest, '--test', 'tauri_ac_tests', '--', '--nocapture'],
    { encoding: 'utf8', stdio: 'pipe' },
  );
  cargoOutput = `${cargoResult.stdout ?? ''}\n${cargoResult.stderr ?? ''}`;
});

for (const [ac, description, rustTest] of cases) {
  test(`${description} @spec:${ac}`, () => {
    if (cargoResult.error || cargoResult.status !== 0) {
      console.error('Tauri desktop cargo test output:');
      console.error(cargoOutput);
    }
    assert.equal(
      cargoResult.error,
      undefined,
      `${ac}: cargo test could not start: ${cargoResult.error?.message ?? 'unknown error'}`,
    );
    assert.equal(
      cargoResult.status,
      0,
      `${ac}: cargo test failed with ${cargoResult.status ?? 'signal'}\n${cargoOutput.slice(-12000)}`,
    );
    assert.match(cargoOutput, new RegExp(`test ${rustTest} \.\.\. ok`), `${ac}: Rust test result missing`);
  });
}
