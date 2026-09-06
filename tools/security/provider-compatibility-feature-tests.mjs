#!/usr/bin/env node
import { readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const manifestPath = resolve(root, 'docs/provider-compatibility-manifest.json');
const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
if (manifest.revision !== 'PR-265' || manifest.network !== 'forbidden' || manifest.credentials !== 'mocked') {
  throw new Error('provider compatibility manifest identity/policy mismatch');
}
if (!Array.isArray(manifest.providers) || manifest.providers.length !== 6) {
  throw new Error('provider compatibility matrix must contain six bounded entries');
}
const expected = [
  ['provider-core', 'provider_compatibility_contract', 'AC-2651'],
  ['provider-core', 'provider_compatibility_contract', 'AC-2652'],
  ['provider-adapter-openai', 'provider_contract', 'AC-2651'],
  ['provider-adapter-anthropic', 'provider_contract', 'AC-2651'],
  ['provider-adapter-gemini', 'provider_contract', 'AC-2653'],
  ['provider-adapter-openrouter', 'provider_contract', 'AC-2653'],
  ['provider-adapter-ollama', 'provider_contract', 'AC-2654'],
  ['provider-adapter-openai-compatible', 'adapter_contract', 'AC-2655'],
];
for (const [pkg, test] of expected) {
  const result = spawnSync('cargo', ['test', '-p', pkg, '--test', test, '--locked'], {
    cwd: root,
    encoding: 'utf8',
    env: {
      PATH: process.env.PATH,
      HOME: process.env.HOME,
      CARGO_HOME: process.env.CARGO_HOME,
      RUSTUP_HOME: process.env.RUSTUP_HOME,
      CARGO_TERM_COLOR: 'never',
      HANK_PROVIDER_NETWORK: 'disabled',
    },
  });
  if (result.status !== 0) {
    process.stderr.write(result.stdout || '');
    process.stderr.write(result.stderr || '');
    process.exit(result.status ?? 1);
  }
}
console.log(`1..${expected.length}`);
expected.forEach(([pkg, test, ac], index) => console.log(`ok ${index + 1} - ${pkg}/${test} @spec:${ac}`));
