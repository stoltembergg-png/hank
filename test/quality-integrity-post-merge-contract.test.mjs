import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

test('quality integrity runs on integrated main pushes @spec:AC-302', () => {
  const workflow = readFileSync('.github/workflows/quality-integrity.yml', 'utf8');
  assert.match(workflow, /push:\s*\n\s*branches: \[main\]/);
});
