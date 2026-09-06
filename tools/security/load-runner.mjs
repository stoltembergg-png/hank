#!/usr/bin/env node
// Deterministic load-contract runner. No host metrics or production traffic.
import { readFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..', '..');
const manifestPath = resolve(root, 'docs/performance/load-manifest.json');
const defaultOut = resolve(root, 'security/reports/load.json');
const outArg = process.argv.indexOf('--out');
const outPath = outArg >= 0 && process.argv[outArg + 1] ? resolve(root, process.argv[outArg + 1]) : defaultOut;
function runGit(args) {
  const result = spawnSync('git', args, { cwd: root, encoding: 'utf8' });
  if (result.status !== 0) process.exit(1);
  return result.stdout.trim();
}
const dirty = runGit(['status', '--porcelain', '--untracked-files=all']);
if (dirty) {
  process.stderr.write('load runner requires a clean checkout before execution\\n');
  process.exit(1);
}
const headSha = runGit(['rev-parse', 'HEAD']);
const treeSha = runGit(['rev-parse', 'HEAD^{tree}']);
const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
const expectedProfiles = ['S', 'M', 'L'];
const expectedFixtureDigest = '8caf08c83a68555b411c01ee6dd6b34c140123f98c81e735ca38a2ae599fa031';
const valid = manifest.revision === 'PR-262'
  && manifest.seed === 26200
  && manifest.warmup_iterations <= 8
  && manifest.repetitions >= 1 && manifest.repetitions <= 8
  && JSON.stringify(manifest.profiles) === JSON.stringify(expectedProfiles)
  && manifest.fixture_digest === expectedFixtureDigest
  && manifest.host_metrics === false
  && manifest.production_traffic === false
  && manifest.credentials === false
  && typeof manifest.fixture_digest === 'string' && manifest.fixture_digest.length === 64;
if (!valid) {
  process.stderr.write('load manifest validation failed\n');
  process.exit(1);
}
const result = spawnSync('cargo', ['test', '-p', 'test-support', '--test', 'load_contract', '--locked', '--offline'], {
  cwd: root,
  encoding: 'utf8',
  env: { ...process.env, CARGO_TERM_COLOR: 'never', RUSTFLAGS: '' },
  timeout: 120_000,
  killSignal: 'SIGTERM',
});
const timedOut = result.error?.code === 'ETIMEDOUT';
const passed = result.status === 0;
const receipt = {
  schema_version: 1,
  revision: manifest.revision,
  head_sha: headSha,
  tree_sha: treeSha,
  profiles: manifest.profiles,
  seed: manifest.seed,
  fixture_digest: manifest.fixture_digest,
  cargo_exit_code: result.status ?? -1,
  cargo_timed_out: timedOut,
  status: passed ? 'pass' : 'fail',
};
mkdirSync(dirname(outPath), { recursive: true });
writeFileSync(outPath, `${JSON.stringify(receipt, null, 2)}\n`, { mode: 0o600 });
if (!passed) {
  process.stderr.write(result.stderr ?? 'load contract failed\n');
  process.exit(result.status ?? 1);
}
console.log(JSON.stringify({ status: receipt.status, path: outPath.replace(`${root}/`, '') }));
