import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const workflowPath = new URL('../.github/workflows/review-remediation-agent.yml', import.meta.url);
const guidePath = new URL('../docs/review-remediation-agent.md', import.meta.url);

function readWorkflow() {
  return readFileSync(workflowPath, 'utf8');
}

function jobBlock(workflow, name) {
  const marker = new RegExp(`^  ${name}:\\s*$`, 'm');
  const match = marker.exec(workflow);
  if (!match) return '';
  const start = match.index;
  const next = workflow.slice(start + match[0].length).search(/^  [a-zA-Z0-9_-]+:\s*$/m);
  return workflow.slice(start, next === -1 ? workflow.length : start + match[0].length + next);
}

test('declares only the supported review events and four bounded jobs', () => {
  const workflow = readWorkflow();
  assert.match(workflow, /^name:\s*Review remediation agent\s*$/m);
  assert.match(workflow, /^on:\s*$/m);
  assert.match(workflow, /^  pull_request_review:\s*$/m);
  assert.match(workflow, /^  check_run:\s*$/m);
  assert.doesNotMatch(workflow, /^  (?:push|pull_request|workflow_dispatch|schedule):/m);
  assert.match(workflow, /^permissions:\s*$/m);
  assert.match(workflow, /^concurrency:\s*$/m);
  assert.match(workflow, /group:\s*review-remediation-\$\{\{\s*github\.repository\s*\}\}-\$\{\{\s*github\.run_id\s*\}\}/);
  assert.doesNotMatch(workflow, /group:[^\n]*github\.event_name/);
  assert.match(workflow, /cancel-in-progress:\s*false/);
  assert.doesNotMatch(workflow, /group:[^\n]*\}\}-\$\{\{\s*github\.event_name\s*\}\}/);
  for (const job of ['collect', 'propose', 'validate', 'publish']) {
    const block = jobBlock(workflow, job);
    assert.ok(block, `${job} job is missing`);
    assert.match(block, /timeout-minutes:\s*[1-9][0-9]*/);
  }
});

test('keeps trust boundaries between collect/propose/validate/publish', () => {
  const workflow = readWorkflow();
  const collect = jobBlock(workflow, 'collect');
  const propose = jobBlock(workflow, 'propose');
  const validate = jobBlock(workflow, 'validate');
  const publish = jobBlock(workflow, 'publish');

  assert.doesNotMatch(collect, /XIAOMI_MIMO_API_KEY/);
  assert.match(propose, /XIAOMI_MIMO_API_KEY:\s*\$\{\{\s*secrets\.XIAOMI_MIMO_API_KEY\s*\}\}/);
  assert.doesNotMatch(validate, /XIAOMI_MIMO_API_KEY/);
  assert.doesNotMatch(publish, /XIAOMI_MIMO_API_KEY/);
  assert.match(collect, /contents:\s*read/);
  assert.match(validate, /contents:\s*read/);
  assert.match(publish, /contents:\s*write/);
  assert.match(publish, /pull-requests:\s*write/);
  assert.doesNotMatch(collect, /contents:\s*write/);
  assert.doesNotMatch(propose, /contents:\s*write/);
  assert.doesNotMatch(validate, /contents:\s*write/);
  assert.match(publish, /gh pr create --draft/);
  assert.doesNotMatch(workflow, /gh pr (?:merge|review|approve)/i);
  assert.doesNotMatch(workflow, /--force(?:-with-lease)?\b/);
});

test('pins actions and source identity, and prevents pull-request code from using credentials', () => {
  const workflow = readWorkflow();
  for (const sha of [
    '3d3c42e5aac5ba805825da76410c181273ba90b1',
    '820762786026740c76f36085b0efc47a31fe5020',
    '043fb46d1a93c77aae656e7c1c64a875d1fc6a0a',
    'fa0a91b85d4f404e444e00e005971372dc801d16',
  ]) assert.match(workflow, new RegExp(sha));
  assert.doesNotMatch(workflow, /pull_request_target/);
  assert.match(workflow, /persist-credentials:\s*false/g);
  assert.match(workflow, /ref:\s*\$\{\{\s*needs\.collect\.outputs\.source_sha\s*\}\}/);
  assert.match(workflow, /repository:\s*\$\{\{\s*github\.repository\s*\}\}/);
  assert.match(workflow, /git -C target rev-parse HEAD/);
  assert.match(workflow, /sha256sum/);
  assert.doesNotMatch(workflow, /awk '\{print \\$1\}'/);
  assert.match(workflow, /review-remediation\/pr-/);
  assert.match(workflow, /if:\s*\$\{\{\s*needs\.collect\.outputs\.status\s*==\s*'READY'/);
  assert.match(workflow, /if:\s*\$\{\{\s*needs\.propose\.outputs\.status\s*==\s*'PROPOSED'/);
  assert.match(workflow, /if:\s*\$\{\{\s*needs\.validate\.outputs\.status\s*==\s*'VALIDATED'/);
});

test('fails closed during first rollout when the trusted helper is not on the default branch', () => {
  const collect = jobBlock(readWorkflow(), 'collect');
  assert.match(collect, /if \[\[ -f tools\/review-remediation-agent\.mjs \]\]/);
  assert.match(collect, /HELPER_NOT_ON_DEFAULT_BRANCH/);
  assert.match(collect, /status.*NOOP/);
});

test('does not execute source-controlled build or package scripts during validation', () => {
  const workflow = readWorkflow();
  const validate = jobBlock(workflow, 'validate');
  const publish = jobBlock(workflow, 'publish');
  assert.match(validate, /git diff --check/);
  assert.match(validate, /Array\.isArray\(value\.gates\)/);
  assert.match(validate, /value\.gates\.length === 5/);
  assert.match(validate, /semantic-syntax/);
  assert.match(validate, /gate\?\.status === 'PASS'/);
  assert.doesNotMatch(validate, /cargo test|cargo clippy|npm (?:ci|run|test|exec)|pnpm |yarn /i);
  assert.match(validate, /proposal\/proposal\.json/);
  assert.match(publish, /--proposal proposal\/proposal\.json/);
  assert.match(publish, /EXPECTED_SOURCE_PR/);
  assert.match(publish, /EXPECTED_SOURCE_SHA/);
  assert.match(publish, /git -C target diff --exit-code/);
  assert.match(publish, /expected_files/);
});

test('fixes the MiMo model and endpoint in trusted workflow code', () => {
  const workflow = readWorkflow();
  assert.match(workflow, /mimo-v2\.5/);
  assert.match(workflow, /https:\/\/api\.xiaomimimo\.com\/v1/);
  assert.match(workflow, /review-remediation-agent\.mjs (?:propose|validate|publish)/);
  assert.doesNotMatch(workflow, /\$\{\{\s*github\.event\.[^}]+\.model/);
});

test('documents operations without embedding a credential', () => {
  const guide = readFileSync(guidePath, 'utf8');
  assert.match(guide, /XIAOMI_MIMO_API_KEY/);
  assert.match(guide, /mimo-v2\.5/);
  assert.match(guide, /https:\/\/api\.xiaomimimo\.com\/v1/);
  assert.match(guide, /fork/i);
  assert.match(guide, /rascunho/i);
  assert.match(guide, /(?:não|nao)[ -]?(?:aprova|faz merge)|no auto-merge/i);
  assert.match(guide, /rota(?:ção|cao)|rotate/i);
  assert.match(guide, /rollback/i);
  assert.doesNotMatch(guide, /(?:sk|ghp|github_pat)_[A-Za-z0-9_-]{12,}/);
});
