#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const tests = [
  ['manifest_declares_bounded_profiles_and_fixture_digest', 'AC-2301'],
  ['admission_and_backpressure_are_explicit', 'AC-2302'],
  ['cancellation_and_completion_are_accounted', 'AC-2303'],
  ['repeated_runs_are_deterministic_and_redacted', 'AC-2304'],
  ['invalid_manifest_fails_closed', 'AC-2305'],
];
const result = spawnSync('cargo', ['test', '-p', 'test-support', '--test', 'load_contract', '--locked', '--offline'], {
  cwd: root,
  encoding: 'utf8',
  env: { ...process.env, CARGO_TERM_COLOR: 'never', RUSTFLAGS: '' },
  timeout: 120_000,
  killSignal: 'SIGTERM',
});
if (result.status !== 0) {
  process.stderr.write(result.stderr ?? 'load contract failed\n');
  process.exit(result.status ?? 1);
}
const observed = [...(result.stdout ?? '').matchAll(/^test (\S+) \.\.\. (ok|FAILED|ignored)$/gm)]
  .map((match) => ({ name: match[1], outcome: match[2] }))
  .sort((a, b) => a.name.localeCompare(b.name));
const expected = [...tests].sort(([a], [b]) => a.localeCompare(b));
if (observed.length !== expected.length || expected.some(([name], index) => observed[index]?.name !== name
  || observed[index]?.outcome !== 'ok')) {
  process.stderr.write('cargo test set diverged from load contract\n');
  process.exit(1);
}
console.log('TAP version 13');
tests.forEach(([name, ac], index) => console.log(`ok ${index + 1} - rust::${name} @spec:${ac}`));
console.log(`1..${tests.length}`);
console.log(`# tests ${tests.length}`);
console.log(`# pass ${tests.length}`);
console.log('# fail 0');
