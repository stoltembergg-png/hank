import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

test('ONP evidence runs on pull requests and integrated main pushes @spec:AC-302', () => {
  const workflow = readFileSync('.github/workflows/onp-sdd-evidence.yml', 'utf8');
  assert.match(workflow, /pull_request:\s*\n\s*push:\s*\n\s*branches:\s*\n\s*- main/);
});
