import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const source = readFileSync(new URL('./load-feature-tests.mjs', import.meta.url), 'utf8');

// @spec:AC-2304
test('load bridge isolates Cargo environment and rename paths', () => {
  assert.match(source, /const safeEnv = \{/);
  assert.doesNotMatch(source, /env: \{\.\.\.process\.env/);
  assert.match(source, /porcelain=v1/);
  assert.match(source, /function gitRaw/);
  assert.match(source, /code\.includes\(['"]R['"]\)/);
  assert.match(source, /code\.includes\(['"]C['"]\)/);
});
