import { createPublicKey } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { verifyUpdaterBundle } from '../tools/release-updater-bundle.mjs';
import { assertPublicKeyMatches } from '../tools/release-artifact-signing.mjs';

function option(name) {
  const index = process.argv.indexOf(name);
  if (index < 0 || !process.argv[index + 1]) throw new Error(`${name} is required`);
  return process.argv[index + 1];
}

const bundlePath = option('--bundle');
const artifactPath = option('--artifact');
const signingMetadataPath = option('--signing-metadata');
const bundle = JSON.parse(readFileSync(bundlePath, 'utf8'));
const metadata = JSON.parse(readFileSync(signingMetadataPath, 'utf8'));
if (!Array.isArray(bundle.updates) || bundle.updates.length < 2) throw new Error('protected updater bundle must contain two updates');
assertPublicKeyMatches(bundle.publicKeyPem, metadata.publicKeyPem);
if (bundle.signer?.keyId !== metadata.signerKeyId) throw new Error('protected updater signer key mismatch');
for (const field of ['repository', 'event', 'ref', 'commit', 'tree', 'workflow', 'policy', 'channel', 'os']) {
  if (bundle.identity?.[field] !== metadata.identity?.[field]) throw new Error(`protected updater identity mismatch: ${field}`);
}
const verification = verifyUpdaterBundle({ bundlePath, artifactPath, publicKey: metadata.publicKeyPem });
const publicKeyDerB64 = createPublicKey(metadata.publicKeyPem).export({ type: 'spki', format: 'der' }).toString('base64');
const first = bundle.updates[0];
const identity = bundle.identity;
const environment = {
  HANK_UPDATER_PUBLIC_KEY_DER_B64: publicKeyDerB64,
  HANK_UPDATER_CURRENT_VERSION: String(first.version - 1),
  HANK_UPDATER_EVENT: identity.event,
  HANK_UPDATER_WORKFLOW: identity.workflow,
  HANK_UPDATER_POLICY: identity.policy,
  HANK_UPDATER_KEY_ID: bundle.signer.keyId,
  HANK_UPDATER_CHANNEL: identity.channel,
  HANK_UPDATER_OS: first.os,
  HANK_UPDATER_ARCH: first.arch,
  HANK_UPDATER_REPOSITORY: identity.repository,
  HANK_UPDATER_REF: identity.ref,
  HANK_UPDATER_COMMIT: identity.commit,
  HANK_UPDATER_TREE: identity.tree,
  HANK_UPDATER_RELEASE_SCOPE: 'protected-release-signed-artifact',
  HANK_UPDATER_VERIFIED_DIGEST: verification.digest,
};

if (process.argv.includes('--shell')) {
  for (const [key, value] of Object.entries(environment)) process.stdout.write(`${key}=${value}\n`);
} else {
  process.stdout.write(`${JSON.stringify({ ...environment, verified: verification })}\n`);
}
