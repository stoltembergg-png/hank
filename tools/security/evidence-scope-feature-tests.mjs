#!/usr/bin/env node
import fs from 'node:fs';
import { spawnSync } from 'node:child_process';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { buildReport, readManifest, assertSourceClean } from '../evidence-scope-contract.mjs';

const root = new URL('../..', import.meta.url);
const rootPath = fileURLToPath(root);
const reportPath = `${rootPath}/security/reports/evidence-scope.json`;
fs.mkdirSync(`${rootPath}/security/reports`, { recursive: true });
assertSourceClean({ projectRoot: rootPath });

const result = spawnSync(process.execPath, ['--test', '--test-reporter=tap', 'tools/evidence-scope-contract.spec.mjs'], {
  cwd: rootPath,
  encoding: 'utf8',
  env: { ...process.env, HANK_EVIDENCE_SCOPE_NETWORK: 'disabled' },
});
const tap = `${result.stdout ?? ''}${result.stderr ?? ''}`;
process.stdout.write(tap);
if (result.status !== 0) process.exit(result.status ?? 1);
assertSourceClean({ projectRoot: rootPath });

const plan = Number((tap.match(/^1\.\.(\d+)$/m) ?? [])[1]);
const passed = (tap.match(/^ok \d+ - /gm) ?? []).length;
const failed = (tap.match(/^(?:not )?ok \d+ - /gm) ?? []).filter((line) => line.startsWith('not ok')).length;
const skipped = (tap.match(/^(?:ok|not ok) \d+ - .*\b(?:SKIP|TODO)\b/gim) ?? []).length;
if (plan !== 6 || passed !== 6 || failed !== 0 || skipped !== 0) throw new Error('incomplete or skipped scope evidence');

const sourceCommit = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: rootPath, encoding: 'utf8' }).trim();
const sourceTree = execFileSync('git', ['rev-parse', 'HEAD^{tree}'], { cwd: rootPath, encoding: 'utf8' }).trim();
const report = buildReport({
  manifest: readManifest(),
  sourceCommit,
  sourceTree,
  generatedAt: new Date().toISOString(),
  testSummary: { plan: 3, passed: 3, failed: 0, skipped: 0, contractTests: { plan, passed, failed, skipped } },
});
const temporary = `${reportPath}.tmp-${process.pid}`;
fs.writeFileSync(temporary, `${JSON.stringify(report, null, 2)}\n`, { flag: 'wx' });
fs.renameSync(temporary, reportPath);
process.stdout.write(`# sourceCommit=${sourceCommit}\n# sourceTree=${sourceTree}\n`);
