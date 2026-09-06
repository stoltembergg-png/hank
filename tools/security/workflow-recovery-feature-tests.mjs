#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const tests = [
  ['lease_fencing_rejects_competing_runner', 'AC-2401'],
  ['recovery_is_bounded_and_increments_generation', 'AC-2402'],
  ['recovery_marks_unknown_without_execution', 'AC-2403'],
  ['repeated_recovery_does_not_duplicate_active_lease', 'AC-2404'],
  ['invalid_recovery_inputs_fail_without_mutation', 'AC-2405'],
];
const run = spawnSync('cargo', ['test', '-p', 'agent-runtime', '--test', 'workflow_recovery_contract', '--locked', '--offline'], {
  cwd: root,
  encoding: 'utf8',
  env: { PATH: process.env.PATH, HOME: process.env.HOME, CARGO_HOME: process.env.CARGO_HOME, RUSTUP_HOME: process.env.RUSTUP_HOME, CARGO_TERM_COLOR: 'never', CARGO_INCREMENTAL: '0', CARGO_BUILD_JOBS: '1', RUSTFLAGS: '' },
  timeout: 180_000,
});
if (run.status !== 0) {
  process.stderr.write(run.stderr ?? 'workflow recovery contract failed\n');
  process.exit(run.status ?? 1);
}
const observed = [...run.stdout.matchAll(/^test (\S+) \.\.\. (ok|FAILED|ignored)$/gm)].map((m) => [m[1], m[2]]).sort();
const expected = [...tests].sort();
if (observed.length !== expected.length || expected.some(([name], i) => observed[i]?.[0] !== name || observed[i]?.[1] !== 'ok')) {
  process.stderr.write('workflow recovery test set diverged\n');
  process.exit(1);
}
const head = spawnSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).stdout.trim();
const tree = spawnSync('git', ['rev-parse', 'HEAD^{tree}'], { cwd: root, encoding: 'utf8' }).stdout.trim();
console.log('TAP version 13');
tests.forEach(([name, ac], i) => console.log(`ok ${i + 1} - rust::${name} @spec:${ac}`));
console.log(`1..${tests.length}`);
console.log(`# head_sha ${head}`);
console.log(`# tree_sha ${tree}`);
console.log(`# pass ${tests.length}`);
console.log('# fail 0');
