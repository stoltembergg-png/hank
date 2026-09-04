import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { test } from 'node:test';

const root = process.cwd();

function runCargoTest(packageName, target, filter) {
  const result = spawnSync(
    'cargo',
    ['test', '-p', packageName, '--test', target, '--locked', '--', filter],
    {
      cwd: root,
      encoding: 'utf8',
      stdio: 'inherit',
    },
  );
  assert.equal(result.status, 0, `${packageName}/${target}/${filter} failed`);
}

// @spec:AC-1466
// @spec:AC-1467
// @spec:AC-1468
// @spec:AC-1469
// @spec:AC-1470
const contracts = [
  [
    'AC-1466',
    'broker emits an opaque handle without secret material',
    'remote-core',
    'credential_broker_contract',
    'broker_emits_opaque_handle_and_never_carries_secret_material',
  ],
  [
    'AC-1467',
    'diverging scope resolution fails closed',
    'remote-core',
    'credential_broker_contract',
    'resolve_fails_closed_for_diverging_scope',
  ],
  [
    'AC-1468',
    'expired or revoked lease fails closed',
    'remote-core',
    'credential_broker_contract',
    'expired_or_revoked_lease_fails_closed',
  ],
  [
    'AC-1469',
    'broker bounds and redacts audit records',
    'remote-core',
    'credential_broker_contract',
    'broker_is_bounded_and_audit_never_records_secret_values',
  ],
  [
    'AC-1470',
    'lease binding prevents cross-agent use',
    'remote-core',
    'credential_broker_contract',
    'lease_binding_prevents_cross_actor_or_cross_project_use',
  ],
  [
    'AC-1466',
    'adapter obtains independent OS CSPRNG seeds',
    'remote-adapter',
    'os_entropy_contract',
    'os_entropy_produces_independent_seeds',
  ],
];

for (const [ac, description, packageName, target, filter] of contracts) {
// @spec:AC-1466
  test(`${description} @spec:${ac}`, () => {
    runCargoTest(packageName, target, filter);
  });
}
