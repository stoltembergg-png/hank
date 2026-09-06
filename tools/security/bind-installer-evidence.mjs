#!/usr/bin/env node
import fs from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../..', import.meta.url));
const file = `${root}/.spec/verification/installers.json`;
const evidence = JSON.parse(await fs.readFile(file, 'utf8'));
evidence.sourceCommit = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim();
evidence.sourceTree = execFileSync('git', ['rev-parse', 'HEAD^{tree}'], { cwd: root, encoding: 'utf8' }).trim();
const temporary = `${file}.tmp-${process.pid}`;
await fs.writeFile(temporary, `${JSON.stringify(evidence, null, 2)}\n`, { flag: 'wx' });
await fs.rename(temporary, file);
