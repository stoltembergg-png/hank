import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { test } from 'node:test';

const root = process.cwd();

function runCargoTest(filter) {
  const result = spawnSync(
    'cargo',
    [
      'test',
      '-p',
      'recovery-core',
      '--test',
      'release_rollback_contract',
      '--locked',
      '--offline',
      '--',
      filter,
    ],
    { cwd: root, encoding: 'utf8', stdio: 'pipe' },
  );
  const output = [result.stdout, result.stderr].filter(Boolean).join('\n');
  if (result.status !== 0) {
    process.stderr.write(output);
  }
  assert.equal(result.status, 0, `recovery-core/${filter} failed\n${output.slice(-4000)}`);
  assert.match(output, /running\s+[1-9]\d*\s+tests?/i, `recovery-core/${filter} matched no tests`);
}

const contracts = [
  ['AC-3806', 'known-good slot is recorded for a verified proof', 'known_good_slot'],
  ['AC-3807', 'failed boot or health selects the previous known-good', 'failed_boot_or_health'],
  ['AC-3808', 'a revoked version cannot be restored', 'revoked_version'],
  ['AC-3809', 'repeated rollback converges without a loop', 'repeated_rollback'],
  ['AC-3810', 'an invalid proof is rejected at construction', 'invalid_proof'],
  ['AC-3811', 'known-good advances strictly to a newer version', 'known_good_advances'],
  ['AC-3812', 'rollback decisions are audited without sensitive material', 'incident_audit'],
  ['AC-3813', 'proof material is redacted from observability', 'proof_material'],
];

for (const [ac, description, filter] of contracts) {
  test(`${description} @spec:${ac}`, () => {
    runCargoTest(filter);
  });
}