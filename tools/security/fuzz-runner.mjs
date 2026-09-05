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
if (!manifest.targets || !Array.isArray(manifest.targets)) {
  console.error('manifest sem targets válidos');
  process.exit(1);
}

// --- Runner digest (hash do manifest) ---

const manifestBytes = readFileSync(manifestPath);
const runnerDigest = createHash('sha256').update(manifestBytes).digest('hex');
const treeSha = spawnSync('git', ['rev-parse', 'HEAD^{tree}'], {
  cwd: root, encoding: 'utf8',
}).stdout.trim();
const headSha = spawnSync('git', ['rev-parse', 'HEAD'], {
  cwd: root, encoding: 'utf8',
}).stdout.trim();

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
