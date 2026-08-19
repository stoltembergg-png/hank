import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { validateAdrFiles } from './adr-lint.mjs';

const root = process.cwd();
const authority = JSON.parse(fs.readFileSync('docs/decisions/authority.json', 'utf8'));

test('validates the repository ADR template and authority manifest', () => {
  const result = validateAdrFiles([path.resolve('docs/decisions/ADR-TEMPLATE.md')], authority);
  assert.equal(result.status, 'PASS', result.errors.join('; '));
});

test('blocks missing fields, broken structure and accepted ADR without evidence', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'adr-'));
  const file = path.join(dir, 'ADR-001.md');
  fs.writeFileSync(file, '---\nid: ADR-001\nstatus: accepted\n---\n\n## Context\n');
  const result = validateAdrFiles([file], { adrs: [] });
  assert.equal(result.status, 'BLOCKED');
  assert.ok(result.errors.some((error) => error.includes('missing owner')));
  assert.ok(result.errors.some((error) => error.includes('accepted ADR needs SHA')));
});
