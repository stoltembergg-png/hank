import assert from 'node:assert/strict';
import test from 'node:test';
import { isConventionalCommit, validateMessages } from './commit-message-lint.mjs';

test('accepts valid conventional commit headers', () => {
  for (const subject of [
    'feat: add project service',
    'fix(ui): reject remote origin',
    'ci!: change required workflow contract',
    'docs: explain rollback',
    'Merge pull request #1 from example/branch',
    'Merge 0e3c041b30718067a96987106093ea5a35dea323 into 6723ead32ab640a15a02e67ea29da803a34d7a75',
  ]) assert.equal(isConventionalCommit(subject), true, subject);
});

test('rejects invalid, empty, and overlong headers', () => {
  const results = validateMessages([
    'bad message without type',
    'feat:',
    `fix: ${'x'.repeat(70)}`,
    '',
  ]);
  assert.deepEqual(results.map((result) => result.valid), [false, false, false]);
});

test('does not execute commit text', () => {
  assert.equal(isConventionalCommit('feat: $(touch /tmp/should-not-exist)'), true);
});
