import { generateKeyPairSync } from 'node:crypto';

// The key is generated per runner invocation. The private half is returned
// only to the parent process so the E2E can sign an in-memory fixture; it is
// never written to the workspace or diagnostic artifacts.
const { publicKey, privateKey } = generateKeyPairSync('ed25519');
process.stdout.write(JSON.stringify({
  publicKeyDerB64: publicKey.export({ type: 'spki', format: 'der' }).toString('base64'),
  privateKeyDerB64: privateKey.export({ type: 'pkcs8', format: 'der' }).toString('base64'),
}));
