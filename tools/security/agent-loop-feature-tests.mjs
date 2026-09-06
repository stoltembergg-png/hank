#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const tests = [
  ['success_and_trace_are_deterministic', 'AC-2501'],
  ['duplicate_tool_is_not_charged_twice', 'AC-2502'],
  ['permission_cycle_depth_and_budget_stop_fail_closed', 'AC-2503'],
  ['cancellation_and_stale_event_do_not_advance', 'AC-2504'],
  ['invalid_policy_and_turn_bound_fail_closed', 'AC-2505'],
];
const run = spawnSync('cargo', ['test', '-p', 'test-support', '--test', 'agent_loop_contract', '--locked'], {
  cwd: root,
  encoding: 'utf8',
  env: { PATH: process.env.PATH, HOME: process.env.HOME, CARGO_HOME: process.env.CARGO_HOME, RUSTUP_HOME: process.env.RUSTUP_HOME, CARGO_TERM_COLOR: 'never', CARGO_INCREMENTAL: '0', CARGO_BUILD_JOBS: '1', RUSTFLAGS: '' },
  timeout: 180_000,
});
if (run.status !== 0) {
  process.stderr.write(run.stderr ?? 'agent loop contract failed\n');
  process.exit(run.status ?? 1);
}
const observed = [...run.stdout.matchAll(/^test (\S+) \.\.\. (ok|FAILED|ignored)$/gm)].map((match) => [match[1], match[2]]).sort();
const expected = [...tests].sort();
if (observed.length !== expected.length || expected.some(([name], index) => observed[index]?.[0] !== name || observed[index]?.[1] !== 'ok')) {
  process.stderr.write('agent loop test set diverged\n');
  process.exit(1);
}
const git = (args) => {
  const result = spawnSync('git', args, { cwd: root, encoding: 'utf8' });
  if (result.status !== 0) process.exit(1);
  return result.stdout.trim();
};
console.log('TAP version 13');
tests.forEach(([name, ac], index) => console.log(`ok ${index + 1} - rust::${name} @spec:${ac}`));
console.log(`1..${tests.length}`);
console.log(`# head_sha ${git(['rev-parse', 'HEAD'])}`);
console.log(`# tree_sha ${git(['rev-parse', 'HEAD^{tree}'])}`);
console.log(`# pass ${tests.length}`);
console.log('# fail 0');
