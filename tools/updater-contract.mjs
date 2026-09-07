import { mkdir, rm, stat, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { verifyAttestation } from './release-signing.mjs';

export function validateUpdateMetadata(metadata, policy, publicKey) {
  if (!metadata || metadata.schemaVersion !== 1 || metadata.channel !== policy.channel || metadata.os !== policy.os || metadata.arch !== policy.arch) throw new Error('update metadata policy mismatch');
  if (metadata.version <= policy.currentVersion || metadata.version < policy.minimumVersion) throw new Error('downgrade or minimum version rejected');
  if (!Number.isSafeInteger(metadata.size) || metadata.size < 0 || metadata.size > policy.maxBytes) throw new Error('update size rejected');
  if (!Number.isSafeInteger(metadata.expiresAt) || metadata.expiresAt <= policy.now) throw new Error('update metadata expired');
  if (!Buffer.isBuffer(metadata.bytes) || metadata.bytes.length !== metadata.size || metadata.bytes.length > policy.maxBytes) throw new Error('update byte size rejected');
  verifyAttestation(metadata.attestation, publicKey, { channel: metadata.channel, artifactBytes: metadata.bytes, trustedKeyring: policy.trustedKeyring });
  const signedUpdate = metadata.attestation.update;
  if (!signedUpdate || signedUpdate.version !== metadata.version || signedUpdate.expiresAt !== metadata.expiresAt || signedUpdate.os !== metadata.os || signedUpdate.arch !== metadata.arch) throw new Error('unsigned update policy mismatch');
  return true;
}

export async function stageUpdate({ root, metadata, policy, publicKey, consent = false, failAfterWrite = false }) {
  if (!consent) throw new Error('explicit consent required');
  validateUpdateMetadata(metadata, policy, publicKey);
  const stage = path.join(root, 'staging');
  await rm(stage, { recursive: true, force: true });
  try {
    await mkdir(stage, { recursive: true });
    await writeFile(path.join(stage, 'artifact.bin'), metadata.bytes, { flag: 'wx' });
    if (failAfterWrite) throw new Error('injected post-write failure');
    const staged = await stat(path.join(stage, 'artifact.bin'));
    if (staged.size !== metadata.size) throw new Error('staged size mismatch');
  } catch (error) {
    await rm(stage, { recursive: true, force: true });
    throw error;
  }
  return { outcome: 'staged', version: metadata.version, channel: metadata.channel, digest: metadata.attestation.artifact.digest, currentVersion: policy.currentVersion };
}
