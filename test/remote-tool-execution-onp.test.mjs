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
      'remote-core',
      '--test',
      'remote_tool_dispatch_contract',
      '--locked',
      '--offline',
      '--',
      filter,
    ],
    { cwd: root, encoding: 'utf8', stdio: 'inherit' },
  );
  assert.equal(result.status, 0, `remote-core/${filter} failed`);
}

const contracts = [
  ['AC-3007', 'tool allowed on the exact node is idempotent and conflicts are rejected', 'permitted_tool'],
  ['AC-3008', 'node, project, capability, and permission gates fail closed', 'wrong_node_or_permission_is_rejected_before_transport'],
  ['AC-3009', 'request payload does not carry sensitive material', 'payload'],
  ['AC-3010', 'post-dispatch failure becomes unknown and is not retried', 'timeout_or_transport_loss_becomes_unknown_and_is_not_retried'],
  ['AC-3011', 'pre-dispatch cancellation has no remote effect', 'cancellation_before_dispatch_has_no_remote_effect'],
  ['AC-3012', 'divergent or sensitive responses are not accepted as success', 'invalid_or_sensitive_response_is_unknown'],
  ['AC-3013', 'revoked or expired leases cannot dispatch', 'revoked_or_expired_lease_cannot_dispatch'],
  ['AC-3014', 'in-flight cancellation makes late results unknown', 'in_flight_cancellation_is_terminal_and_late_result_becomes_unknown'],
];

for (const [ac, description, filter] of contracts) {
  test(`${description} @spec:${ac}`, () => {
    runCargoTest(filter);
  });
}
