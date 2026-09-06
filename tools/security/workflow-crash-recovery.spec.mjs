import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const source = readFileSync(new URL('./workflow-crash-recovery-feature-tests.mjs', import.meta.url), 'utf8');

// @spec:AC-1053
test('recovery evidence rejects unallowed dirty paths and preserves identity @spec:AC-1053', () => {
  assert.match(source, /statusAllowed\(before\.status\)/);
  assert.match(source, /statusAllowed\(after\.status\)/);
  assert.match(source, /after\.head !== before\.head/);
  assert.match(source, /after\.tree !== before\.tree/);
  assert.match(source, /after\.status !== before\.status/);
});
