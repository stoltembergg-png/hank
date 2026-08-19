import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const config = readFileSync('.github/dependabot.yml', 'utf8');

function section(ecosystem) {
  const start = config.indexOf(`package-ecosystem: ${ecosystem}`);
  assert.notEqual(start, -1, `${ecosystem} update is missing`);
  const end = config.indexOf('\n  - package-ecosystem:', start + 1);
  return config.slice(start, end === -1 ? config.length : end);
}

test('Dependabot covers Cargo, frontend npm, and GitHub Actions', () => {
  assert.match(section('cargo'), /directory: \/\n/);
  assert.match(section('npm'), /directory: \/frontend\n/);
  assert.match(section('github-actions'), /directory: \/\n/);
});

test('Dependabot updates are bounded and weekly', () => {
  for (const ecosystem of ['cargo', 'npm', 'github-actions']) {
    const block = section(ecosystem);
    assert.match(block, /interval: weekly/);
    assert.match(block, /open-pull-requests-limit: 5/);
  }
});

test('Dependabot configuration cannot authorize automatic merge or secrets', () => {
  assert.doesNotMatch(config, /auto[-_]merge|allow[-_]merge|secrets:/i);
});
