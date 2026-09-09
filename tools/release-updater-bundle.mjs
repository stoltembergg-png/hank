import { readFileSync, writeFileSync } from 'node:fs';
import { createPrivateKey, createPublicKey, KeyObject } from 'node:crypto';
import { pathToFileURL } from 'node:url';
import { artifactDigest, signAttestation, verifyAttestation } from './release-signing.mjs';

const VERSION_RE = /^v?(\d+)\.(\d+)\.(\d+)(?:[-+].*)?$/;
const PLATFORMS = new Set(['linux', 'windows', 'macos']);
const ARCHES = new Set(['x86_64', 'aarch64']);

export function versionCode(version) {
  if (typeof version === 'number') {
    if (!Number.isSafeInteger(version) || version < 0) throw new Error('version must be semver');
    return version;
  }
  if (typeof version !== 'string') throw new Error('version must be semver');
  const match = VERSION_RE.exec(version.trim());
  if (!match) throw new Error('version must be semver');
  const parts = match.slice(1).map(Number);
  const code = parts[0] * 1_000_000 + parts[1] * 1_000 + parts[2];
  if (!Number.isSafeInteger(code) || code < 0) throw new Error('version must be semver');
  return code;
}

function boundedExpiry(value) {
  if (!Number.isSafeInteger(value) || value <= 0) throw new Error('expiresAt must be a positive integer');
  return value;
}

function assertPlatform(os, arch) {
  if (!PLATFORMS.has(os)) throw new Error('unsupported updater platform');
  if (!ARCHES.has(arch)) throw new Error('unsupported updater architecture');
}

function readJson(path, label) {
  try {
    return JSON.parse(readFileSync(path, 'utf8'));
  } catch (error) {
    throw new Error(`invalid ${label}: ${error.message}`);
  }
}

function publicKeyPem(publicKey) {
  const key = publicKey instanceof KeyObject ? publicKey : createPublicKey(publicKey);
  return key.export({ type: 'spki', format: 'pem' }).toString();
}

function normalizedArtifact(bytes, name) {
  return { name, digest: artifactDigest(bytes), size: bytes.length };
}

export function createUpdaterBundle({
  artifactPath,
  attestationPath,
  outputPath,
  version,
  nextVersion,
  expiresAt,
  os,
  arch,
  privateKey,
  publicKey,
}) {
  if (!artifactPath || !attestationPath || !outputPath) throw new Error('artifact, attestation, and output paths are required');
  const bytes = readFileSync(artifactPath);
  const base = readJson(attestationPath, 'attestation');
  const firstVersion = versionCode(version);
  const secondVersion = versionCode(nextVersion);
  if (secondVersion <= firstVersion) throw new Error('next updater version must increase');
  const expiry = boundedExpiry(expiresAt);
  assertPlatform(os, arch);
  if (!base.artifact?.name || !base.identity || !base.signer) throw new Error('base attestation is incomplete');
  const artifact = normalizedArtifact(bytes, base.artifact.name);
  if (base.artifact.digest !== artifact.digest || base.artifact.size !== artifact.size) {
    throw new Error('base attestation artifact mismatch');
  }
  const signingKey = privateKey instanceof KeyObject ? privateKey : createPrivateKey(privateKey);
  const updates = [firstVersion, secondVersion].map((updateVersion) => ({
    version: updateVersion,
    expiresAt: expiry,
    os,
    arch,
    attestation: signAttestation({
      schemaVersion: 2,
      artifact,
      identity: base.identity,
      signer: base.signer,
      update: { version: updateVersion, expiresAt: expiry, os, arch },
    }, signingKey),
  }));
  const resolvedPublicKey = publicKey ? (publicKey instanceof KeyObject ? publicKey : createPublicKey(publicKey)) : createPublicKey(signingKey);
  const signingPublicDer = createPublicKey(signingKey).export({ type: 'spki', format: 'der' }).toString('base64');
  const resolvedPublicDer = resolvedPublicKey.export({ type: 'spki', format: 'der' }).toString('base64');
  if (signingPublicDer !== resolvedPublicDer) throw new Error('release signer public key mismatch');
  const bundle = {
    schemaVersion: 1,
    artifact,
    identity: base.identity,
    signer: base.signer,
    publicKeyPem: publicKeyPem(resolvedPublicKey),
    updates,
  };
  writeFileSync(outputPath, `${JSON.stringify(bundle)}\n`, { flag: 'w' });
  return bundle;
}

export function verifyUpdaterBundle({ bundlePath, artifactPath, publicKey }) {
  if (!bundlePath || !artifactPath) throw new Error('bundle and artifact paths are required');
  const bundle = readJson(bundlePath, 'updater bundle');
  if (bundle.schemaVersion !== 1 || !Array.isArray(bundle.updates) || bundle.updates.length < 2) {
    throw new Error('invalid updater bundle');
  }
  const bytes = readFileSync(artifactPath);
  const digest = artifactDigest(bytes);
  if (bundle.artifact?.digest !== digest || bundle.artifact?.size !== bytes.length) throw new Error('artifact digest mismatch');
  if (!bundle.artifact?.name || !bundle.identity || !bundle.signer) throw new Error('invalid updater bundle identity');
  const verificationKey = publicKey
    ? (publicKey instanceof KeyObject ? publicKey : createPublicKey(publicKey))
    : createPublicKey(bundle.publicKeyPem);
  let previous = -1;
  for (const entry of bundle.updates) {
    if (!Number.isSafeInteger(entry.version) || entry.version <= previous || entry.expiresAt <= 0 || !PLATFORMS.has(entry.os) || !ARCHES.has(entry.arch)) {
      throw new Error('invalid updater binding');
    }
    previous = entry.version;
    const update = entry.attestation?.update;
    if (!update || update.version !== entry.version || update.expiresAt !== entry.expiresAt || update.os !== entry.os || update.arch !== entry.arch) {
      throw new Error('updater binding mismatch');
    }
    if (entry.attestation.artifact?.digest !== digest || entry.attestation.artifact?.size !== bytes.length) throw new Error('artifact digest mismatch');
    verifyAttestation(entry.attestation, verificationKey, { artifactBytes: bytes });
  }
  return { verified: bundle.updates.length, artifact: bundle.artifact.name, digest };
}

function argValue(args, name) {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : undefined;
}

function cli() {
  const [command, ...args] = process.argv.slice(2);
  if (command === 'version-code') {
    console.log(versionCode(argValue(args, '--version')));
    return;
  }
  if (command === 'create') {
    const privatePem = process.env.HANK_RELEASE_SIGNING_PRIVATE_KEY_PEM;
    if (!privatePem) throw new Error('release signing key is unavailable');
    const publicPem = process.env.HANK_RELEASE_SIGNING_PUBLIC_KEY_PEM;
    createUpdaterBundle({
      artifactPath: argValue(args, '--artifact'), attestationPath: argValue(args, '--attestation'), outputPath: argValue(args, '--output'),
      version: argValue(args, '--version'), nextVersion: argValue(args, '--next-version'), expiresAt: Number(argValue(args, '--expires-at')),
      os: argValue(args, '--os'), arch: argValue(args, '--arch'), privateKey: privatePem, publicKey: publicPem,
    });
    return;
  }
  if (command === 'verify') {
    const publicPem = process.env.HANK_RELEASE_SIGNING_PUBLIC_KEY_PEM;
    console.log(JSON.stringify(verifyUpdaterBundle({ bundlePath: argValue(args, '--bundle'), artifactPath: argValue(args, '--artifact'), publicKey: publicPem })));
    return;
  }
  throw new Error('usage: version-code|create|verify');
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try { cli(); } catch (error) { console.error(error.message); process.exitCode = 1; }
}
