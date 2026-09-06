import test from 'node:test';
import assert from 'node:assert/strict';
import { generateKeyPairSync } from 'node:crypto';
import { artifactDigest, signAttestation, verifyAttestation } from '../tools/release-signing.mjs';

const { publicKey, privateKey } = generateKeyPairSync('ed25519');
const base = {
  artifact: { name: 'hank-linux.tar.gz', digest: artifactDigest('fixture'), size: 7 },
  identity: {
    repository: 'stoltembergg-png/hank', event: 'workflow_dispatch', ref: 'refs/tags/v0.3.0',
    commit: 'a'.repeat(64), tree: 'b'.repeat(64), workflow: 'release.yml',
    policy: 'release-policy-v1', channel: 'stable', os: 'linux-x86_64',
  },
  signer: { keyId: 'fixture-ed25519-v1' },
};

function keyring(revoked = false) {
  return { 'fixture-ed25519-v1': { revoked, publicKey: publicKey.export({ type: 'spki', format: 'der' }).toString('base64') } };
}

function signed() { return signAttestation(structuredClone(base), privateKey); }

test('AC-2661: valid artifact tuple verifies with independent public key @spec:AC-2661', () => {
  const result = verifyAttestation(signed(), publicKey, { commit: base.identity.commit, tree: base.identity.tree, channel: 'stable', trustedKeyring: keyring(), artifactBytes: 'fixture' });
  assert.equal(result.valid, true);
  assert.equal(result.artifactDigest, base.artifact.digest);
});

test('AC-2662: substituted digest is rejected @spec:AC-2662', () => {
  const attestation = signed();
  assert.throws(() => verifyAttestation(attestation, publicKey, { artifactBytes: 'substituted' }), /artifact digest mismatch/);
});

test('AC-2663: wrong commit, tree, channel, or policy is rejected @spec:AC-2663', () => {
  for (const [field, value] of [['commit', 'c'.repeat(64)], ['tree', 'd'.repeat(64)], ['channel', 'nightly'], ['policy', 'other-policy']]) {
    const attestation = signed();
    attestation.identity[field] = value;
    assert.throws(() => verifyAttestation(attestation, publicKey, { [field]: base.identity[field] }), /mismatch/);
  }
});

test('AC-2664: stale or revoked signer identity is rejected @spec:AC-2664', () => {
  const attestation = signed();
  assert.throws(() => verifyAttestation(attestation, publicKey, { trustedKeyring: keyring(true) }), /untrusted or revoked signer/);
});

test('AC-2665: malformed or incomplete proof fails closed @spec:AC-2665', () => {
  for (const mutation of [
    (a) => { delete a.identity.tree; },
    (a) => { a.artifact.digest = 'sha256:bad'; },
    (a) => { a.signature.value = ''; },
    (a) => { a.schemaVersion = 99; },
  ]) {
    const attestation = signed();
    mutation(attestation);
    assert.throws(() => verifyAttestation(attestation, publicKey));
  }
});

test('AC-2666: oversized identity fields are rejected @spec:AC-2666', () => {
  const attestation = signed();
  attestation.identity.repository = 'x'.repeat(257);
  assert.throws(() => verifyAttestation(attestation, publicKey), /identity repository/);
});

test('AC-2667: digest is deterministic and length bounded @spec:AC-2667', () => {
  assert.equal(artifactDigest('fixture'), artifactDigest('fixture'));
  assert.match(artifactDigest('fixture'), /^sha256:[0-9a-f]{64}$/);
});
