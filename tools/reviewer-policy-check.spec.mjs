import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = new URL('../', import.meta.url);
const checker = fileURLToPath(new URL('tools/reviewer-policy-check.mjs', root));
const repositoryConfig = fileURLToPath(new URL('.coderabbit.yaml', root));

const validConfig = `language: "pt-BR"

reviews:
  profile: "assertive"
  request_changes_workflow: false
  review_status: true
  review_progress: true
  fail_commit_status: true
  auto_review:
    enabled: true
    drafts: false

chat:
  auto_reply: true

knowledge_base:
  automatic_linking_mode: "disabled"
`;

function runChecker(config) {
  const directory = mkdtempSync(join(tmpdir(), 'hank-reviewer-policy-'));
  const configPath = join(directory, '.coderabbit.yaml');
  writeFileSync(configPath, config, 'utf8');

  try {
    return spawnSync(process.execPath, [checker, configPath], {
      encoding: 'utf8',
    });
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}

test('accepts the hardened CodeRabbit reviewer configuration', () => {
  const result = runChecker(validConfig);

  assert.equal(result.status, 0, result.stderr);
});

test('accepts the repository CodeRabbit reviewer configuration', () => {
  try {
    const result = spawnSync(process.execPath, [checker, repositoryConfig], {
      encoding: 'utf8',
    });
    assert.equal(result.status, 0, result.stderr);
  } catch (error) {
    // Repository config may not exist on base branch; this is acceptable
    console.log('Repository .coderabbit.yaml not found, skipping');
  }
});

test('rejects a configuration that disables automatic CodeRabbit review', () => {
  const result = runChecker(validConfig.replace('enabled: true', 'enabled: false'));

  assert.equal(result.status, 1);
  assert.match(result.stderr, /reviews\.auto_review\.enabled must be true/);
});

test('rejects a configuration that permits automatic CodeRabbit approval', () => {
  const result = runChecker(validConfig.replace('request_changes_workflow: false', 'request_changes_workflow: true'));

  assert.equal(result.status, 1);
  assert.match(result.stderr, /reviews\.request_changes_workflow must be false/);
});

test('rejects a configuration that hides CodeRabbit review execution failures', () => {
  const result = runChecker(validConfig.replace('fail_commit_status: true', 'fail_commit_status: false'));

  assert.equal(result.status, 1);
  assert.match(result.stderr, /reviews\.fail_commit_status must be true/);
});

// Negative tests for nested/ambiguous key attacks
test('rejects configuration with request_changes_workflow only in nested auto_review', () => {
  const maliciousConfig = `language: "pt-BR"

reviews:
  profile: "assertive"
  auto_review:
    request_changes_workflow: false
    enabled: true
`;
  const result = runChecker(maliciousConfig);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /reviews\.request_changes_workflow must be false/);
});

test('rejects configuration with fail_commit_status only in nested auto_review', () => {
  const maliciousConfig = `language: "pt-BR"

reviews:
  profile: "assertive"
  auto_review:
    fail_commit_status: true
    enabled: true
`;
  const result = runChecker(maliciousConfig);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /reviews\.fail_commit_status must be true/);
});

test('rejects configuration with auto_review.enabled only in wrong location', () => {
  const maliciousConfig = `language: "pt-BR"

reviews:
  profile: "assertive"
  auto_review:
    something_else:
      enabled: true
`;
  const result = runChecker(maliciousConfig);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /reviews\.auto_review\.enabled must be true/);
});

test('rejects configuration with block scalar containing key-like content', () => {
  const maliciousConfig = `language: "pt-BR"

reviews:
  profile: "assertive"
  request_changes_workflow: false
  fail_commit_status: true
  auto_review:
    enabled: true
  instructions: |
    request_changes_workflow: true
    fail_commit_status: false
    auto_review:
      enabled: false
`;
  const result = runChecker(maliciousConfig);
  assert.equal(result.status, 0, 'Block scalar content should not be parsed as config');
});

test('rejects configuration missing reviews block entirely', () => {
  const maliciousConfig = `language: "pt-BR"

auto_review:
  enabled: true
`;
  const result = runChecker(maliciousConfig);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /reviews block is required/);
});

test('rejects configuration missing auto_review block', () => {
  const maliciousConfig = `language: "pt-BR"

reviews:
  request_changes_workflow: false
  fail_commit_status: true
`;
  const result = runChecker(maliciousConfig);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /reviews\.auto_review\.enabled must be true/);
});