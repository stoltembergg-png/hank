#!/usr/bin/env node
import fs from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../..', import.meta.url));
const manifest = JSON.parse(fs.readFileSync(`${root}/docs/release-signing-manifest.json`, 'utf8'));
if (manifest.schemaVersion !== 1 || manifest.review !== 'PR-266' || manifest.algorithm !== 'ed25519') throw new Error('invalid release signing manifest');
if (manifest.network !== 'forbidden' || manifest.credentials !== 'synthetic-only') throw new Error('unsafe fixture policy');
const expected = ['AC-2661', 'AC-2662', 'AC-2663', 'AC-2664', 'AC-2665', 'AC-2666', 'AC-2667'];
const test = spawnSync(process.execPath, ['--test', 'tools/release-signing.spec.mjs'], { cwd: root, encoding: 'utf8', env: { ...process.env, HANK_RELEASE_SIGNING_NETWORK: 'disabled' } });
process.stdout.write(test.stdout || '');
process.stderr.write(test.stderr || '');
if (test.status !== 0) process.exit(test.status ?? 1);
for (const ac of expected) if (!test.stdout.includes(ac)) throw new Error(`missing TAP evidence for ${ac}`);
const status = spawnSync('git', ['status', '--porcelain=v1', '-z'], { cwd: root, encoding: 'buffer' }).stdout.toString('utf8');
const paths = [];
for (const record of status.split('\0').filter(Boolean)) {
  const statusCode = record.slice(0, 2);
  const path = record.slice(3);
  paths.push(path);
  if (statusCode[0] === 'R' || statusCode[0] === 'C') paths.push(status.split('\0')[paths.length] || '');
}
const allowed = new Set(['tools/release-signing.mjs', 'tools/release-signing.spec.mjs', 'tools/security/release-signing-feature-tests.mjs', 'docs/release-signing-manifest.json', 'docs/release-signing.md', '.spec/features/release-signing/spec.md', '.spec/features/release-signing/tasks.md', '.github/workflows/ci-release-signing.yml', '.github/workflows/onp-sdd-evidence.yml', '.spec/verification/release-signing.json', 'onpspec.config.json']);
const unexpected = paths.filter((path) => path && !allowed.has(path) && !path.startsWith('.spec/features/release-signing/') && !path.startsWith('.spec/verification/'));
if (unexpected.length) throw new Error(`unexpected changed paths: ${unexpected.join(', ')}`);
console.log(`1..${expected.length}`);
for (let i = 0; i < expected.length; i += 1) console.log(`ok ${i + 1} - ${expected[i]} release signing contract`);
