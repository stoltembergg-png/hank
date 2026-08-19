#!/usr/bin/env node
import { pathToFileURL } from 'node:url';

const titlePattern = /^(?:PR-\d{3}:|(?:build|chore|ci|docs|feat|fix|perf|refactor|revert|style|test)(?:\([^)]+\))?!?:)\s+\S/;

export function isValidPrTitle(title) {
  return typeof title === 'string' && title.length <= 100 && titlePattern.test(title.trim());
}

function main() {
  const title = process.env.PR_TITLE;
  if (!isValidPrTitle(title)) {
    console.error('invalid PR title: expected PR-###: <summary> or conventional-commit syntax');
    process.exit(1);
  }
  console.log(`PASS ${title}`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) main();
