#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const root = process.cwd();
const dir = path.join(root, 'docs/decisions');
const allowed = new Set(['proposed', 'accepted', 'superseded', 'rejected']);
const required = ['id', 'status', 'owner', 'date'];

function parse(file) {
  const raw = fs.readFileSync(file, 'utf8');
  const text = raw.replace(/\r\n/g, '\n');
  if (!text.startsWith('---\n')) throw new Error(`${file}: missing frontmatter`);
  const end = text.indexOf('\n---\n', 4);
  if (end < 0) throw new Error(`${file}: unterminated frontmatter`);
  const data = {};
  for (const line of text.slice(4, end).split('\n')) {
    const [key, ...rest] = line.split(':');
    if (key && rest.length) data[key.trim()] = rest.join(':').trim();
  }
  return { text, data };
}

export function validateAdrFiles(files, authority) {
  const errors = [];
  const ids = new Set();
  for (const file of files) {
    const { text, data } = parse(file);
    for (const key of required) if (!data[key]) errors.push(`${file}: missing ${key}`);
    if (!allowed.has(data.status)) errors.push(`${file}: invalid status ${data.status}`);
    if (ids.has(data.id)) errors.push(`${file}: duplicate id ${data.id}`);
    ids.add(data.id);
    for (const section of ['## Context', '## Decision', '## Alternatives', '## Consequences', '## Risks and threat boundary', '## Evidence', '## Rollback and supersession']) {
      if (!text.includes(section)) errors.push(`${file}: missing section ${section}`);
    }
    if (data.status === 'accepted' && !/sha:\s*`?[0-9a-f]{40}`?/i.test(text)) errors.push(`${file}: accepted ADR needs SHA evidence`);
    if (/-----BEGIN (?:RSA|OPENSSH|PGP) PRIVATE KEY-----|AKIA[0-9A-Z]{16}/.test(text)) errors.push(`${file}: possible secret`);
  }
  for (const file of authority.adrs ?? []) if (!fs.existsSync(path.join(dir, file))) errors.push(`authority missing ${file}`);
  return { status: errors.length ? 'BLOCKED' : 'PASS', errors, count: files.length };
}

function main() {
  const authority = JSON.parse(fs.readFileSync(path.join(dir, 'authority.json'), 'utf8'));
  const files = fs.readdirSync(dir).filter((name) => /^ADR-.*\.md$/.test(name)).map((name) => path.join(dir, name));
  const result = validateAdrFiles(files, authority);
  console.log(JSON.stringify(result, null, 2));
  process.exitCode = result.status === 'PASS' ? 0 : 1;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) main();
