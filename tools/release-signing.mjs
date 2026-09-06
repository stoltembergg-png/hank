import { createHash, sign, verify } from 'node:crypto';

export const SCHEMA_VERSION = 1;
const HEX64 = /^[0-9a-f]{64}$/;

function requiredString(value, field) {
  if (typeof value !== 'string' || value.length === 0 || value.length > 256) {
    throw new Error(`invalid ${field}`);
  }
  return value;
}

export function artifactDigest(bytes) {
  const data = Buffer.isBuffer(bytes) ? bytes : Buffer.from(bytes);
  return `sha256:${createHash('sha256').update(data).digest('hex')}`;
}

export function canonicalPayload(attestation) {
  const fields = {
    schemaVersion: attestation.schemaVersion,
    artifact: {
      name: attestation.artifact.name,
      digest: attestation.artifact.digest,
      size: attestation.artifact.size,
    },
    identity: {
      repository: attestation.identity.repository,
      event: attestation.identity.event,
      ref: attestation.identity.ref,
      commit: attestation.identity.commit,
      tree: attestation.identity.tree,
      workflow: attestation.identity.workflow,
      policy: attestation.identity.policy,
      channel: attestation.identity.channel,
      os: attestation.identity.os,
    },
    signer: { keyId: attestation.signer.keyId },
  };
  return Buffer.from(JSON.stringify(fields));
}

export function validateAttestation(attestation, expected = {}) {
  if (!attestation || attestation.schemaVersion !== SCHEMA_VERSION) throw new Error('unsupported attestation schema');
  requiredString(attestation.artifact?.name, 'artifact name');
  const digest = requiredString(attestation.artifact?.digest, 'artifact digest');
  if (!/^sha256:[0-9a-f]{64}$/.test(digest)) throw new Error('invalid artifact digest');
  if (!Number.isSafeInteger(attestation.artifact.size) || attestation.artifact.size < 0) throw new Error('invalid artifact size');
  for (const field of ['repository', 'event', 'ref', 'commit', 'tree', 'workflow', 'policy', 'channel', 'os']) {
    requiredString(attestation.identity?.[field], `identity ${field}`);
  }
  requiredString(attestation.signer?.keyId, 'signer key id');
  if (!HEX64.test(attestation.identity.commit) || !HEX64.test(attestation.identity.tree)) throw new Error('invalid git identity');
  for (const [field, expectedValue] of Object.entries(expected)) {
    if (field === 'trustedKeyring' || field === 'artifactBytes') continue;
    const actual = field === 'signerKeyId' ? attestation.signer?.keyId : attestation.identity?.[field];
    if (expectedValue !== undefined && actual !== expectedValue) throw new Error(`${field} mismatch`);
  }
  if (attestation.signature?.algorithm !== 'ed25519' || typeof attestation.signature.value !== 'string' || attestation.signature.value.length > 512) {
    throw new Error('invalid signature envelope');
  }
  return true;
}

export function verifyAttestation(attestation, publicKey, expected = {}) {
  validateAttestation(attestation, expected);
  if (expected.trustedKeyring) {
    const trusted = expected.trustedKeyring[attestation.signer.keyId];
    if (!trusted || trusted.revoked || trusted.publicKey !== publicKey.export({ type: 'spki', format: 'der' }).toString('base64')) throw new Error('untrusted or revoked signer');
  }
  if (expected.artifactBytes !== undefined && artifactDigest(expected.artifactBytes) !== attestation.artifact.digest) throw new Error('artifact digest mismatch');
  const valid = verify(null, canonicalPayload(attestation), publicKey, Buffer.from(attestation.signature.value, 'base64'));
  if (!valid) throw new Error('signature verification failed');
  return { valid: true, artifactDigest: attestation.artifact.digest, signerKeyId: attestation.signer.keyId };
}

export function signAttestation(unsigned, privateKey) {
  const attestation = { ...unsigned, schemaVersion: SCHEMA_VERSION };
  validateAttestation({ ...attestation, signature: { algorithm: 'ed25519', value: 'placeholder' } });
  return { ...attestation, signature: { algorithm: 'ed25519', value: sign(null, canonicalPayload(attestation), privateKey).toString('base64') } };
}
