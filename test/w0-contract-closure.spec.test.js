import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {
  parseQueueCards,
  validateQueue,
  validateCard,
  validateCards,
  validateArchitectureGraph,
  evaluateExecution,
  evaluateGate
} from '../tools/w0-contract-validator.mjs';

const root = process.cwd();
const read = (relative) => fs.readFileSync(path.join(root, relative), 'utf8');
const json = (relative) => JSON.parse(read(relative));
const contract = (name) => `.planning/contracts/${name}`;
const sha = '0123456789abcdef0123456789abcdef01234567';

// US-001 — Fronteira independente do core
test('AC-001: Grafo de camadas e ownership @spec:AC-001', () => {
  const graph = json(contract('architecture-graph.json'));
  const schema = json(contract('architecture-graph.schema.json'));
  assert.deepEqual(schema.required, ['schema_version', 'layers', 'edges', 'allowed_edges', 'forbidden_edges', 'evidence_policy']);
  assert.equal(validateArchitectureGraph(graph).status, 'PASS');
  for (const layer of graph.layers) for (const field of ['owner', 'responsibility', 'process_lifecycle', 'contract']) assert.ok(layer[field]);
});

test('AC-002: Adapter não-Tauri exercita o mesmo caso de uso @spec:AC-002', () => {
  const graph = json(contract('architecture-graph.json'));
  const matrix = read(contract('AB-001-layer-ownership.md'));
  assert.match(matrix, /fake-adapter/);
  assert.match(matrix, /cli-adapter/);
  assert.ok(graph.edges.some((edge) => edge.from === 'fake-adapter' && edge.to === 'application-api'));
  assert.ok(!graph.edges.some((edge) => edge.from === 'agent-core' && edge.to === 'tauri-shell'));
});

test('AC-003: Edges proibidas falham fechadas @spec:AC-003', () => {
  const fixtures = json(contract('architecture-graph.invalid-fixtures.json'));
  for (const fixture of fixtures) assert.equal(validateArchitectureGraph(fixture.graph).status, fixture.expected_status);
});

test('AC-003: graph validator rejects duplicate IDs, cycles, and undeclared edges @spec:AC-003', () => {
  const graph = json(contract('architecture-graph.json'));
  const duplicate = { ...graph, layers: [...graph.layers, { ...graph.layers[0] }] };
  const cycle = { ...graph, edges: [...graph.edges, { from: 'agent-core', to: 'application-api', kind: 'cycle' }] };
  const undeclared = { ...graph, edges: [...graph.edges, { from: 'tauri-shell', to: 'agent-core', kind: 'bypass' }] };
  assert.equal(validateArchitectureGraph(duplicate).status, 'BLOCKED');
  assert.equal(validateArchitectureGraph(cycle).status, 'BLOCKED');
  assert.equal(validateArchitectureGraph(undeclared).status, 'BLOCKED');
});

// US-002 — Ownership e dependências sem ambiguidade
test('AC-004: Matriz cobre comandos e crates @spec:AC-004', () => {
  const graph = json(contract('architecture-graph.json'));
  const owners = graph.layers.map((layer) => layer.owner);
  assert.equal(new Set(owners).size, graph.layers.length);
  assert.ok(graph.layers.every((layer) => Array.isArray(layer.allowed_dependencies)));
});

test('AC-005: Ciclos e edges inválidas são rejeitados @spec:AC-005', () => {
  const fixtures = json(contract('queue-invalid-fixtures.json'));
  const missing = fixtures.find((fixture) => fixture.id === 'missing-dependency');
  const invalid = fixtures.find((fixture) => fixture.id === 'invalid-category');
  const cycle = fixtures.find((fixture) => fixture.id === 'cycle');
  assert.equal(validateCard(missing.card, missing.known_ids).status, 'BLOCKED');
  assert.equal(validateCard(invalid.card, invalid.known_ids).status, 'BLOCKED');
  const cycleCards = cycle.cards.map((card) => ({ ...card, fields: {} }));
  assert.equal(validateCards(cycleCards).status, 'BLOCKED');
});

test('AC-006: Lifecycle e compatibilidade estão definidos @spec:AC-006', () => {
  const graph = json(contract('architecture-graph.json'));
  assert.ok(graph.layers.every((layer) => layer.process_lifecycle && layer.contract));
  assert.match(read(contract('ADR-AB-001.md')), /Compatibilidade/);
});

// US-003 — Fila de PRs e DAG mecanicamente auditáveis
test('AC-007: Os 270 cards têm schema completo @spec:AC-007', () => {
  const result = validateQueue();
  assert.equal(result.status, 'PASS', result.errors.join('; '));
  assert.equal(result.count, 270);
  assert.deepEqual(result.ids, Array.from({ length: 270 }, (_, index) => `PR-${String(index + 1).padStart(3, '0')}`));
});

test('AC-008: Dependências e labels inválidos falham @spec:AC-008', () => {
  const fixtures = json(contract('queue-invalid-fixtures.json'));
  for (const fixture of fixtures) {
    const result = fixture.cards ? validateCards(fixture.cards.map((card) => ({ ...card, fields: {} }))) : validateCard(fixture.card, fixture.known_ids);
    assert.equal(result.status, fixture.expected_status, fixture.id);
  }
  assert.match(read(contract('queue-validator-contract.md')), /não pode ser normalizada silenciosamente/);
});

test('AC-009: PR-001 e M16 são inequívocos @spec:AC-009', () => {
  const cards = parseQueueCards();
  assert.equal(cards.filter((card) => card.id === 'PR-001').length, 1);
  assert.ok(cards.some((card) => card.id === 'PR-252'));
  assert.ok(cards.some((card) => card.id === 'PR-270'));
  assert.match(read('.planning/master/queue-index.md'), /PR-001/);
  assert.match(read('.planning/master/queue-index.md'), /PR-270/);
});

// US-004 — Execução por agentes com evidência e isolamento
test('AC-010: Preflight captura identidade e escopo @spec:AC-010', () => {
  const schema = json(contract('PR-EXECUTION-CONTRACT.schema.json'));
  for (const field of ['card_id', 'repository', 'branch', 'worktree', 'base_sha', 'tree_sha', 'dirty_state', 'scope', 'allowed_files', 'author', 'reviewer', 'policy_revision', 'schema_revision']) assert.ok(schema.required.includes(field));
  assert.match(read(contract('execution-gate-contract.md')), /Preflight obrigatório/);
});

test('AC-011: Worktree, branch e path allowlist são impostos @spec:AC-011', () => {
  const main = json(contract('execution-invalid-fixtures.json')).find((fixture) => fixture.id === 'main-branch');
  assert.equal(evaluateExecution(main.input).status, 'BLOCKED');
  const scopeDrift = { ...main.input, branch: 'feature/x', scope: ['outside.txt'], allowed_files: ['inside.txt'] };
  assert.equal(evaluateExecution(scopeDrift).status, 'BLOCKED');
});

test('AC-011: execution gate rejects incomplete reviewer and unsafe paths @spec:AC-011', () => {
  const valid = { card_id: 'PR-001', repository: 'stoltembergg-png/hank', branch: 'feature/x', worktree: 'C:/work', base_sha: sha, tree_sha: sha, dirty_state: 'clean', scope: ['inside.txt'], non_goals: [], allowed_files: ['inside.txt'], allowed_commands: [], author: 'agent-a', reviewer: 'reviewer-b', policy_revision: 'p1', schema_revision: 's1', rollback: 'revert' };
  assert.equal(evaluateExecution({ ...valid, reviewer: undefined }).status, 'BLOCKED');
  assert.equal(evaluateExecution({ ...valid, scope: ['../outside.txt'], allowed_files: ['../outside.txt'] }).status, 'BLOCKED');
  assert.equal(evaluateExecution({ ...valid, allowed_files: ['C:/outside.txt'], scope: ['C:/outside.txt'] }).status, 'BLOCKED');
  assert.equal(evaluateExecution({ ...valid, base_sha: 'bad' }).status, 'BLOCKED');
});

test('AC-012: Review independente e anti-self-approval @spec:AC-012', () => {
  const valid = { card_id: 'PR-001', repository: 'stoltembergg-png/hank', branch: 'feature/x', worktree: 'C:/work', base_sha: sha, tree_sha: sha, dirty_state: 'clean', scope: ['inside.txt'], non_goals: [], allowed_files: ['inside.txt'], allowed_commands: [], author: 'agent-a', reviewer: 'reviewer-b', policy_revision: 'p1', schema_revision: 's1', rollback: 'revert' };
  assert.equal(evaluateExecution(valid).status, 'PASS');
  assert.equal(evaluateExecution({ ...valid, reviewer: valid.author }).status, 'BLOCKED');
});

test('AC-013: Evidência stale é invalidada @spec:AC-013', () => {
  const fixture = json(contract('execution-invalid-fixtures.json')).find((item) => item.id === 'stale-policy');
  assert.equal(evaluateExecution(fixture.input, fixture.observed).status, fixture.expected_status);
});

// US-005 — Gate negativo e fechamento honesto
test('AC-014: Fixtures negativas cobrem W0 @spec:AC-014', () => {
  const matrix = json(contract('w0-negative-test-matrix.json'));
  assert.equal(matrix.length, 8);
  assert.deepEqual(new Set(matrix.map((item) => item.blocker)), new Set(['ARCH-001', 'ARCH-002', 'GOV-001', 'GOV-002', 'GOV-003']));
  assert.ok(matrix.every((item) => ['PASS', 'NO_PROOF', 'BLOCKED'].includes(item.expected_status)));
});

test('AC-015: Gate produz estados machine-readable @spec:AC-015', () => {
  const result = evaluateGate({ status: 'INVALID', reason: 'fixture', evidence: { sha, tree: sha, policy: 'p1', schema: 's1' } });
  assert.equal(result.status, 'BLOCKED');
});

test('AC-015: gate requires all five W0 reports and matching identity @spec:AC-015', () => {
  const reports = ['ARCH-001', 'ARCH-002', 'GOV-001', 'GOV-002', 'GOV-003'].map((blocker) => ({ blocker, status: 'PASS', sha, tree: sha, policy: 'p1', schema: 's1' }));
  const base = { status: 'PASS', reason: 'all reports', evidence: { sha, tree: sha, policy: 'p1', schema: 's1' }, reports };
  assert.equal(evaluateGate(base).status, 'PASS');
  assert.equal(evaluateGate({ ...base, reports: reports.slice(0, 4) }).status, 'NO_PROOF');
  assert.equal(evaluateGate({ ...base, reports: reports.map((report) => report.blocker === 'GOV-003' ? { ...report, status: 'BLOCKED' } : report) }).status, 'BLOCKED');
  assert.equal(evaluateGate({ ...base, evidence: { ...base.evidence, tree: 'stale' } }).status, 'NO_PROOF');
});

test('AC-016: Auditoria não declara resolução sem prova @spec:AC-016', () => {
  const plan = read('.planning/master/blocker-closure-plan.md');
  const blockers = plan.match(/^### [A-Z]+-\d{3}$/gm) ?? [];
  assert.equal(blockers.length, 27);
  assert.doesNotMatch(plan, /^- \*\*Status atual:\*\*.*RESOLVED$/gm);
  assert.match(plan, /PARTIAL\/NO_PROOF/);
  assert.match(read(contract('w0-closure-gate.md')), /não liberada|não.*liber/i);
});
