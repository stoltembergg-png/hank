#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import { writeFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

const categories = {
  feat: 'Added',
  fix: 'Fixed',
  perf: 'Performance',
  refactor: 'Changed',
  docs: 'Documentation',
  ci: 'Infrastructure',
  build: 'Infrastructure',
  test: 'Tests',
  chore: 'Maintenance',
  revert: 'Reverted',
  style: 'Maintenance',
};

export function parseCommit(line) {
  const [sha, subject = ''] = line.split('\t');
  const match = subject.match(/^([a-z]+)(?:\(([^)]+)\))?(!)?:\s+(.+)$/i);
  const type = match?.[1]?.toLowerCase() || 'other';
  return { sha, subject, type, scope: match?.[2] || null, breaking: Boolean(match?.[3]) };
}

export function renderChangelog({ range, tip, commits }) {
  const grouped = new Map();
  for (const commit of commits) {
    const category = categories[commit.type] || 'Other';
    if (!grouped.has(category)) grouped.set(category, []);
    grouped.get(category).push(commit);
  }
  const lines = [
    '# Changelog proposal',
    '',
    `<!-- source-range: ${range} -->`,
    `<!-- source-tip: ${tip} -->`,
    '',
  ];
  for (const category of [...grouped.keys()].sort()) {
    lines.push(`## ${category}`, '');
    for (const commit of grouped.get(category).sort((a, b) => a.sha.localeCompare(b.sha))) {
      const scope = commit.scope ? `**${commit.scope}:** ` : '';
      const marker = commit.breaking ? ' **BREAKING**' : '';
      lines.push(`- ${scope}${commit.subject.replace(/^\S+?(?:\([^)]*\))?!?:\s+/, '')}${marker} ([${commit.sha.slice(0, 7)}])`);
    }
    lines.push('');
  }
  if (commits.length === 0) lines.push('No conventional commits in range.', '');
  return `${lines.join('\n').trimEnd()}\n`;
}

function main() {
  const rangeIndex = process.argv.indexOf('--range');
  const outputIndex = process.argv.indexOf('--output');
  const range = rangeIndex >= 0 ? process.argv[rangeIndex + 1] : 'origin/main...HEAD';
  const output = outputIndex >= 0 ? process.argv[outputIndex + 1] : null;
  if (!range || range.startsWith('-')) throw new Error('usage: --range <git-range> [--output <path>]');
  const lines = execFileSync('git', ['log', '--format=%H%x09%s', range], { encoding: 'utf8' })
    .split('\n').filter(Boolean);
  const tip = execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
  const result = renderChangelog({ range, tip, commits: lines.map(parseCommit) });
  if (output) writeFileSync(output, result);
  else process.stdout.write(result);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) main();
