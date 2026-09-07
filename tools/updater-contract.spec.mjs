import test from 'node:test';
import assert from 'node:assert/strict';
import { generateKeyPairSync } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdtemp, readFile, stat, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { artifactDigest, signAttestation } from './release-signing.mjs';
import { stageUpdate, validateUpdateMetadata } from './updater-contract.mjs';

const { publicKey, privateKey } = generateKeyPairSync('ed25519');
const bytes = Buffer.from('signed-update-fixture');
const keyId = 'updater-fixture-v1';
const keyring = { [keyId]: { revoked: false, publicKey: publicKey.export({ type: 'spki', format: 'der' }).toString('base64') } };
const policy = { channel: 'stable', os: 'linux', arch: 'x86_64', currentVersion: 3, minimumVersion: 3, maxBytes: 1024, now: 100, trustedKeyring: keyring };
const sourceCommit = execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
const sourceTree = execFileSync('git', ['rev-parse', 'HEAD^{tree}'], { encoding: 'utf8' }).trim();
const metadata = () => {
  const attestation = signAttestation({
    schemaVersion: 2,
    artifact: { name: 'hank.AppImage', digest: artifactDigest(bytes), size: bytes.length },
    identity: { repository: 'stoltembergg-png/hank', event: 'release', ref: 'refs/tags/v4', commit: sourceCommit, tree: sourceTree, workflow: 'release.yml', policy: 'updater-v1', channel: 'stable', os: 'linux-x86_64' },
    signer: { keyId },
    update: { version: 4, expiresAt: 200, os: 'linux', arch: 'x86_64' },
  }, privateKey);
  return { schemaVersion: 1, version: 4, channel: 'stable', os: 'linux', arch: 'x86_64', size: bytes.length, expiresAt: 200, bytes, attestation };
};

test('AC-2681: valid signed metadata stages an update @spec:AC-2681', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'hank-updater-'));
  const result = await stageUpdate({ root, metadata: metadata(), policy, publicKey, consent: true });
  assert.equal(result.outcome, 'staged');
  assert.equal(await readFile(path.join(root, 'staging/artifact.bin'), 'utf8'), bytes.toString());
});

test('AC-2682: invalid signature or digest blocks staging @spec:AC-2682', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'hank-updater-'));
  const update = metadata(); update.bytes = Buffer.from('substituted'); update.size = update.bytes.length;
  await assert.rejects(stageUpdate({ root, metadata: update, policy, publicKey, consent: true }), /artifact digest mismatch/);
});

test('AC-2683: wrong channel or platform blocks staging @spec:AC-2683', async () => {
  for (const field of ['channel', 'os', 'arch']) {
    const root = await mkdtemp(path.join(os.tmpdir(), 'hank-updater-'));
    const update = metadata(); update[field] = field === 'channel' ? 'nightly' : 'windows';
    await assert.rejects(stageUpdate({ root, metadata: update, policy, publicKey, consent: true }), /policy mismatch/);
  }
});

test('AC-2684: downgrade and minimum version are rejected @spec:AC-2684', () => {
  for (const version of [2, 3]) assert.throws(() => validateUpdateMetadata({ ...metadata(), version }, policy, publicKey), /downgrade or minimum/);
});

test('AC-2685: expiry and size limits are rejected @spec:AC-2685', () => {
  assert.throws(() => validateUpdateMetadata({ ...metadata(), expiresAt: 100 }, policy, publicKey), /expired/);
  assert.throws(() => validateUpdateMetadata({ ...metadata(), size: 1025 }, policy, publicKey), /size rejected/);
});

test('AC-2686: explicit consent is required and profile is untouched @spec:AC-2686', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'hank-updater-'));
  await writeFile(path.join(root, 'profile.json'), '{"preserve":true}');
  await assert.rejects(stageUpdate({ root, metadata: metadata(), policy, publicKey }), /explicit consent/);
  assert.equal(await readFile(path.join(root, 'profile.json'), 'utf8'), '{"preserve":true}');
});

test('AC-2687: interrupted or failed staging leaves no partial artifact @spec:AC-2687', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'hank-updater-'));
  await assert.rejects(stageUpdate({ root, metadata: metadata(), policy, publicKey, consent: true, failAfterWrite: true }), /injected post-write failure/);
  await assert.rejects(stat(path.join(root, 'staging')));
});
