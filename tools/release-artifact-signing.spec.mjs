import assert from 'node:assert/strict';
import { createPublicKey, generateKeyPairSync } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import {
  signArtifacts,
  verifyArtifacts,
  signingMetadata,
  assertPublicKeyMatches,
} from './release-artifact-signing.mjs';

const identity = {
  repository: 'stoltembergg-png/hank',
  event: 'push',
  ref: 'refs/heads/main',
  commit: 'a'.repeat(40),
  tree: 'b'.repeat(40),
  workflow: 'Publish testable prerelease',
  policy: 'release-prerelease-v1',
  channel: 'prerelease',
  os: 'multi',
};

function fixture() {
  const directory = mkdtempSync(join(process.env.TEMP ?? process.env.TMP ?? '.', 'hank-signing-'));
  const attestationDirectory = join(directory, 'attestations');
  const names = ['hank-v1.0.0-dev.a.tar.gz', 'hank-v1.0.0-dev.a-setup.exe', 'hank-v1.0.0-dev.a-x86_64.AppImage'];
  names.forEach((name, index) => writeFileSync(join(directory, name), `artifact-${index}`));
  const { privateKey, publicKey } = generateKeyPairSync('ed25519');
  return { directory, attestationDirectory, names, privateKey, publicKey };
}

test('real release artifacts are signed and independently verified by exact identity', () => {
  const state = fixture();
  try {
    const signed = signArtifacts({
      directory: state.directory,
      attestationDirectory: state.attestationDirectory,
      names: state.names,
      identity,
      signerKeyId: 'release-key-v1',
      privateKey: state.privateKey,
    });
    assert.equal(signed.attestations.length, 3);
    assert.equal(signed.publicKeyPem.includes('PRIVATE'), false);
    assert.deepEqual(verifyArtifacts({
      directory: state.directory,
      attestationDirectory: state.attestationDirectory,
      names: state.names,
      identity,
      signerKeyId: 'release-key-v1',
      publicKey: state.publicKey,
    }), { verified: 3, artifacts: [...state.names].sort() });
  } finally {
    rmSync(state.directory, { recursive: true, force: true });
  }
});

test('substituted, missing, or wrong-identity release evidence fails closed', () => {
  const state = fixture();
  try {
    signArtifacts({
      directory: state.directory,
      attestationDirectory: state.attestationDirectory,
      names: state.names,
      identity,
      signerKeyId: 'release-key-v1',
      privateKey: state.privateKey,
    });
    writeFileSync(join(state.directory, state.names[1]), 'substituted');
    assert.throws(() => verifyArtifacts({
      directory: state.directory,
      attestationDirectory: state.attestationDirectory,
      names: state.names,
      identity,
      signerKeyId: 'release-key-v1',
      publicKey: state.publicKey,
    }), /artifact digest mismatch/);
    assert.throws(() => verifyArtifacts({
      directory: state.directory,
      attestationDirectory: state.attestationDirectory,
      names: state.names,
      identity: { ...identity, tree: 'c'.repeat(40) },
      signerKeyId: 'release-key-v1',
      publicKey: state.publicKey,
    }), /tree mismatch/);
    writeFileSync(join(state.directory, state.names[1]), 'artifact-1');
    assert.throws(() => verifyArtifacts({
      directory: state.directory,
      attestationDirectory: state.attestationDirectory,
      names: [...state.names, 'missing.bin'],
      identity,
      signerKeyId: 'release-key-v1',
      publicKey: state.publicKey,
    }), /artifact is not a file/);
  } finally {
    rmSync(state.directory, { recursive: true, force: true });
  }
});

test('signing metadata contains public identity only and rejects invalid signer fields', () => {
  const state = fixture();
  try {
    const metadata = signingMetadata({
      identity,
      signerKeyId: 'release-key-v1',
      publicKey: createPublicKey(state.privateKey),
      artifacts: state.names,
    });
    assert.equal(metadata.algorithm, 'ed25519');
    assert.equal(metadata.signerKeyId, 'release-key-v1');
    assert.deepEqual(metadata.artifacts, [...state.names].sort());
    assert.equal(JSON.stringify(metadata).includes('PRIVATE'), false);
    assert.equal(assertPublicKeyMatches(metadata.publicKeyPem, state.publicKey), true);
    const other = generateKeyPairSync('ed25519');
    assert.throws(() => assertPublicKeyMatches(metadata.publicKeyPem, other.publicKey), /public key mismatch/);
    assert.throws(() => signingMetadata({
      identity,
      signerKeyId: '',
      publicKey: state.publicKey,
      artifacts: state.names,
    }), /signer key id/);
  } finally {
    rmSync(state.directory, { recursive: true, force: true });
  }
});

test('CLI requires the protected public key when verifying a release', () => {
  const state = fixture();
  try {
    signArtifacts({
      directory: state.directory,
      attestationDirectory: state.attestationDirectory,
      names: state.names,
      identity,
      signerKeyId: 'release-key-v1',
      privateKey: state.privateKey,
    });
    const args = [
      fileURLToPath(new URL('./release-artifact-signing.mjs', import.meta.url)),
      'verify',
      '--directory', state.directory,
      '--attestationDirectory', state.attestationDirectory,
      '--artifacts', state.names.join(','),
      '--commit', identity.commit,
      '--tree', identity.tree,
      '--requireTrustedKey', '1',
    ];
    const publicKeyPem = createPublicKey(state.privateKey).export({ type: 'spki', format: 'pem' }).toString();
    const valid = spawnSync(process.execPath, args, {
      cwd: process.cwd(),
      env: { ...process.env, HANK_RELEASE_SIGNING_PUBLIC_KEY_PEM: publicKeyPem },
      encoding: 'utf8',
    });
    assert.equal(valid.status, 0, valid.stderr);
    const other = generateKeyPairSync('ed25519');
    const invalid = spawnSync(process.execPath, args, {
      cwd: process.cwd(),
      env: {
        ...process.env,
        HANK_RELEASE_SIGNING_PUBLIC_KEY_PEM: createPublicKey(other.privateKey).export({ type: 'spki', format: 'pem' }).toString(),
      },
      encoding: 'utf8',
    });
    assert.equal(invalid.status, 1);
    assert.match(invalid.stderr, /public key mismatch/);
  } finally {
    rmSync(state.directory, { recursive: true, force: true });
  }
});
