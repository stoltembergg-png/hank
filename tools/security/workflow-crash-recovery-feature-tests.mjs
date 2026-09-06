#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = process.env.HANK_RUNNER_ROOT
  ? resolve(process.env.HANK_RUNNER_ROOT)
  : resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');

function gitText(args) {
  const result = spawnSync('git', args, { cwd: root, encoding: 'utf8' });
  if (result.status !== 0) process.exit(1);
  return result.stdout;
}
function identity() {
  return {
    head: gitText(['rev-parse', 'HEAD']).trim(),
    tree: gitText(['rev-parse', 'HEAD^{tree}']).trim(),
    status: gitText(['status', '--porcelain=v1', '-z', '--untracked-files=all']),
  };
}
function statusAllowed(status) {
  const fields = status.split('\0').filter(Boolean);
  for (let i = 0; i < fields.length; i += 1) {
    const entry = fields[i];
    const code = entry.slice(0, 2);
    const paths = [entry.slice(3)];
    if (code[0] === 'R' || code[0] === 'C') paths.push(fields[++i] ?? '');
    if (paths.some((path) => !path.startsWith('.spec/verification/')
      && !path.startsWith('security/reports/'))) return false;
  }
  return true;
}
const before = identity();
const generatedEvidenceAllowed = process.env.HANK_ALLOW_GENERATED_EVIDENCE === '1';
if (before.status && (!generatedEvidenceAllowed || !statusAllowed(before.status))) {
  process.stderr.write('workflow recovery runner requires a clean checkout\n');
  process.exit(1);
}
const tests = [
  ['lease_fencing_rejects_competing_runner', 'AC-1051'],
  ['recovery_is_bounded_and_increments_generation', 'AC-1052'],
  ['recovery_marks_unknown_without_execution', 'AC-1052'],
  ['repeated_recovery_does_not_duplicate_active_lease', 'AC-1053'],
  ['invalid_recovery_inputs_fail_without_mutation', 'AC-1053'],
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
const after = identity();
if (after.head !== before.head || after.tree !== before.tree || after.status !== before.status
  || (after.status && !statusAllowed(after.status))) {
  process.stderr.write('workflow recovery runner identity changed during execution\n');
  process.exit(1);
}
console.log('TAP version 13');
tests.forEach(([name, ac], i) => console.log(`ok ${i + 1} - rust::${name} @spec:${ac}`));
console.log(`1..${tests.length}`);
console.log(`# head_sha ${before.head}`);
console.log(`# tree_sha ${before.tree}`);
console.log(`# pass ${tests.length}`);
console.log('# fail 0');
