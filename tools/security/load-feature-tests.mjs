#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
function git(args) {
  const value = spawnSync('git', args, { cwd: root, encoding: 'utf8' });
  if (value.status !== 0) process.exit(1);
  return value.stdout.trim();
}
const status = git(['status', '--porcelain=v1', '-z', '--untracked-files=all']);
const entries = status ? status.split('\0').filter(Boolean) : [];
const allowed = (path) => path.startsWith('.spec/verification/') || path.startsWith('security/reports/');
const unexpectedDirty = [];
for (let index = 0; index < entries.length; index += 1) {
  const entry = entries[index];
  const code = entry.slice(0, 2);
  const path = entry.slice(3);
  if (!allowed(path)) unexpectedDirty.push(path);
  if (code.includes('R') || code.includes('C')) {
    const original = entries[index + 1] ?? '';
    index += 1;
    if (!allowed(original)) unexpectedDirty.push(original);
  }
}
if (unexpectedDirty.length > 0) {
  process.stderr.write('load feature tests require a clean source checkout\n');
  process.exit(1);
}
const headSha = git(['rev-parse', 'HEAD']);
const treeSha = git(['rev-parse', 'HEAD^{tree}']);
const tests = [
  ['manifest_declares_bounded_profiles_and_fixture_digest', 'AC-2301'],
  ['admission_and_backpressure_are_explicit', 'AC-2302'],
  ['cancellation_and_completion_are_accounted', 'AC-2303'],
  ['repeated_runs_are_deterministic_and_redacted', 'AC-2304'],
  ['invalid_manifest_fails_closed', 'AC-2305'],
  ['invalid_plan_returns_typed_error', 'AC-2305'],
];
const safeEnv = {
  PATH: process.env.PATH,
  HOME: process.env.HOME,
  CI: '1',
  CARGO_HOME: process.env.CARGO_HOME,
  RUSTUP_HOME: process.env.RUSTUP_HOME,
  CARGO_TERM_COLOR: 'never',
  CARGO_INCREMENTAL: '0',
  CARGO_BUILD_JOBS: '1',
  CARGO_NET_OFFLINE: 'true',
  RUSTFLAGS: '',
};
const result = spawnSync('cargo', ['test', '-p', 'test-support', '--test', 'load_contract', '--locked', '--offline'], {
  cwd: root,
  encoding: 'utf8',
  env: safeEnv,
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
console.log(`# head_sha ${headSha}`);
console.log(`# tree_sha ${treeSha}`);
console.log(`# tests ${tests.length}`);
console.log(`# pass ${tests.length}`);
console.log('# fail 0');
