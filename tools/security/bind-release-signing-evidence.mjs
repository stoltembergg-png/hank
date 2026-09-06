#!/usr/bin/env node
import fs from 'node:fs';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../..', import.meta.url));
const evidencePath = `${root}/.spec/verification/release-signing.json`;
const evidence = JSON.parse(fs.readFileSync(evidencePath, 'utf8'));
evidence.sourceCommit = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim();
evidence.sourceTree = execFileSync('git', ['rev-parse', 'HEAD^{tree}'], { cwd: root, encoding: 'utf8' }).trim();
fs.writeFileSync(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
