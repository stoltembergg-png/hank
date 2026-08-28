import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

test('ONP SDD evidence uses a jsdom-compatible pinned Node runtime @spec:AC-302', () => {
  const workflow = readFileSync('.github/workflows/onp-sdd-evidence.yml', 'utf8');
  assert.match(workflow, /Setup pinned Node\.js 22/);
  assert.match(workflow, /node-version: 22\.22\.2/);
  assert.doesNotMatch(workflow, /node-version: 20\.19\.1/);
});
