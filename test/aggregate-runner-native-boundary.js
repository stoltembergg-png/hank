import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const runner = readFileSync(new URL('../tools/run-all-tests.mjs', import.meta.url), 'utf8');
const workflow = readFileSync(
  new URL('../.github/workflows/onp-sdd-evidence.yml', import.meta.url),
  'utf8',
);

test('aggregate runner exposes an explicit native-test boundary', () => {
  assert.match(runner, /HANK_SKIP_TAURI/);
  assert.match(runner, /Tauri acceptance tests/);
  assert.match(runner, /process\.env\.HANK_SKIP_TAURI/);
});

test('ONP workflow runs native Tauri coverage explicitly', () => {
  assert.match(workflow, /HANK_SKIP_TAURI:\s*['"]?1/);
  assert.match(workflow, /Verify Tauri desktop[\s\S]*?HANK_SKIP_TAURI:\s*['"]?['"]?[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify tauri-desktop/);
});

test('ONP workflow runs repository workspace verification explicitly', () => {
  assert.match(
    workflow,
    /Verify repository workspace[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify repository-workspace/,
  );
});

test('ONP workflow runs native evaluation contract verification explicitly', () => {
  assert.match(
    workflow,
    /Verify native evaluation contract[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify native-evaluation-contract/,
  );
});

test('ONP workflow runs Git worktree verification explicitly', () => {
  assert.match(
    workflow,
    /Verify Git worktree manager[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify git-worktree-manager/,
  );
});

test('ONP workflow runs branch policy verification explicitly', () => {
  assert.match(
    workflow,
    /Verify branch policy[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify branch-policy/,
  );
});

test('ONP workflow runs task-to-branch mapping verification explicitly', () => {
  assert.match(
    workflow,
    /Verify task-to-branch mapping[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify task-branch-mapping/,
  );
});

test('ONP workflow runs reviewer agent profile verification explicitly', () => {
  assert.match(
    workflow,
    /Verify reviewer agent profile[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify reviewer-agent-profile/,
  );
});
test('ONP workflow runs QA agent profile verification explicitly', () => {
  assert.match(
    workflow,
    /Verify QA agent profile[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify qa-agent-profile/,
  );
});

test('ONP workflow runs security agent profile verification explicitly', () => {
  assert.match(
    workflow,
    /Verify security agent profile[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify security-agent-profile/,
  );
});

test('ONP workflow runs architecture agent profile verification explicitly', () => {
  assert.match(
    workflow,
    /Verify architecture agent profile[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify architecture-agent-profile/,
  );
});

test('ONP workflow runs PR generation workflow verification explicitly', () => {
  assert.match(
    workflow,
    /Verify PR generation workflow[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify pr-generation-workflow/,
  );
  assert.match(
    workflow,
    /Verify review workflow[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify review-workflow/,
  );
  assert.match(
    workflow,
    /Verify CI status integration[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify ci-status-integration/,
  );
  assert.match(
    workflow,
    /Verify fix-review workflow[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify fix-review-workflow/,
  );
  assert.match(
    workflow,
    /Verify release-agent workflow[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify release-agent-workflow/,
  );
  assert.match(
    workflow,
    /Verify improvement observation event[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify improvement-observation-event/,
  );
  assert.match(
    workflow,
    /Verify improvement candidate entity[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify improvement-candidate-entity/,
  );
  assert.match(
    workflow,
    /Verify self-evaluation workflow[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify self-evaluation-workflow/,
  );
  assert.match(
    workflow,
    /Verify skill improvement proposal[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify skill-improvement-proposal/,
  );
  assert.match(
    workflow,
    /Verify workflow improvement proposal[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify workflow-improvement-proposal/,
  );
  assert.match(
    workflow,
    /Verify planning reconciliation[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify planning-reconciliation/,
  );
  assert.match(
    workflow,
    /Verify Claim\/Evidence contract[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify claim-evidence/,
  );
  assert.match(
    workflow,
    /Verify planning evidence binding[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify planning-evidence-binding/,
  );
  assert.match(
    workflow,
    /Verify planning adversarial E2E[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify planning-adversarial-e2e/,
  );
  assert.match(
    workflow,
    /Verify agent configuration proposal[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify agent-configuration-proposal/,
  );
  assert.match(
    workflow,
    /Verify automated evaluation[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify automated-evaluation/,
  );
  assert.match(
    workflow,
    /Verify regression evaluation[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify regression-evaluation/,
  );
  assert.match(
    workflow,
    /Verify improvement scoring[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify improvement-scoring/,
  );
  assert.match(
    workflow,
    /Verify automatic rollback[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify automatic-rollback/,
  );
  assert.match(
    workflow,
    /Verify automatic skill rollout[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify automatic-skill-rollout/,
  );
  assert.match(
    workflow,
    /Verify self-development issue[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify self-development-issue/,
  );
  assert.match(
    workflow,
    /Verify self-development branch[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify self-development-branch/,
  );
  assert.match(
    workflow,
    /Verify self-development PR[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify self-development-pr/,
  );
  assert.match(
    workflow,
    /Verify MCP transport abstraction[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify mcp-transport-abstraction/,
  );
  assert.match(
    workflow,
    /Verify MCP stdio client[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify mcp-stdio-client/,
  );
  assert.match(
    workflow,
    /Verify MCP HTTP client[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify mcp-http-client/,
  );
  assert.match(
    workflow,
    /Verify MCP permission integration[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify mcp-permission-integration/,
  );
  assert.match(
    workflow,
    /Verify MCP tool discovery[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify mcp-tool-discovery/,
  );
  assert.match(
    workflow,
    /Verify MCP settings UI[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify mcp-settings-ui/,
  );
  assert.match(
    workflow,
    /Verify plugin manifest[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify plugin-manifest/,
  );
  assert.match(
    workflow,
    /Verify plugin discovery[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify plugin-discovery/,
  );
  assert.match(
    workflow,
    /Verify plugin lifecycle[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify plugin-lifecycle/,
  );
  assert.match(
    workflow,
    /Verify plugin permissions[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify plugin-permissions/,
  );
  assert.match(
    workflow,
    /Verify provider plugins[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify provider-plugins/,
  );
  assert.match(
    workflow,
    /Verify tool plugins[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify tool-plugins/,
  );
  assert.match(
    workflow,
    /Verify runtime transport[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify runtime-transport/,
  );
});

test('ONP workflow runs coding agent profile verification explicitly', () => {
  assert.match(
    workflow,
    /Verify coding agent profile[\s\S]*?node tools\/ci\/run-onp-spec\.mjs verify coding-agent-profile/,
  );
});
