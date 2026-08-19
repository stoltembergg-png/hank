#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

const policy = JSON.parse(readFileSync(new URL('../.commitlintrc.json', import.meta.url), 'utf8'));
const typePattern = policy.types.join('|');
const header = new RegExp(`^(?:${typePattern})(?:\\([a-z0-9][a-z0-9/_-]*\\))?!?: .+\\S$`);

export function isConventionalCommit(subject) {
  if (/^Merge /.test(subject)) return true;
  if (subject.length > policy.subjectMaxLength) return false;
  return header.test(subject);
}

export function validateMessages(messages) {
  return messages
    .filter((message) => message.trim())
    .map((subject) => ({ subject, valid: isConventionalCommit(subject) }));
}

function main() {
  const rangeIndex = process.argv.indexOf('--range');
  const range = rangeIndex >= 0 ? process.argv[rangeIndex + 1] : 'origin/main...HEAD';
  if (!range || range.startsWith('-')) {
    console.error('usage: node tools/commit-message-lint.mjs [--range <git-range>]');
    process.exit(2);
  }
  let output;
  try {
    output = execFileSync('git', ['log', '--format=%s', range], { encoding: 'utf8' });
  } catch (error) {
    console.error(`cannot read commit range: ${range}`);
    process.exit(error.status || 2);
  }
  const results = validateMessages(output.split('\n'));
  const invalid = results.filter((result) => !result.valid);
  for (const result of results) console.log(`${result.valid ? 'PASS' : 'FAIL'} ${result.subject}`);
  if (invalid.length) process.exit(1);
  console.log(`Conventional commit policy: ${results.length}/${results.length} PASS`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) main();
