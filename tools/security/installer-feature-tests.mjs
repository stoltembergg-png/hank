#!/usr/bin/env node
import fs from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../..', import.meta.url));
const manifest = JSON.parse(fs.readFileSync(`${root}/docs/installer-manifest.json`, 'utf8'));
if (manifest.schemaVersion !== 1 || manifest.review !== 'PR-267' || manifest.publication !== 'forbidden-in-PR-267') throw new Error('invalid installer manifest');
if (manifest.security.installShellFromMetadata !== false || manifest.security.secretsEmbedded !== false || manifest.security.profileDeletionByDefault !== false) throw new Error('unsafe installer policy');
const status = spawnSync('git', ['status', '--porcelain=v1', '-z'], { cwd: root, encoding: 'buffer' });
if (status.error || status.status !== 0) throw new Error(`git status failed: ${status.stderr?.toString() || status.error?.message || status.status}`);
const paths = status.stdout.toString('utf8').split('\0').filter(Boolean).map((record) => record.slice(3));
const unexpected = paths.filter((path) => path && !path.startsWith('.spec/verification/') && !path.startsWith('security/reports/'));
if (unexpected.length) throw new Error(`unexpected dirty paths: ${unexpected.join(', ')}`);
const expected = ['AC-2671', 'AC-2672', 'AC-2673', 'AC-2674', 'AC-2675', 'AC-2676', 'AC-2677'];
const result = spawnSync(process.execPath, ['--test', '--test-reporter=tap', 'tools/installer-contract.spec.mjs'], { cwd: root, encoding: 'utf8', env: { ...process.env, HANK_INSTALLER_NETWORK: 'disabled' } });
if (result.status !== 0) { process.stdout.write(result.stdout || ''); process.stderr.write(result.stderr || ''); process.exit(result.status ?? 1); }
const tap = result.stdout || '';
if ((tap.match(/^1\.\.\d+$/gm) || []).join() !== `1..${expected.length}`) throw new Error('TAP must contain exactly one complete plan');
for (let index = 0; index < expected.length; index += 1) if (!new RegExp(`^ok ${index + 1} - ${expected[index]}: .*@spec:${expected[index]}$`, 'm').test(tap)) throw new Error(`missing non-skipped TAP result for ${expected[index]}`);
if (/^\s*(not )?ok .*\b(SKIP|TODO)\b/im.test(tap)) throw new Error('skipped or todo evidence is not proof');
process.stdout.write(tap);
