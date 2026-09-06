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
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '..', '..');
const EXPECTED_MANIFEST_REVISION = 'FT-001';
const EXPECTED_TARGET_IDS = ['FT-001', 'FT-002', 'FT-003', 'FT-004', 'FT-005', 'FT-006', 'FT-007'];
const VALID_KINDS = new Set([
  'envelope',
  'policy',
  'state',
  'permission',
  'release_metadata',
  'hash_chain',
  'rate_limit',
]);

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
  if (!VALID_KINDS.has(target.kind)) {
    console.error(`kind inválido para ${target.id}: ${target.kind}`);
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

const runnerSource = readFileSync(fileURLToPath(import.meta.url));
const runnerDigest = createHash('sha256').update(runnerSource).digest('hex');
if (manifest.runner_digest !== runnerDigest) {
  console.error('runner_digest mismatch: RUNNER_DIGEST_MISMATCH');
  process.exit(1);
}
const treeSha = spawnSync('git', ['rev-parse', 'HEAD^{tree}'], {
  cwd: root, encoding: 'utf8',
}).stdout.trim();
const headSha = spawnSync('git', ['rev-parse', 'HEAD'], {
  cwd: root, encoding: 'utf8',
}).stdout.trim();
if (!treeSha || !headSha) {
  console.error('git revision metadata ausente');
  process.exit(1);
}

// --- Executar contrato Rust ---

const cargoTest = spawnSync(
  'cargo',
  [
    'test',
    '-p', 'test-support',
    '--test', 'fuzz_contract',
    '--locked',
  ],
  {
    cwd: root,
    encoding: 'utf8',
    env: { ...process.env, RUSTFLAGS: '' },
  },
);

const contractPassed = cargoTest.status === 0;
const report = {
  status: contractPassed ? 'pass' : 'fail',
  schema_version: 1,
  tree_sha: treeSha,
  head_sha: headSha,
  runner_digest: runnerDigest,
  manifest_revision: manifest.manifest_revision,
  cargo_exit_code: cargoTest.status ?? -1,
  cargo_output: cargoTest.stdout ?? '',
  cargo_stderr: contractPassed ? undefined : (cargoTest.stderr ?? ''),
  target_count: manifest.targets.length,
  timestamp_iso: new Date().toISOString(),
};

// --- Relatório ---

mkdirSync(resolve(root, 'security', 'reports'), { recursive: true });
writeFileSync(outPath, JSON.stringify(report, null, 2) + '\n', 'utf8');
console.log(`fuzz: status=${report.status} tree=${treeSha.slice(0,12)}... head=${headSha.slice(0,12)}... runner_digest=${runnerDigest.slice(0,12)}...`);
if (report.status !== 'pass') {
  console.error(report.cargo_stderr);
  process.exit(1);
}
