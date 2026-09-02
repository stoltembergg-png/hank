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
  const result = spawnSync(process.execPath, [checker, repositoryConfig], {
    encoding: 'utf8',
  });

  assert.equal(result.status, 0, result.stderr);
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

test('rejects reviewer values nested below their canonical CodeRabbit paths', () => {
  const result = runChecker(`reviews:
  auto_review:
    request_changes_workflow: false
    fail_commit_status: true
    nested:
      enabled: true
`);

  assert.equal(result.status, 1);
  assert.match(result.stderr, /reviews\.request_changes_workflow must be false/);
  assert.match(result.stderr, /reviews\.fail_commit_status must be true/);
  assert.match(result.stderr, /reviews\.auto_review\.enabled must be true/);
});

test('ignores reviewer-looking keys inside block scalar content', () => {
  const result = runChecker(`reviews:
  auto_review:
    enabled: true
  high_level_summary_instructions: |
    request_changes_workflow: false
    fail_commit_status: true
`);

  assert.equal(result.status, 1);
  assert.match(result.stderr, /reviews\.request_changes_workflow must be false/);
  assert.match(result.stderr, /reviews\.fail_commit_status must be true/);
});

test('rejects duplicate canonical reviewer keys', () => {
  const result = runChecker(`reviews:
  request_changes_workflow: false
  request_changes_workflow: true
  fail_commit_status: true
  auto_review:
    enabled: true
`);

  assert.equal(result.status, 1);
  assert.match(result.stderr, /reviews\.request_changes_workflow must be false/);
});

test('rejects a configuration with a missing canonical reviewer path', () => {
  const result = runChecker(`reviews:
  fail_commit_status: true
  auto_review:
    enabled: true
`);

  assert.equal(result.status, 1);
  assert.match(result.stderr, /reviews\.request_changes_workflow must be false/);
});

test('rejects reviewer-looking keys inside an explicitly-indented block scalar', () => {
  const result = runChecker(`reviews:
  high_level_summary_instructions: |2
    request_changes_workflow: false
    fail_commit_status: true
    auto_review:
      enabled: true
`);

  assert.equal(result.status, 1);
  assert.match(result.stderr, /reviews\.request_changes_workflow must be false/);
  assert.match(result.stderr, /reviews\.fail_commit_status must be true/);
  assert.match(result.stderr, /reviews\.auto_review\.enabled must be true/);
});

test('rejects reviewer-looking keys inside a YAML sequence', () => {
  const result = runChecker(`reviews:
  - request_changes_workflow: false
    fail_commit_status: true
    auto_review:
      enabled: true
`);

  assert.equal(result.status, 1);
  assert.match(result.stderr, /reviews\.request_changes_workflow must be false/);
});

test('accepts inline comments on canonical YAML block headers', () => {
  const result = runChecker(`reviews: # reviewer settings
  request_changes_workflow: false
  fail_commit_status: true
  auto_review: # automatic review
    enabled: true
`);

  assert.equal(result.status, 0, result.stderr);
});

test('accepts a plain scalar with an embedded quote before an inline comment', () => {
  const result = runChecker(`reviews:
  request_changes_workflow: false
  fail_commit_status: true
  high_level_summary_instructions: plain "quoted # ${'x'.repeat(101)} trailing comment
  auto_review:
    enabled: true
`);

  assert.equal(result.status, 0, result.stderr);
});

test('accepts a single-quoted scalar with a literal backslash and doubled quote', () => {
  const config = [
    'reviews:',
    '  request_changes_workflow: false',
    '  fail_commit_status: true',
    "  high_level_summary_instructions: 'literal \\''' # trailing " + 'x'.repeat(101),
    '  auto_review:',
    '    enabled: true',
  ].join('\n');
  const result = runChecker(config);

  assert.equal(result.status, 0, result.stderr);
});

test('rejects tagged and anchored summary scalars before measuring their value', () => {
  const result = runChecker(`reviews:
  request_changes_workflow: false
  fail_commit_status: true
  high_level_summary_instructions: &summary !!str "short # ${'x'.repeat(101)}"
  auto_review:
    enabled: true
`);

  assert.equal(result.status, 1);
  assert.match(result.stderr, /reviewer configuration must use untagged, unanchored scalars/);
});

test('rejects an oversized CodeRabbit summary instruction', () => {
  const result = runChecker(`reviews:
  request_changes_workflow: false
  fail_commit_status: true
  high_level_summary_instructions: "${'x'.repeat(101)}"
  auto_review:
    enabled: true
`);

  assert.equal(result.status, 1);
  assert.match(result.stderr, /reviews\.high_level_summary_instructions must be at most 100 characters/);
});

test('rejects a block-scalar CodeRabbit summary instruction', () => {
  const result = runChecker(`reviews:
  request_changes_workflow: false
  fail_commit_status: true
  high_level_summary_instructions: |-
    ${'x'.repeat(101)}
  auto_review:
    enabled: true
`);

  assert.equal(result.status, 1);
  assert.match(result.stderr, /reviews\.high_level_summary_instructions must use an inline scalar/);
});

test('rejects an unclosed quoted scalar', () => {
  const result = runChecker(`reviews:
  request_changes_workflow: false
  fail_commit_status: true
  profile: "assertive
  auto_review:
    enabled: true
`);

  assert.equal(result.status, 1);
  assert.match(result.stderr, /reviewer configuration contains an unclosed quoted scalar/);
});

test('rejects a quoted or scalar reviews parent', () => {
  const quoted = runChecker(`"reviews":
  request_changes_workflow: false
  fail_commit_status: true
  auto_review:
    enabled: true
`);
  const scalar = runChecker(`reviews: []
`);

  assert.equal(quoted.status, 1);
  assert.match(quoted.stderr, /reviews block is required/);
  assert.equal(scalar.status, 1);
  assert.match(scalar.stderr, /reviews block is required/);
});
