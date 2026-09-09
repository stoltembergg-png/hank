import assert from 'node:assert/strict';
import { generateKeyPairSync } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import test from 'node:test';
import { artifactDigest, signAttestation } from './release-signing.mjs';
import { signingMetadata } from './release-artifact-signing.mjs';
import {
  createUpdaterBundle,
  verifyUpdaterBundle,
  versionCode,
} from './release-updater-bundle.mjs';

const commit = 'a'.repeat(40);
const tree = 'b'.repeat(40);

function fixtureIdentity() {
  return {
    repository: 'stoltembergg-png/hank',
    event: 'workflow_dispatch',
    ref: 'refs/tags/v1.0.0',
    commit,
    tree,
    workflow: 'Publish testable prerelease',
    policy: 'release-prerelease-v1',
    channel: 'prerelease',
    os: 'multi',
  };
}

test('version codes are monotonic and bounded', () => {
  assert.equal(versionCode('v1.2.3'), 1_002_003);
  assert.equal(versionCode('2.0.0-beta.1'), 2_000_000);
  assert.ok(versionCode('2.0.0') > versionCode('1.99.99'));
  assert.throws(() => versionCode('not-semver'), /version must be semver/);
});

test('release updater bundle signs and verifies two real artifact versions', () => {
  const root = mkdtempSync(join(tmpdir(), 'hank-release-updater-'));
  try {
    const artifactPath = join(root, 'hank-setup.exe');
    const attestationPath = join(root, 'hank-setup.exe.attestation.json');
    const outputPath = join(root, 'hank-updater.json');
    const bytes = Buffer.from('release-binary-fixture');
    const { privateKey, publicKey } = generateKeyPairSync('ed25519');
    const base = signAttestation({
      artifact: { name: 'hank-setup.exe', digest: artifactDigest(bytes), size: bytes.length },
      identity: fixtureIdentity(),
      signer: { keyId: 'release-key-v1' },
    }, privateKey);
    writeFileSync(artifactPath, bytes);
    writeFileSync(attestationPath, `${JSON.stringify(base)}\n`);

    const bundle = createUpdaterBundle({
      artifactPath,
      attestationPath,
      outputPath,
      version: 1_002_003,
      nextVersion: 1_002_004,
      expiresAt: 4_102_444_800,
      os: 'windows',
      arch: 'x86_64',
      privateKey,
      publicKey,
    });
    assert.equal(bundle.schemaVersion, 1);
    assert.equal(bundle.updates.length, 2);
    assert.equal(bundle.updates[0].attestation.update.os, 'windows');
    assert.equal(bundle.updates[1].attestation.update.version, 1_002_004);
    writeFileSync(join(root, 'release-signing-metadata.json'), `${JSON.stringify(signingMetadata({
      identity: fixtureIdentity(), signerKeyId: 'release-key-v1', publicKey, artifacts: ['hank-setup.exe'],
    }))}\n`);
    const helper = spawnSync(process.execPath, [
      'desktop-e2e/updater-release-env.mjs', '--bundle', outputPath, '--artifact', artifactPath,
      '--signing-metadata', join(root, 'release-signing-metadata.json'),
    ], { encoding: 'utf8' });
    assert.equal(helper.status, 0, helper.stderr);
    assert.equal(JSON.parse(helper.stdout).verified.verified, 2);
    assert.deepEqual(verifyUpdaterBundle({
      bundlePath: outputPath,
      artifactPath,
      publicKey,
    }), { verified: 2, artifact: 'hank-setup.exe', digest: artifactDigest(bytes) });

    const tampered = Buffer.from('substituted-binary');
    writeFileSync(artifactPath, tampered);
    assert.throws(() => verifyUpdaterBundle({ bundlePath: outputPath, artifactPath, publicKey }), /artifact digest mismatch/);
    assert.deepEqual(JSON.parse(readFileSync(outputPath, 'utf8')).artifact.digest, artifactDigest(bytes));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
