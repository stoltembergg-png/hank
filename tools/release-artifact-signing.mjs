#!/usr/bin/env node
import { createHash, createPrivateKey, createPublicKey } from 'node:crypto';
import { mkdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { artifactDigest, signAttestation, verifyAttestation } from './release-signing.mjs';

const GIT_ID = /^(?:[0-9a-f]{40}|[0-9a-f]{64})$/;
const NAME = /^[A-Za-z0-9][A-Za-z0-9._-]{0,239}$/;
const IDENTITY_FIELDS = ['repository', 'event', 'ref', 'commit', 'tree', 'workflow', 'policy', 'channel', 'os'];

function requiredString(value, field) {
  if (typeof value !== 'string' || value.length === 0 || value.length > 256) throw new Error(`invalid ${field}`);
  return value;
}

function validateIdentity(identity) {
  if (!identity || typeof identity !== 'object' || Array.isArray(identity)) throw new Error('release identity is missing');
  for (const field of IDENTITY_FIELDS) requiredString(identity[field], `identity ${field}`);
  if (!GIT_ID.test(identity.commit) || !GIT_ID.test(identity.tree) || identity.commit.length !== identity.tree.length) {
    throw new Error('invalid git identity');
  }
  return Object.fromEntries(IDENTITY_FIELDS.map((field) => [field, identity[field]]));
}

function normalizeNames(names) {
  if (!Array.isArray(names) || names.length === 0) throw new Error('release artifacts are missing');
  const normalized = names.map((name) => {
    if (typeof name !== 'string' || !NAME.test(name) || path.isAbsolute(name) || name.includes('..')) {
      throw new Error(`invalid artifact name: ${name}`);
    }
    return name;
  });
  if (new Set(normalized).size !== normalized.length) throw new Error('release artifact names must be unique');
  return [...normalized].sort((left, right) => left.localeCompare(right));
}

function normalizeSignerKeyId(value) {
  const keyId = requiredString(value, 'signer key id');
  if (!/^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$/.test(keyId)) throw new Error('invalid signer key id');
  return keyId;
}

function resolvePath(directory, name, kind) {
  if (typeof directory !== 'string' || directory.length === 0) throw new Error(`${kind} directory is missing`);
  const root = path.resolve(directory);
  const candidate = path.resolve(root, name);
  if (candidate !== root && !candidate.startsWith(`${root}${path.sep}`)) throw new Error(`${kind} path escapes directory: ${name}`);
  return candidate;
}

function resolveFile(directory, name, kind) {
  const candidate = resolvePath(directory, name, kind);
  try {
    if (!statSync(candidate).isFile()) throw new Error(`${kind} is not a file: ${name}`);
  } catch (error) {
    if (error?.message?.includes(`${kind} is not a file`)) throw error;
    throw new Error(`${kind} is not a file: ${name}`);
  }
  return candidate;
}

function asPrivateKey(value) {
  const key = value?.type === 'private' ? value : createPrivateKey(value);
  if (key.asymmetricKeyType !== 'ed25519') throw new Error('release signer must use ed25519');
  return key;
}

function asPublicKey(value) {
  const key = value?.type === 'public' ? value : createPublicKey(value);
  if (key.asymmetricKeyType !== 'ed25519') throw new Error('release verifier must use ed25519');
  return key;
}

export function assertPublicKeyMatches(actual, expected) {
  const actualDer = asPublicKey(actual).export({ type: 'spki', format: 'der' }).toString('base64');
  const expectedDer = asPublicKey(expected).export({ type: 'spki', format: 'der' }).toString('base64');
  if (actualDer !== expectedDer) throw new Error('release signer public key mismatch');
  return true;
}

export function signingMetadata({ identity, signerKeyId, publicKey, artifacts }) {
  const normalizedIdentity = validateIdentity(identity);
  const keyId = normalizeSignerKeyId(signerKeyId);
  const names = normalizeNames(artifacts);
  const key = asPublicKey(publicKey);
  return {
    schemaVersion: 1,
    algorithm: 'ed25519',
    signerKeyId: keyId,
    identity: normalizedIdentity,
    artifacts: names,
    publicKeyPem: key.export({ type: 'spki', format: 'pem' }).toString(),
  };
}

export function signArtifacts({ directory, attestationDirectory, names, identity, signerKeyId, privateKey }) {
  const normalizedIdentity = validateIdentity(identity);
  const keyId = normalizeSignerKeyId(signerKeyId);
  const privateSigner = asPrivateKey(privateKey);
  const publicSigner = createPublicKey(privateSigner);
  const normalizedNames = normalizeNames(names);
  mkdirSync(attestationDirectory, { recursive: true });
  const attestations = normalizedNames.map((name) => {
    const file = resolveFile(directory, name, 'artifact');
    const bytes = readFileSync(file);
    const attestation = signAttestation({
      artifact: { name, digest: artifactDigest(bytes), size: bytes.length },
      identity: normalizedIdentity,
      signer: { keyId },
    }, privateSigner);
    const attestationPath = resolvePath(attestationDirectory, `${name}.attestation.json`, 'attestation');
    const serialized = `${JSON.stringify(attestation, null, 2)}\n`;
    writeFileSync(attestationPath, serialized, { flag: 'w' });
    return { name, path: `${name}.attestation.json`, digest: createHash('sha256').update(serialized).digest('hex') };
  });
  const metadata = signingMetadata({ identity: normalizedIdentity, signerKeyId: keyId, publicKey: publicSigner, artifacts: normalizedNames });
  writeFileSync(path.join(attestationDirectory, 'release-signing-metadata.json'), `${JSON.stringify(metadata, null, 2)}\n`, { flag: 'w' });
  return { ...metadata, attestations };
}

export function verifyArtifacts({ directory, attestationDirectory, names, identity, signerKeyId, publicKey }) {
  const normalizedIdentity = validateIdentity(identity);
  const normalizedNames = normalizeNames(names);
  const verifier = asPublicKey(publicKey);
  const keyId = normalizeSignerKeyId(signerKeyId);
  for (const name of normalizedNames) {
    const artifactPath = resolveFile(directory, name, 'artifact');
    const attestationPath = resolveFile(attestationDirectory, `${name}.attestation.json`, 'attestation');
    let attestation;
    try {
      attestation = JSON.parse(readFileSync(attestationPath, 'utf8'));
    } catch {
      throw new Error(`invalid attestation: ${name}`);
    }
    verifyAttestation(attestation, verifier, {
      artifactBytes: readFileSync(artifactPath),
      signerKeyId: keyId,
      ...normalizedIdentity,
    });
    if (attestation.artifact.name !== name) throw new Error(`attestation artifact mismatch: ${name}`);
  }
  return { verified: normalizedNames.length, artifacts: normalizedNames };
}

function parseArgs(argv) {
  const [command, ...tokens] = argv;
  const options = { command };
  for (let index = 0; index < tokens.length; index += 1) {
    const token = tokens[index];
    if (!token.startsWith('--') || index + 1 >= tokens.length) throw new Error(`invalid argument: ${token}`);
    options[token.slice(2)] = tokens[++index];
  }
  return options;
}

function cli() {
  const options = parseArgs(process.argv.slice(2));
  if (!['sign', 'verify'].includes(options.command)) throw new Error('command must be sign or verify');
  const names = String(options.artifacts ?? '').split(',').filter(Boolean);
  if (options.command === 'sign') {
    if (!process.env.HANK_RELEASE_SIGNING_PRIVATE_KEY_PEM) throw new Error('protected release signer key is unavailable');
    if (!process.env.HANK_RELEASE_SIGNING_PUBLIC_KEY_PEM) throw new Error('protected release signer public key is unavailable');
    const identity = Object.fromEntries(IDENTITY_FIELDS.map((field) => [field, options[field]]));
    const result = signArtifacts({
      directory: options.directory,
      attestationDirectory: options.attestationDirectory,
      names,
      identity,
      signerKeyId: options.signerKeyId,
      privateKey: process.env.HANK_RELEASE_SIGNING_PRIVATE_KEY_PEM,
    });
    assertPublicKeyMatches(result.publicKeyPem, process.env.HANK_RELEASE_SIGNING_PUBLIC_KEY_PEM);
    process.stdout.write(JSON.stringify({ signed: result.attestations.length, signerKeyId: result.signerKeyId }) + '\n');
    return;
  }
  const metadataPath = resolveFile(options.attestationDirectory, 'release-signing-metadata.json', 'signing metadata');
  let metadata;
  try {
    metadata = JSON.parse(readFileSync(metadataPath, 'utf8'));
  } catch {
    throw new Error('invalid signing metadata');
  }
  const identity = Object.fromEntries(IDENTITY_FIELDS.map((field) => [field, options[field] ?? metadata.identity?.[field]]));
  const trustedKey = options.requireTrustedKey === '1'
    ? (() => {
      if (!process.env.HANK_RELEASE_SIGNING_PUBLIC_KEY_PEM) throw new Error('protected release signer public key is unavailable');
      assertPublicKeyMatches(metadata.publicKeyPem, process.env.HANK_RELEASE_SIGNING_PUBLIC_KEY_PEM);
      return process.env.HANK_RELEASE_SIGNING_PUBLIC_KEY_PEM;
    })()
    : metadata.publicKeyPem;
  const result = verifyArtifacts({
    directory: options.directory,
    attestationDirectory: options.attestationDirectory,
    names,
    identity,
    signerKeyId: options.signerKeyId ?? metadata.signerKeyId,
    publicKey: trustedKey,
  });
  process.stdout.write(JSON.stringify(result) + '\n');
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  try {
    cli();
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : 'release signing failed'}\n`);
    process.exitCode = 1;
  }
}
