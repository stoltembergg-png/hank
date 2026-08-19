import assert from 'node:assert/strict';
import test from 'node:test';
import { isValidPrTitle } from './pr-title-lint.mjs';

test('accepts queue and conventional titles', () => {
  for (const title of ['PR-014: Validate PR title', 'ci: pin workflow action', 'fix(ui): reject remote origin']) {
    assert.equal(isValidPrTitle(title), true, title);
  }
});

test('rejects ambiguous titles', () => {
  for (const title of ['', 'work', 'PR-14: missing zero', 'feat:', 'x'.repeat(101)]) {
    assert.equal(isValidPrTitle(title), false, title);
  }
});

test('does not execute title text', () => {
  assert.equal(isValidPrTitle('PR-014: $(touch /tmp/title-should-not-exist)'), true);
});
