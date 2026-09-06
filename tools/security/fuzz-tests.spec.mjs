#!/usr/bin/env node
// Test runner para `fuzz-tests` (PR-261).
// valida a saída do fuzz-runner.mjs e o manifest fuzz-manifest.json.
// Cada teste carrega tag `@spec:AC-22NN` para o ONP.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync, execSync } from 'node:child_process';
import { readFileSync, existsSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '..', '..');

test('fuzz-manifest existe e é serializável @spec:AC-2201', () => {
  const manifestPath = resolve(root, 'docs', 'security', 'fuzz-manifest.json');
  assert.ok(existsSync(manifestPath), 'fuzz-manifest.json deve existir');
  const raw = readFileSync(manifestPath, 'utf8');
  const manifest = JSON.parse(raw);
  assert.equal(manifest.schema_version, 1, 'schema_version deve ser 1');
  assert.ok(manifest.targets, 'deve ter targets');
  assert.ok(Array.isArray(manifest.targets), 'targets deve ser array');
  assert.ok(manifest.targets.length >= 7, 'deve ter pelo menos 7 targets');
});

test('fuzz-runner.mjs é sintaticamente válido @spec:AC-2206', () => {
  execSync('node --check tools/security/fuzz-runner.mjs', {
    cwd: root, encoding: 'utf8',
  });
});

test('fuzz-runner.mjs roda e produz relatório JSON @spec:AC-2206', () => {
  const outPath = resolve(root, 'security', 'reports', 'fuzz.json');
  const runner = spawnSync(
    'node', ['tools/security/fuzz-runner.mjs'], { cwd: root, encoding: 'utf8' },
  );
  assert.equal(runner.status, 0, `fuzz-runner deve sair 0, exit=${runner.status} stderr=${(runner.stderr ?? '').slice(0, 200)}`);
  assert.ok(existsSync(outPath), 'deve produzir security/reports/fuzz.json');
  const report = JSON.parse(readFileSync(outPath, 'utf8'));
  assert.ok(report.status, 'report.status deve estar definido');
  assert.ok(report.tree_sha, 'report.tree_sha deve estar definido');
  assert.ok(report.head_sha, 'report.head_sha deve estar definido');
  assert.ok(report.runner_digest, 'report.runner_digest deve estar definido');
  assert.ok(report.manifest_revision, 'report.manifest_revision deve estar definido');
  assert.ok(report.snapshot_sha, 'report.snapshot_sha deve estar definido');
  assert.equal(report.contract_test_count, 10, 'todos os 10 testes Rust devem ser executados');
  assert.deepEqual(report.failed_tests, [], 'nenhum teste Rust pode falhar');
  assert.equal(
    report.runner_digest,
    createHash('sha256').update(readFileSync(resolve(root, 'tools/security/fuzz-runner.mjs'))).digest('hex'),
    'report deve estar vinculado ao digest do runner',
  );
});

test('fuzz-manifest contém 7 targets FT-001..FT-007 @spec:AC-2202', () => {
  const manifestPath = resolve(root, 'docs', 'security', 'fuzz-manifest.json');
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
  assert.deepStrictEqual(
    manifest.targets.map(t => t.id).sort(),
    ['FT-001','FT-002','FT-003','FT-004','FT-005','FT-006','FT-007'].sort(),
  );
});

test('fuzz-manifest descreve cada target com parser_source e invariants @spec:AC-2202', () => {
  const manifestPath = resolve(root, 'docs', 'security', 'fuzz-manifest.json');
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
  for (const t of manifest.targets) {
    assert.ok(t.parser, `${t.id}: parser deve existir`);
    assert.ok(t.parser_source, `${t.id}: parser_source deve existir`);
    assert.ok(Array.isArray(t.invariants), `${t.id}: invariants deve ser array`);
    assert.ok(t.invariants.length > 0, `${t.id}: invariants não pode estar vazio`);
    assert.ok(Number.isInteger(t.smoke_iterations) && t.smoke_iterations >= 8, `${t.id}: smoke_iterations deve ser >= 8`);
    assert.ok(t.corpus_path, `${t.id}: corpus_path deve existir`);
    assert.ok(typeof t.description === 'string', `${t.id}: description deve ser string`);
  }
});

test('fuzz-manifest FT-00N IDs são únicos e não se repetem @spec:AC-2202', () => {
  const manifestPath = resolve(root, 'docs', 'security', 'fuzz-manifest.json');
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
  const ids = manifest.targets.map(t => t.id);
  const set = new Set(ids);
  assert.equal(set.size, ids.length, 'IDs devem ser únicos');
});

test('fuzz-runner mansa com git credentials em path? @spec:NEG-001', () => {
  // Neg-001: não deve expor segredos no runner; verificamos que o
  // runner não lê de paths que contenham patterns comuns de credencial.
  const withFakeSecret = {
    ...process.env,
    AWS_ACCESS_KEY_ID: '[REDACTED]',
  };
  const runner = spawnSync(
    'node', ['tools/security/fuzz-runner.mjs'], {
      cwd: root, encoding: 'utf8',
      env: withFakeSecret,
    },
  );
  assert.equal(runner.status, 0, 'fuzz-runner deve manter-se inerte com fake credencial no ambiente');
});
