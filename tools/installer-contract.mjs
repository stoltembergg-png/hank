import { createHash } from 'node:crypto';
import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import path from 'node:path';

export const SCHEMA_VERSION = 1;
export const SUPPORT_MATRIX = [
  { os: 'linux', arch: 'x86_64', format: 'appimage', profilePolicy: 'preserve' },
  { os: 'windows', arch: 'x86_64', format: 'nsis', profilePolicy: 'preserve' },
  { os: 'macos', arch: 'aarch64', format: 'dmg', profilePolicy: 'preserve' },
];

export function artifactDigest(bytes) {
  return `sha256:${createHash('sha256').update(bytes).digest('hex')}`;
}

export function canonicalRelativePath(value) {
  if (typeof value !== 'string' || value.length === 0 || value.length > 240) throw new Error('invalid package path');
  const normalized = path.posix.normalize(value.replaceAll('\\', '/'));
  if (normalized.startsWith('/') || normalized === '..' || normalized.startsWith('../') || normalized.includes('\0')) throw new Error('path escapes package root');
  return normalized;
}

export function validateInstallerManifest(manifest) {
  if (manifest?.schemaVersion !== SCHEMA_VERSION || manifest.review !== 'PR-267') throw new Error('invalid installer manifest');
  if (!Array.isArray(manifest.targets) || manifest.targets.length !== SUPPORT_MATRIX.length) throw new Error('incomplete support matrix');
  for (const expected of SUPPORT_MATRIX) {
    const actual = manifest.targets.find((target) => target.os === expected.os && target.arch === expected.arch);
    if (!actual || actual.format !== expected.format || actual.profilePolicy !== 'preserve') throw new Error(`missing target ${expected.os}/${expected.arch}`);
    if (!Array.isArray(actual.sidecars) || actual.sidecars.some((sidecar) => canonicalRelativePath(sidecar) !== sidecar)) throw new Error('invalid sidecar path');
  }
  if (manifest.security?.installShellFromMetadata !== false || manifest.security?.secretsEmbedded !== false) throw new Error('unsafe installer policy');
  return true;
}

export async function simulateInstall({ root, artifact, profile = { 'profile.json': '{"version":1}' } }) {
  validateInstallerManifest(artifact.manifest);
  const target = artifact.manifest.targets.find((entry) => entry.os === artifact.os && entry.arch === artifact.arch);
  if (!target || artifact.digest !== artifactDigest(artifact.bytes)) throw new Error('artifact identity mismatch');
  await mkdir(path.join(root, 'app'), { recursive: true });
  await writeFile(path.join(root, 'app', 'artifact.bin'), artifact.bytes);
  await mkdir(path.join(root, 'profile'), { recursive: true });
  for (const [relative, content] of Object.entries(profile)) await writeFile(path.join(root, 'profile', canonicalRelativePath(relative)), content);
  return { outcome: 'installed', os: artifact.os, arch: artifact.arch, digest: artifact.digest, profilePreserved: true };
}

export async function simulateUninstall({ root }) {
  await rm(path.join(root, 'app'), { recursive: true, force: true });
  const profileExists = await readFile(path.join(root, 'profile', 'profile.json'), 'utf8').then(() => true, () => false);
  return { outcome: 'uninstalled', profilePreserved: profileExists };
}
