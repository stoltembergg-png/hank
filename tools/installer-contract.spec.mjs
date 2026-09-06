import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, readFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { artifactDigest, canonicalRelativePath, simulateInstall, simulateUninstall, SUPPORT_MATRIX, validateInstallerManifest } from './installer-contract.mjs';
import manifest from '../docs/installer-manifest.json' with { type: 'json' };

const bytes = Buffer.from('synthetic-installer-artifact');
const identity = { repository: 'stoltembergg-png/hank', commit: 'a'.repeat(64), tree: 'b'.repeat(64) };
const valid = (target = SUPPORT_MATRIX[0]) => ({ manifest, os: target.os, arch: target.arch, bytes, digest: artifactDigest(bytes), identity });

test('AC-2671: declared OS/arch matrix is complete and bounded @spec:AC-2671', () => {
  assert.equal(validateInstallerManifest(manifest), true);
  assert.deepEqual(manifest.targets.map(({ os, arch }) => `${os}/${arch}`), ['linux/x86_64', 'windows/x86_64', 'macos/aarch64']);
});

test('AC-2672: clean install creates only declared app/profile paths @spec:AC-2672', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'hank-installer-'));
  const result = await simulateInstall({ root, artifact: valid(), expectedIdentity: identity });
  assert.equal(result.outcome, 'installed');
  assert.equal(await readFile(path.join(root, 'app/artifact.bin'), 'utf8'), bytes.toString());
  assert.equal(await readFile(path.join(root, 'profile/profile.json'), 'utf8'), '{"version":1}');
});

test('AC-2673: wrong platform or digest rejects before install @spec:AC-2673', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'hank-installer-'));
  await assert.rejects(simulateInstall({ root, artifact: { ...valid(), os: 'freebsd' } }), /artifact identity mismatch|target/);
  await assert.rejects(simulateInstall({ root, artifact: { ...valid(), digest: artifactDigest('wrong') }, expectedIdentity: identity }), /artifact identity mismatch/);
  await assert.rejects(simulateInstall({ root, artifact: { ...valid(), identity: { ...identity, commit: 'c'.repeat(64) } }, expectedIdentity: identity }), /commit identity mismatch/);
});

test('AC-2674: uninstall removes app but preserves profile @spec:AC-2674', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'hank-installer-'));
  await simulateInstall({ root, artifact: valid(), expectedIdentity: identity });
  const result = await simulateUninstall({ root });
  assert.deepEqual(result, { outcome: 'uninstalled', profilePreserved: true });
});

test('AC-2675: package paths are canonical and traversal is rejected @spec:AC-2675', () => {
  assert.equal(canonicalRelativePath('sidecars/helper.bin'), 'sidecars/helper.bin');
  for (const value of ['../escape', '/absolute', 'sidecars/../../escape', '']) assert.throws(() => canonicalRelativePath(value), /path escapes|invalid package path/);
});

test('AC-2676: unsafe metadata and embedded secrets reject manifest @spec:AC-2676', () => {
  for (const [field, value] of [['installShellFromMetadata', true], ['secretsEmbedded', true], ['signatureRequired', false], ['digestRequired', false], ['profileDeletionByDefault', true], ['migrationBeforeUse', false]]) {
    assert.throws(() => validateInstallerManifest({ ...manifest, security: { ...manifest.security, [field]: value } }), /unsafe installer policy/);
  }
  assert.throws(() => validateInstallerManifest({ ...manifest, targets: manifest.targets.map((target) => ({ ...target, sidecars: ['../secret'] })) }), /invalid sidecar path|path escapes/);
});

test('AC-2677: profile migration is required before use and remains preserved @spec:AC-2677', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'hank-installer-'));
  const profile = { 'profile.json': '{"schemaVersion":1,"migration":"required-before-use"}' };
  await simulateInstall({ root, artifact: valid(), profile, expectedIdentity: identity });
  assert.equal(await readFile(path.join(root, 'profile/profile.json'), 'utf8'), profile['profile.json']);
});
