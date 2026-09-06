#!/usr/bin/env node
import fs from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../..', import.meta.url));
const manifest = JSON.parse(fs.readFileSync(`${root}/docs/release-signing-manifest.json`, 'utf8'));
if (manifest.schemaVersion !== 1 || manifest.review !== 'PR-266' || manifest.algorithm !== 'ed25519') throw new Error('invalid release signing manifest');
if (manifest.network !== 'forbidden' || manifest.credentials !== 'synthetic-only') throw new Error('unsafe fixture policy');
const status = spawnSync('git', ['status', '--porcelain=v1', '-z'], { cwd: root, encoding: 'buffer' }).stdout.toString('utf8');
const records = status.split('\0').filter(Boolean);
const paths = [];
for (let index = 0; index < records.length; index += 1) {
  const record = records[index];
  if (record.length < 4) throw new Error('malformed git status record');
  const code = record.slice(0, 2);
  paths.push(record.slice(3));
  if (code[0] === 'R' || code[0] === 'C') {
    const original = records[++index];
    if (!original) throw new Error('incomplete rename/copy status record');
    paths.push(original);
  }
}
const unexpected = paths.filter((path) => path && !path.startsWith('.spec/verification/') && !path.startsWith('security/reports/'));
if (unexpected.length) throw new Error(`unexpected dirty paths: ${unexpected.join(', ')}`);
const expected = ['AC-2661', 'AC-2662', 'AC-2663', 'AC-2664', 'AC-2665', 'AC-2666', 'AC-2667'];
const result = spawnSync(process.execPath, ['--test', 'tools/release-signing.spec.mjs'], { cwd: root, encoding: 'utf8', env: { ...process.env, HANK_RELEASE_SIGNING_NETWORK: 'disabled' } });
if (result.status !== 0) { process.stdout.write(result.stdout || ''); process.stderr.write(result.stderr || ''); process.exit(result.status ?? 1); }
const tap = result.stdout || '';
const plans = tap.match(/^1\.\.\d+$/gm) || [];
if (plans.length !== 1 || plans[0] !== `1..${expected.length}`) throw new Error('TAP must contain exactly one complete plan');
for (let index = 0; index < expected.length; index += 1) {
  const line = new RegExp(`^ok ${index + 1} - ${expected[index]}: .*@spec:${expected[index]}$`, 'm');
  if (!line.test(tap)) throw new Error(`missing non-skipped TAP result for ${expected[index]}`);
}
if (/^\s*(not )?ok .*\b(SKIP|TODO)\b/im.test(tap)) throw new Error('skipped or todo evidence is not proof');
process.stdout.write(tap);
