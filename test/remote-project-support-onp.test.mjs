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
      'remote_project_contract',
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
  assert.equal(result.status, 0, `remote-core/${filter} failed\n${output.slice(-4000)}`);
  assert.match(output, /running\s+[1-9]\d*\s+tests?/i, `remote-core/${filter} matched no tests`);
}

const contracts = [
  ['AC-3015', 'project descriptor binds to the exact node and peer', 'descriptor_binds'],
  ['AC-3016', 'wrong project or node is rejected fail closed', 'wrong_project_or_node'],
  ['AC-3017', 'stale or older version conflicts and requires reconcile', 'stale_or_older_version'],
  ['AC-3018', 'oversized input and unknown capability are rejected', 'descriptor_rejects'],
  ['AC-3019', 'typed references carry no raw filesystem paths', 'typed_references'],
  ['AC-3020', 'capability scope is explicit and denied by default', 'capability_scope'],
  ['AC-3021', 'binding survives restart via ledger reconstruction', 'binding_survives'],
  ['AC-3022', 'duplicate bind is idempotent for the same version', 'duplicate_bind'],
];

for (const [ac, description, filter] of contracts) {
  test(`${description} @spec:${ac}`, () => {
    runCargoTest(filter);
  });
}