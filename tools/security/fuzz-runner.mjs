#!/usr/bin/env node
// Runner do fuzz harness para `fuzz-tests` (PR-261).
//
// Sem I/O fora do workspace. Carrega e valida o manifest,
// executa o contrato Rust e emite relatório JSON em
// security/reports/fuzz.json com tree_sha, head_sha, runner_digest
// e status. O runner não afirma ausência de vulnerabilidade, apenas
// que o manifest e a suíte permanecem coerentes.
//
// Cada target FT-NNN é registrado no manifest e exercitado pelo
// contrato Rust `crates/test-support/tests/fuzz_contract.rs`.
//
// Invocação: node tools/security/fuzz-runner.mjs [--out <path>]
// Por padrão, grava em security/reports/fuzz.json.

import { spawnSync } from 'node:child_process';
import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'node:fs';
import { resolve, dirname, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '..', '..');
const CARGO_TIMEOUT_MS = 120_000;
const EXPECTED_MANIFEST_REVISION = 'FT-001';
const EXPECTED_TARGET_IDS = ['FT-001', 'FT-002', 'FT-003', 'FT-004', 'FT-005', 'FT-006', 'FT-007'];
const EXPECTED_TARGET_KINDS = {
  'FT-001': 'envelope',
  'FT-002': 'policy',
  'FT-003': 'state',
  'FT-004': 'permission',
  'FT-005': 'release_metadata',
  'FT-006': 'hash_chain',
  'FT-007': 'rate_limit',
};
const EXPECTED_CONTRACT_TESTS = [
  'manifest_is_well_formed_and_self_consistent_ac_2201',
  'targets_enumerated_and_registered_ac_2202',
  'reproducible_seed_and_corpus_ac_2203',
  'crash_artifact_reproduces_ac_2204',
  'bounded_resource_time_limits_ac_2205',
  'runner_output_is_single_tap_ac_2206',
  'no_credentials_or_unsafe_corpus_in_repo_ac_2207',
  'regression_zero_iterations_is_fail_closed_ac_2205',
  'regression_replay_crash_is_total',
  'regression_each_target_kind_is_exercised',
];

const args = process.argv.slice(2);
let outPath = resolve(root, 'security', 'reports', 'fuzz.json');
for (let i = 0; i < args.length; i++) {
  if (args[i] === '--out' && i + 1 < args.length) {
    outPath = resolve(args[++i]);
  }
}

// --- Validar manifest ---

const manifestPath = resolve(root, 'docs', 'security', 'fuzz-manifest.json');
if (!existsSync(manifestPath)) {
  console.error(`manifest não encontrado: ${manifestPath}`);
  process.exit(1);
}
const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));

if (manifest.schema_version !== 1) {
  console.error(`manifest schema_version inválido: ${manifest.schema_version}`);
  process.exit(1);
}
if (manifest.manifest_revision !== EXPECTED_MANIFEST_REVISION) {
  console.error(`manifest_revision inválida: ${manifest.manifest_revision}`);
  process.exit(1);
}
if (!manifest.targets || !Array.isArray(manifest.targets)) {
  console.error('manifest sem targets válidos');
  process.exit(1);
}
if (manifest.targets.length !== EXPECTED_TARGET_IDS.length) {
  console.error(`quantidade de targets inválida: ${manifest.targets.length}`);
  process.exit(1);
}
const targetIds = manifest.targets.map((target) => target.id);
if (targetIds.some((id, index) => id !== EXPECTED_TARGET_IDS[index])) {
  console.error('targets fora da enumeração canônica FT-001..FT-007');
  process.exit(1);
}
for (const target of manifest.targets) {
  if (target.kind !== EXPECTED_TARGET_KINDS[target.id]) {
    console.error(`kind divergente para ${target.id}: ${target.kind}`);
    process.exit(1);
  }
  if (typeof target.parser !== 'string' || typeof target.parser_source !== 'string') {
    console.error(`parser ausente para ${target.id}`);
    process.exit(1);
  }
  if (!Array.isArray(target.invariants) || target.invariants.length === 0) {
    console.error(`invariants ausentes para ${target.id}`);
    process.exit(1);
  }
  if (!Number.isInteger(target.smoke_iterations) || target.smoke_iterations < 8) {
    console.error(`smoke_iterations inválido para ${target.id}`);
    process.exit(1);
  }
}

// --- Runner digest (hash do próprio runner) ---

// Hash the canonical LF representation so the manifest is identical on
// Windows checkouts (CRLF) and Linux runners (LF).
const runnerSource = readFileSync(fileURLToPath(import.meta.url), 'utf8').replace(/\r\n/g, '\n');
const runnerDigest = createHash('sha256').update(runnerSource).digest('hex');
if (manifest.runner_digest !== runnerDigest) {
  console.error('runner_digest mismatch: RUNNER_DIGEST_MISMATCH');
  process.exit(1);
}
const runGit = (gitArgs) => {
  const result = spawnSync('git', gitArgs, { cwd: root, encoding: 'utf8' });
  if (result.status !== 0) {
    console.error(`git metadata failed: ${gitArgs.join(' ')}`);
    process.exit(1);
  }
  return result.stdout;
};
const treeSha = runGit(['rev-parse', 'HEAD^{tree}']).trim();
const headSha = runGit(['rev-parse', 'HEAD']).trim();
const stagedDiff = runGit(['diff', '--binary', '--cached', 'HEAD']);
const workingDiff = runGit(['diff', '--binary', 'HEAD']);
const normalizedOutputPath = relative(root, outPath).replaceAll('\\', '/');
const untracked = runGit(['ls-files', '--others', '--exclude-standard'])
  .split('\n')
  .map((path) => path.trim())
  .filter((path) => path && path.replaceAll('\\', '/') !== normalizedOutputPath);
if (untracked.length > 0) {
  console.error(`untracked input files present: ${untracked.join(', ')}`);
  process.exit(1);
}
const snapshotSha = createHash('sha256')
  .update(headSha)
  .update('\u0000')
  .update(stagedDiff)
  .update('\u0000')
  .update(workingDiff)
  .digest('hex');

// --- Executar contrato Rust ---

const cargoTest = spawnSync(
  'cargo',
  [
    'test',
    '-p', 'test-support',
    '--test', 'fuzz_contract',
    '--locked', '--offline',
  ],
  {
    cwd: root,
    encoding: 'utf8',
    env: { ...process.env, CARGO_TERM_COLOR: 'never', RUSTFLAGS: '' },
    timeout: CARGO_TIMEOUT_MS,
    killSignal: 'SIGTERM',
  },
);

const cargoTimedOut = cargoTest.error?.code === 'ETIMEDOUT';
const cargoPassed = cargoTest.status === 0;
const cargoStdout = cargoTest.stdout ?? '';
const executedTests = [...cargoStdout.matchAll(/^test (\S+) \.\.\. (ok|FAILED|ignored)$/gm)]
  .map((match) => ({ name: match[1], outcome: match[2] }));
const failedTests = executedTests
  .filter((test) => test.outcome !== 'ok')
  .map((test) => test.name);
const observedNames = executedTests.map((test) => test.name);
const contractTestsMatch = observedNames.length === EXPECTED_CONTRACT_TESTS.length
  && EXPECTED_CONTRACT_TESTS.every((name) => observedNames.includes(name));
const contractPassed = cargoPassed && contractTestsMatch && failedTests.length === 0;
const report = {
  status: contractPassed ? 'pass' : 'fail',
  schema_version: 1,
  tree_sha: treeSha,
  head_sha: headSha,
  snapshot_sha: snapshotSha,
  working_tree_dirty: stagedDiff.length > 0 || workingDiff.length > 0,
  runner_digest: runnerDigest,
  manifest_revision: manifest.manifest_revision,
  cargo_exit_code: cargoTest.status ?? -1,
  cargo_timeout_ms: CARGO_TIMEOUT_MS,
  cargo_timed_out: cargoTimedOut,
  contract_test_count: executedTests.length,
  failed_tests: failedTests,
  target_count: manifest.targets.length,
};

// --- Relatório ---

mkdirSync(resolve(root, 'security', 'reports'), { recursive: true });
writeFileSync(outPath, JSON.stringify(report, null, 2) + '\n', 'utf8');
console.log(`fuzz: status=${report.status} tree=${treeSha.slice(0,12)}... head=${headSha.slice(0,12)}... runner_digest=${runnerDigest.slice(0,12)}...`);
if (report.status !== 'pass') {
  console.error(cargoTest.stderr ?? 'fuzz contract did not pass');
  process.exit(1);
}
