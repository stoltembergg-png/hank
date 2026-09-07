import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { buildSbom, verifySbom } from './release-sbom.mjs';

const commit = 'a'.repeat(40);
const tree = 'b'.repeat(40);

function fixture() {
  const root = mkdtempSync(join(tmpdir(), 'hank-sbom-'));
  writeFileSync(join(root, 'Cargo.lock'), `version = 4\n\n[[package]]\nname = "agent-core"\nversion = "1.0.0"\n\n[[package]]\nname = "serde"\nversion = "1.0.0"\n`);
  writeFileSync(join(root, 'frontend-package-lock.json'), JSON.stringify({
    lockfileVersion: 3,
    packages: {
      '': { name: 'hank-frontend', version: '1.0.0' },
      'node_modules/react': { version: '19.0.0', name: 'react', resolved: 'https://registry.invalid/react' },
      'node_modules/@scope/tool': { version: '2.0.0', name: '@scope/tool' },
    },
  }));
  return root;
}

test('builds a deterministic SPDX SBOM bound to commit, tree, and version', () => {
  const root = fixture();
  try {
    const first = buildSbom({ root, commit, tree, version: '1.0.0', npmLockfiles: ['frontend-package-lock.json'] });
    const second = buildSbom({ root, commit, tree, version: '1.0.0', npmLockfiles: ['frontend-package-lock.json'] });
    assert.deepEqual(first, second);
    assert.equal(first.spdxVersion, 'SPDX-2.3');
    assert.equal(first.creationInfo.comment, `sourceCommit=${commit}; sourceTree=${tree}; version=1.0.0`);
    assert.equal(first.packages.length, 4);
    assert.deepEqual(first.packages.map((pkg) => pkg.name), ['@scope/tool', 'agent-core', 'react', 'serde']);
    assert.equal(verifySbom({ sbom: first, expectedCommit: commit, expectedTree: tree, expectedVersion: '1.0.0' }), true);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('fails closed on missing lockfiles, identity drift, and duplicate package records', () => {
  const root = fixture();
  try {
    assert.throws(() => buildSbom({ root, commit, tree, version: '1.0.0', npmLockfiles: ['missing.json'] }), /lockfile is missing/);
    const sbom = buildSbom({ root, commit, tree, version: '1.0.0', npmLockfiles: ['frontend-package-lock.json'] });
    assert.throws(() => verifySbom({ sbom: { ...sbom, creationInfo: { ...sbom.creationInfo, comment: 'tampered' } }, expectedCommit: commit, expectedTree: tree, expectedVersion: '1.0.0' }), /provenance/);
    assert.throws(() => verifySbom({ sbom: { ...sbom, packages: [...sbom.packages, sbom.packages[0]] }, expectedCommit: commit, expectedTree: tree, expectedVersion: '1.0.0' }), /package IDs must be unique/);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('rejects invalid release identity and malformed lockfile packages', () => {
  const root = mkdtempSync(join(tmpdir(), 'hank-sbom-invalid-'));
  try {
    writeFileSync(join(root, 'Cargo.lock'), 'version = 4\n\n[[package]]\nname = "missing-version"\n');
    assert.throws(() => buildSbom({ root, commit: 'short', tree, version: '1.0.0' }), /identity/);
    assert.throws(() => buildSbom({ root, commit, tree, version: '1.0.0' }), /package version/);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
