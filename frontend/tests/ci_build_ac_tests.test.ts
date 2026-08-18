import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { describe, expect, it } from 'vitest';

const FRONTEND_ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const REPOSITORY_ROOT = join(FRONTEND_ROOT, '..');
const WORKFLOW_ROOT = join(REPOSITORY_ROOT, '.github', 'workflows');

function workflow(name: string): string {
  const path = join(WORKFLOW_ROOT, name);
  expect(existsSync(path)).toBe(true);
  return readFileSync(path, 'utf8');
}

function assertCommonGateProperties(content: string): void {
  expect(content).not.toMatch(/pull_request:\s*\n\s+branches:\s+\[main\]/);
  expect(content).toContain('workflow_dispatch:');
  expect(content).toContain('permissions:');
  expect(content).toContain('contents: read');
  expect(content).toContain('concurrency:');
  expect(content).toContain('cancel-in-progress: true');
  expect(content).toMatch(/timeout-minutes:\s*15/);
  expect(content).not.toContain('continue-on-error');
  for (const action of content.matchAll(/uses:\s+([^\s#]+)/g)) {
    expect(action[1]).toMatch(/@[0-9a-f]{40}$/);
  }
}

describe('CI Build workflow AC tests', () => {
  // @spec:AC-401
  it('AC-401: workflow Rust executa check/build e publica artifact com digest', () => {
    const rust = workflow('build-rust.yml');
    assertCommonGateProperties(rust);
    expect(rust).toContain('cargo check --workspace');
    expect(rust).toContain('cargo build --workspace');
    expect(rust).toContain("hashFiles('Cargo.lock', 'rust-toolchain.toml')");
    expect(rust).toContain('path: target/debug/');
    expect(rust).toContain('if-no-files-found: error');
    expect(rust).toContain('artifact-digest');
    expect(rust).toContain('tools/ci/require-artifact-digest.sh');
  });

  // @spec:AC-402
  it('AC-402: workflow frontend executa npm ci/build e publica artifact com digest', () => {
    const frontend = workflow('build-frontend.yml');
    assertCommonGateProperties(frontend);
    expect(frontend).toContain('node-version: 20.19.1');
    expect(frontend).toContain('cache-dependency-path: frontend/package-lock.json');
    expect(frontend).toContain('npm ci --no-audit --no-fund');
    expect(frontend).toContain('npm run build');
    expect(frontend).toContain('path: frontend/dist/');
    expect(frontend).toContain('if-no-files-found: error');
    expect(frontend).toContain('artifact-digest');
    expect(frontend).toContain('tools/ci/require-artifact-digest.sh');
  });

  // @spec:AC-403
  it('AC-403: caches are keyed by lockfiles', () => {
    const rust = workflow('build-rust.yml');
    const frontend = workflow('build-frontend.yml');
    expect(rust).toContain("hashFiles('Cargo.lock', 'rust-toolchain.toml')");
    expect(frontend).toContain('cache-dependency-path: frontend/package-lock.json');
  });

  // @spec:AC-404
  it('AC-404: toolchains and minimal runner are explicit', () => {
    const rust = workflow('build-rust.yml');
    const frontend = workflow('build-frontend.yml');
    expect(rust).toContain('runs-on: ubuntu-latest');
    expect(rust).toContain('toolchain: 1.97.0');
    expect(frontend).toContain('runs-on: ubuntu-latest');
    expect(frontend).toContain('node-version: 20.19.1');
  });

  // @spec:AC-405
  it('AC-405: fixture rejeita artifact sem digest', () => {
    const script = join(REPOSITORY_ROOT, 'tools', 'ci', 'require-artifact-digest.sh');
    const missing = spawnSync('bash', [script], {
      env: { ...process.env, ARTIFACT_DIGEST: '' },
      encoding: 'utf8',
    });
    expect(missing.status).not.toBe(0);
    expect(missing.stderr).toContain('artifact digest is missing');

    const valid = spawnSync('bash', [script], {
      env: { ...process.env, ARTIFACT_DIGEST: 'sha256:test' },
      encoding: 'utf8',
    });
    expect(valid.status).toBe(0);
  });
});
