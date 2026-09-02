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
tone_instructions: "Seja conciso e direto. Escreva em pt-BR. Use somente texto; não use emojis, ícones ou floreios."

reviews:
  profile: "chill"
  request_changes_workflow: false
  review_status: false
  review_details: false
  collapse_walkthrough: true
  changed_files_summary: false
  sequence_diagrams: false
  estimate_code_review_effort: false
  assess_linked_issues: false
  related_issues: false
  related_prs: false
  suggested_labels: false
  suggested_reviewers: false
  in_progress_fortune: false
  poem: false
  enable_prompt_for_ai_agents: false
  high_level_summary: true
  high_level_summary_instructions: "Liste impacto, risco e testes em até 3 bullets. Sem emojis."
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

test('rejects a reviewer tone that includes emoji', () => {
  const result = runChecker(validConfig.replace('não use emojis, ícones ou floreios.', 'use 🙂 quando apropriado.'));

  assert.equal(result.status, 1);
  assert.match(result.stderr, /tone_instructions must not contain emoji/);
});

test('rejects encoded emoji escapes in reviewer text', () => {
  for (const encodedEmoji of [String.raw`"\U0001F642"`, String.raw`"\u263A"`]) {
    const toneResult = runChecker(validConfig.replace(
      /tone_instructions:.*\n/,
      `tone_instructions: ${encodedEmoji}\n`,
    ));
    const summaryResult = runChecker(validConfig.replace(
      '  high_level_summary_instructions: "Liste impacto, risco e testes em até 3 bullets. Sem emojis."\n',
      `  high_level_summary_instructions: ${encodedEmoji}\n`,
    ));

    assert.equal(toneResult.status, 1);
    assert.match(toneResult.stderr, /tone_instructions must not contain emoji/);
    assert.equal(summaryResult.status, 1);
    assert.match(summaryResult.stderr, /reviews\.high_level_summary_instructions must not contain emoji/);
  }
});

test('accepts legal non-emoji YAML escapes in reviewer text', () => {
  const tone = String.raw`"Use \"quoted\" wording."`;
  const summary = String.raw`"Liste impacto.\nUse tabs\tonly when needed."`;
  const toneResult = runChecker(validConfig.replace(/tone_instructions:.*\n/, `tone_instructions: ${tone}\n`));
  const summaryResult = runChecker(validConfig.replace(
    '  high_level_summary_instructions: "Liste impacto, risco e testes em até 3 bullets. Sem emojis."\n',
    `  high_level_summary_instructions: ${summary}\n`,
  ));

  assert.equal(toneResult.status, 0, toneResult.stderr);
  assert.equal(summaryResult.status, 0, summaryResult.stderr);
});

test('rejects trailing tokens after an early double-quote closure', () => {
  const malformed = String.raw`"ok\u0041"junk"`;
  const toneResult = runChecker(validConfig.replace(/tone_instructions:.*\n/, `tone_instructions: ${malformed}\n`));
  const summaryResult = runChecker(validConfig.replace(
    '  high_level_summary_instructions: "Liste impacto, risco e testes em até 3 bullets. Sem emojis."\n',
    `  high_level_summary_instructions: ${malformed}\n`,
  ));

  assert.equal(toneResult.status, 1);
  assert.match(toneResult.stderr, /tone_instructions contains unexpected closing quote/);
  assert.equal(summaryResult.status, 1);
  assert.match(summaryResult.stderr, /reviews\.high_level_summary_instructions contains unexpected closing quote/);
});

test('rejects trailing tokens after an early single-quote closure', () => {
  const malformed = String.raw`'ok'junk'`;
  const toneResult = runChecker(validConfig.replace(/tone_instructions:.*\n/, `tone_instructions: ${malformed}\n`));
  const summaryResult = runChecker(validConfig.replace(
    '  high_level_summary_instructions: "Liste impacto, risco e testes em até 3 bullets. Sem emojis."\n',
    `  high_level_summary_instructions: ${malformed}\n`,
  ));

  assert.equal(toneResult.status, 1);
  assert.match(toneResult.stderr, /tone_instructions contains malformed single-quoted scalar/);
  assert.equal(summaryResult.status, 1);
  assert.match(summaryResult.stderr, /reviews\.high_level_summary_instructions contains malformed single-quoted scalar/);
});

test('rejects flow collections in reviewer text settings', () => {
  const flowValues = ['[]', '{}', '[text, {nested: value}]'];

  for (const flowValue of flowValues) {
    const toneResult = runChecker(validConfig.replace(
      /tone_instructions:.*\n/,
      `tone_instructions: ${flowValue}\n`,
    ));
    const summaryResult = runChecker(validConfig.replace(
      '  high_level_summary_instructions: "Liste impacto, risco e testes em até 3 bullets. Sem emojis."\n',
      `  high_level_summary_instructions: ${flowValue}\n`,
    ));

    assert.equal(toneResult.status, 1);
    assert.match(toneResult.stderr, /tone_instructions must be an inline scalar/);
    assert.equal(summaryResult.status, 1);
    assert.match(summaryResult.stderr, /reviews\.high_level_summary_instructions must use an inline scalar/);
  }
});

test('rejects a missing concise reviewer tone', () => {
  const result = runChecker(validConfig.replace(/tone_instructions:.*\n/, ''));

  assert.equal(result.status, 1);
  assert.match(result.stderr, /tone_instructions must be an inline scalar/);
});

test('rejects an assertive reviewer profile', () => {
  const result = runChecker(validConfig.replace('profile: "chill"', 'profile: "assertive"'));

  assert.equal(result.status, 1);
  assert.match(result.stderr, /reviews\.profile must be chill/);
});

test('rejects verbose walkthrough settings', () => {
  const result = runChecker(validConfig.replace('sequence_diagrams: false', 'sequence_diagrams: true'));

  assert.equal(result.status, 1);
  assert.match(result.stderr, /reviews\.sequence_diagrams must be false/);
});

test('accepts the repository CodeRabbit reviewer configuration', () => {
  const result = spawnSync(process.execPath, [checker, repositoryConfig], {
    encoding: 'utf8',
  });

  assert.equal(result.status, 0, result.stderr);
});

test('does not accept output settings hidden inside a block scalar', () => {
  const result = runChecker(`language: "pt-BR"
tone_instructions: "Seja conciso e direto. Sem emojis."
chat: |
  profile: "chill"
  review_status: false
  review_details: false
  collapse_walkthrough: true
  changed_files_summary: false
  sequence_diagrams: false
  estimate_code_review_effort: false
  assess_linked_issues: false
  related_issues: false
  related_prs: false
  suggested_labels: false
  suggested_reviewers: false
  in_progress_fortune: false
  poem: false
  enable_prompt_for_ai_agents: false
reviews:
  request_changes_workflow: false
  fail_commit_status: true
  auto_review:
    enabled: true
`);

  assert.equal(result.status, 1);
  assert.match(result.stderr, /reviews\.profile must be chill/);
  assert.match(result.stderr, /reviews\.review_details must be false/);
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
  const result = runChecker(validConfig
    .replace('reviews:\n', 'reviews: # reviewer settings\n')
    .replace('auto_review:\n', 'auto_review: # automatic review\n'));

  assert.equal(result.status, 0, result.stderr);
});

test('accepts a plain scalar with an embedded quote before an inline comment', () => {
  const result = runChecker(validConfig.replace(
    '  high_level_summary_instructions: "Liste impacto, risco e testes em até 3 bullets. Sem emojis."\n',
    `  high_level_summary_instructions: plain "quoted # ${'x'.repeat(101)} trailing comment\n`,
  ));

  assert.equal(result.status, 0, result.stderr);
});

test('accepts a single-quoted scalar with a literal backslash and doubled quote', () => {
  const summary = String.raw`'literal \''' # trailing ${'x'.repeat(101)}`;
  const config = validConfig.replace(
    '  high_level_summary_instructions: "Liste impacto, risco e testes em até 3 bullets. Sem emojis."\n',
    `  high_level_summary_instructions: ${summary}\n`,
  );
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

test('rejects a bare-tagged summary scalar before measuring its value', () => {
  const result = runChecker(`reviews:
  request_changes_workflow: false
  fail_commit_status: true
  high_level_summary_instructions: ! "short # ${'x'.repeat(101)}"
  auto_review:
    enabled: true
`);

  assert.equal(result.status, 1);
  assert.match(result.stderr, /reviewer configuration must use untagged, unanchored scalars/);
});

test('accepts a decorated scalar in an unrelated reviewer setting', () => {
  const result = runChecker(validConfig.replace('language: "pt-BR"', 'language: &locale "pt-BR"'));

  assert.equal(result.status, 0, result.stderr);
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

test('rejects unsupported YAML after a valid reviews block', () => {
  const result = runChecker(`reviews:
  request_changes_workflow: false
  fail_commit_status: true
  auto_review:
    enabled: true
not valid yaml
`);

  assert.equal(result.status, 1);
  assert.match(result.stderr, /reviewer configuration contains unsupported YAML syntax/);
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
