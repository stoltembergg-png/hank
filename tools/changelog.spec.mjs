import assert from 'node:assert/strict';
import test from 'node:test';
import { parseCommit, renderChangelog } from './changelog.mjs';

const commits = [
  parseCommit('1111111111111111\tfeat(core): add project service'),
  parseCommit('2222222222222222\tfix!: reject invalid state'),
  parseCommit('3333333333333333\tdocs: explain rollback'),
];

test('parses conventional commit metadata', () => {
  assert.deepEqual(parseCommit('abc1234\tfeat(ui): add shell'), {
    sha: 'abc1234', subject: 'feat(ui): add shell', type: 'feat', scope: 'ui', breaking: false,
  });
});

test('renders deterministic categorized output', () => {
  const args = { range: 'base...tip', tip: 'tipsha', commits };
  const first = renderChangelog(args);
  assert.equal(first, renderChangelog(args));
  assert.match(first, /## Added/);
  assert.match(first, /## Fixed/);
  assert.match(first, /BREAKING/);
  assert.match(first, /source-tip: tipsha/);
});

test('wrong source identity produces a different proposal', () => {
  const valid = renderChangelog({ range: 'base...tip', tip: 'tipsha', commits });
  const wrong = renderChangelog({ range: 'old...tip', tip: 'tipsha', commits });
  assert.notEqual(valid, wrong);
});
