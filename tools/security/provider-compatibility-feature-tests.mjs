#!/usr/bin/env node
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const manifestPath = resolve(root, 'docs/provider-compatibility-manifest.json');
const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
const expectedProviders = [
  { id: 'openai', fixture: 'openai-complete-stream-tool.json', support: 'supported', features: ['complete', 'stream', 'tool_use', 'usage_cost'] },
  { id: 'anthropic', fixture: 'anthropic-complete-stream-tool.json', support: 'supported', features: ['complete', 'stream', 'tool_use', 'usage_cost'] },
  { id: 'gemini', fixture: 'gemini-error-capability.json', support: 'bounded', features: ['complete', 'error', 'capability_state'] },
  { id: 'openrouter', fixture: 'openrouter-fallback.json', support: 'bounded', features: ['fallback', 'rate_limit', 'redaction'] },
  { id: 'ollama', fixture: 'ollama-local-model.json', support: 'bounded', features: ['complete', 'offline_fixture', 'capability_state'] },
  { id: 'openai-compatible', fixture: 'openai-compatible-negative.json', support: 'expected-fail', features: ['invalid_request', 'unsupported_capability'] },
];
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

function git(args) {
  const result = spawnSync('git', args, { cwd: root, encoding: 'utf8' });
  if (result.status !== 0) throw new Error(`git ${args.join(' ')} failed`);
  return result.stdout.trim();
}
function canonical(value) {
  return JSON.stringify(value, Object.keys(value).sort());
}
function gitStatus() {
  const result = spawnSync('git', ['status', '--porcelain=v1', '-z'], { cwd: root, encoding: 'utf8' });
  if (result.status !== 0) throw new Error('git status failed');
  return result.stdout;
}
function allowedGeneratedPath(path) {
  return path.startsWith('.spec/verification/') || path.startsWith('security/reports/');
}
function fail(message) {
  process.stderr.write(`${message}\n`);
  process.stdout.write(`1..${expected.length}\n`);
  expected.forEach(([pkg, test, ac], index) => console.log(`not ok ${index + 1} - ${pkg}/${test} @spec:${ac}`));
  process.exitCode = 1;
}

try {
  if (manifest.revision !== 'PR-265' || manifest.schema_version !== 1 || manifest.network !== 'forbidden' || manifest.credentials !== 'mocked') {
    throw new Error('provider compatibility manifest identity/policy mismatch');
  }
  if (canonical(manifest.providers) !== canonical(expectedProviders)) {
    throw new Error('provider compatibility manifest entries diverge from canonical matrix');
  }
  const records = gitStatus().split('\0').filter(Boolean);
  const paths = [];
  for (let index = 0; index < records.length; index += 1) {
    const record = records[index];
    const status = record.slice(0, 2);
    paths.push(record.slice(3));
    if (status.includes('R') || status.includes('C')) {
      const originalPath = records[++index];
      if (originalPath === undefined) throw new Error('incomplete rename/copy record');
      paths.push(originalPath);
    }
  }
  const unexpected = paths.filter((path) => !allowedGeneratedPath(path));
  if (unexpected.length) throw new Error(`provider compatibility runner rejects unexpected paths: ${unexpected.join(', ')}`);
  const headSha = git(['rev-parse', 'HEAD']);
  const treeSha = git(['rev-parse', 'HEAD^{tree}']);
  const sourceDigest = createHash('sha256')
    .update(readFileSync(manifestPath))
    .update(readFileSync(resolve(root, 'crates/provider-core/tests/provider_compatibility_contract.rs')))
    .digest('hex');

  const results = [];
  for (const [pkg, test, ac] of expected) {
    const result = spawnSync('cargo', ['test', '-p', pkg, '--test', test, '--locked', '--offline'], {
      cwd: root,
      encoding: 'utf8',
      env: { PATH: process.env.PATH, HOME: process.env.HOME, CARGO_HOME: process.env.CARGO_HOME, RUSTUP_HOME: process.env.RUSTUP_HOME, CARGO_TERM_COLOR: 'never', HANK_PROVIDER_NETWORK: 'disabled' },
    });
    results.push({ pkg, test, ac, pass: result.status === 0 });
    if (result.status !== 0) {
      process.stderr.write(result.stdout || '');
      process.stderr.write(result.stderr || '');
    }
  }
  process.stdout.write(`1..${expected.length}\n`);
  results.forEach(({ pkg, test, ac, pass }, index) => console.log(`${pass ? 'ok' : 'not ok'} ${index + 1} - ${pkg}/${test} @spec:${ac}`));
  console.log(`# head_sha=${headSha}`);
  console.log(`# tree_sha=${treeSha}`);
  console.log(`# source_digest=sha256:${sourceDigest}`);
  if (results.some(({ pass }) => !pass)) process.exitCode = 1;
} catch (error) {
  fail(error instanceof Error ? error.message : String(error));
}
