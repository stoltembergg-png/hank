#!/usr/bin/env node
import fs from 'node:fs';
import { execFileSync } from 'node:child_process';
import { assertSourceClean, renderSvg } from '../evidence-scope-contract.mjs';

const root = new URL('../..', import.meta.url).pathname;
assertSourceClean({ projectRoot: root });
const reportPath = `${root}/security/reports/evidence-scope.json`;
const report = JSON.parse(fs.readFileSync(reportPath, 'utf8'));
const sourceCommit = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim();
const sourceTree = execFileSync('git', ['rev-parse', 'HEAD^{tree}'], { cwd: root, encoding: 'utf8' }).trim();
if (report.sourceCommit !== sourceCommit || report.sourceTree !== sourceTree) throw new Error('visual evidence source identity is stale');

const svg = renderSvg(report);
fs.writeFileSync(`${root}/security/reports/evidence-scope.svg`, `${svg}\n`);
const html = `<!doctype html>
<meta charset="utf-8">
<title>Hank contract evidence scope</title>
<style>body{margin:0;background:#0b1220}img{display:block;max-width:100%;height:auto}</style>
<img alt="Hank contract evidence scope — not production proof" src="evidence-scope.svg">
`;
fs.writeFileSync(`${root}/security/reports/evidence-scope.html`, html);
