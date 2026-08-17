import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const ROOT = process.cwd();
const contracts = path.join(ROOT, '.planning', 'contracts');
const queueFiles = [
  '.planning/queue/queue-001-095.md',
  '.planning/queue/queue-096-172.md',
  '.planning/queue/queue-173-270.md'
];

const readText = (relative) => fs.readFileSync(path.join(ROOT, relative), 'utf8');
const readJson = (relative) => JSON.parse(readText(relative));

const requiredQueueFields = [
  ['ID'], ['Categoria'], ['Milestone'], ['Título'], ['Objetivo'],
  ['Problema resolvido'], ['Escopo'], ['Não-escopo'], ['Arquivos'],
  ['Dependências anteriores'], ['Requisitos funcionais'], ['NFRs'],
  ['Critérios de aceite verificáveis'], ['Testes obrigatórios'],
  ['Verificações de segurança'], ['Observabilidade'], ['Documentação'],
  ['Definition of Done'], ['Condição para desbloquear']
];

function fieldValue(block, aliases) {
  const lines = block.split(/\r?\n/);
  for (const line of lines) {
    const match = line.match(/^\s*-\s*\*\*([^*]+):\*\*\s*(.*)$/);
    if (!match) continue;
    const key = match[1].trim();
    if (aliases.some((alias) => key === alias || key.startsWith(alias))) return match[2].trim();
  }
  return '';
}

export function parseQueueCards() {
  const cards = [];
  for (const relative of queueFiles) {
    const text = readText(relative);
    const headings = [...text.matchAll(/^###\s+(PR-\d{3})(?:\s+—\s+(.+))?$/gm)];
    for (let i = 0; i < headings.length; i += 1) {
      const start = headings[i].index;
      const end = i + 1 < headings.length ? headings[i + 1].index : text.length;
      const block = text.slice(start, end);
      const fields = Object.fromEntries(requiredQueueFields.map((aliases) => [aliases[0], fieldValue(block, aliases)]));
      const deps = [...(fields['Dependências anteriores'].matchAll(/PR-\d{3}/g))].map((m) => m[0]);
      cards.push({
        id: headings[i][1],
        title: (headings[i][2] ?? '').trim(),
        fields,
        dependencies: deps,
        source: relative
      });
    }
  }
  return cards;
}

function cycleExists(cards) {
  const graph = new Map(cards.map((card) => [card.id, card.dependencies]));
  const visiting = new Set();
  const visited = new Set();
  const visit = (id) => {
    if (visiting.has(id)) return true;
    if (visited.has(id)) return false;
    visiting.add(id);
    for (const dep of graph.get(id) ?? []) if (visit(dep)) return true;
    visiting.delete(id);
    visited.add(id);
    return false;
  };
  return cards.some((card) => visit(card.id));
}

export function validateCards(cards) {
  const errors = [];
  const ids = cards.map((card) => card.id);
  const known = new Set(ids);
  if (new Set(ids).size !== ids.length) errors.push('duplicate card id');
  for (const card of cards) {
    for (const aliases of requiredQueueFields) if (!card.fields?.[aliases[0]] && card[aliases[0]] === undefined) errors.push(`${card.id}: missing ${aliases[0]}`);
    for (const dep of card.dependencies ?? []) if (!known.has(dep)) errors.push(`${card.id}: missing dependency ${dep}`);
  }
  if (cycleExists(cards)) errors.push('dependency cycle detected');
  return { status: errors.length ? 'BLOCKED' : 'PASS', errors };
}

export function validateQueue(cards = parseQueueCards()) {
  const errors = [];
  const ids = cards.map((card) => card.id);
  const expected = Array.from({ length: 270 }, (_, index) => `PR-${String(index + 1).padStart(3, '0')}`);
  if (cards.length !== 270) errors.push(`expected 270 cards, got ${cards.length}`);
  if (new Set(ids).size !== ids.length) errors.push('duplicate card id');
  if (expected.some((id, index) => ids[index] !== id)) errors.push('ids are not exactly PR-001..PR-270 in source order');
  const known = new Set(ids);
  const categories = new Set(['FOUNDATION', 'DOMAIN', 'INFRA', 'BACKEND', 'FRONTEND', 'SECURITY', 'TESTING', 'DEVOPS', 'DOCUMENTATION']);
  for (const card of cards) {
    for (const aliases of requiredQueueFields) if (!card.fields[aliases[0]]) errors.push(`${card.id}: missing ${aliases[0]}`);
    const category = card.fields.Categoria;
    if (category && !categories.has(category)) errors.push(`${card.id}: invalid category ${category}`);
    for (const dep of card.dependencies) if (!known.has(dep)) errors.push(`${card.id}: missing dependency ${dep}`);
  }
  if (cycleExists(cards)) errors.push('dependency cycle detected');
  return { status: errors.length ? 'BLOCKED' : 'PASS', errors, count: cards.length, ids };
}

export function validateCard(card, knownIds = []) {
  const errors = [];
  const required = ['id', 'category', 'milestone', 'title', 'objective', 'problem', 'scope', 'non_scope', 'files', 'functional_requirements', 'nfrs', 'acceptance', 'tests', 'security', 'observability', 'documentation', 'definition_of_done', 'unlock_condition'];
  for (const field of required) if (card[field] === undefined || card[field] === '' || (Array.isArray(card[field]) && card[field].length === 0)) errors.push(`missing ${field}`);
  if (!/^PR-\d{3}$/.test(card.id ?? '')) errors.push('invalid id');
  if (!new Set(['FOUNDATION', 'DOMAIN', 'INFRA', 'BACKEND', 'FRONTEND', 'SECURITY', 'TESTING', 'DEVOPS', 'DOCUMENTATION']).has(card.category)) errors.push('invalid category');
  for (const dep of card.dependencies ?? []) if (!knownIds.includes(dep)) errors.push(`missing dependency ${dep}`);
  return { status: errors.length ? 'BLOCKED' : 'PASS', errors };
}

function graphHasCycle(edges) {
  const adjacency = new Map();
  for (const edge of edges) {
    if (!adjacency.has(edge.from)) adjacency.set(edge.from, []);
    adjacency.get(edge.from).push(edge.to);
  }
  const visiting = new Set();
  const visited = new Set();
  const visit = (id) => {
    if (visiting.has(id)) return true;
    if (visited.has(id)) return false;
    visiting.add(id);
    for (const next of adjacency.get(id) ?? []) if (visit(next)) return true;
    visiting.delete(id);
    visited.add(id);
    return false;
  };
  return [...adjacency.keys()].some((id) => visit(id));
}

export function validateArchitectureGraph(graph = readJson('.planning/contracts/architecture-graph.json')) {
  const errors = [];
  const layers = graph.layers ?? [];
  const ids = new Set(layers.map((layer) => layer.id));
  const owners = new Set();
  if (ids.size !== layers.length) errors.push('duplicate layer id');
  if (!Array.isArray(graph.allowed_edges)) errors.push('allowed_edges missing');
  const allowedEdges = new Set((graph.allowed_edges ?? []).map((edge) => `${edge.from}->${edge.to}`));
  for (const layer of layers) {
    for (const key of ['id', 'owner', 'responsibility', 'process_lifecycle', 'contract']) if (!layer[key]) errors.push(`layer missing ${key}`);
    if (owners.has(layer.owner)) errors.push(`duplicate owner ${layer.owner}`);
    owners.add(layer.owner);
    for (const dependency of layer.allowed_dependencies ?? []) if (!ids.has(dependency) && dependency !== 'ports/application contracts') errors.push(`unknown dependency ${dependency}`);
  }
  for (const edge of graph.edges ?? []) {
    if (!ids.has(edge.from) || !ids.has(edge.to)) errors.push(`unknown edge ${edge.from}->${edge.to}`);
    if (!allowedEdges.has(`${edge.from}->${edge.to}`)) errors.push(`undeclared edge ${edge.from}->${edge.to}`);
    if ((graph.forbidden_edges ?? []).some((bad) => bad.from === edge.from && bad.to === edge.to)) errors.push(`forbidden edge ${edge.from}->${edge.to}`);
  }
  if (graphHasCycle(graph.edges ?? [])) errors.push('architecture edge cycle detected');
  return { status: errors.length ? 'BLOCKED' : 'PASS', errors };
}

const shaPattern = /^[0-9a-f]{40}$/;

function isSafeRelativePath(value) {
  if (typeof value !== 'string' || value.length === 0) return false;
  if (value.startsWith('/') || value.startsWith('\\') || /^[A-Za-z]:[\\/]/.test(value)) return false;
  return !value.split(/[\\/]/).includes('..');
}

export function evaluateExecution(input, observed = null) {
  if (input.branch === 'main' || input.branch === 'master') return { status: 'BLOCKED', reason: 'protected branch' };
  if (!input.author || !input.reviewer) return { status: 'BLOCKED', reason: 'review identity incomplete' };
  if (input.author === input.reviewer) return { status: 'BLOCKED', reason: 'self approval' };
  if (!shaPattern.test(input.base_sha ?? '') || !shaPattern.test(input.tree_sha ?? '') || !input.policy_revision || !input.schema_revision) return { status: 'BLOCKED', reason: 'execution identity incomplete' };
  if (observed && (input.base_sha !== observed.base_sha || input.tree_sha !== observed.tree_sha || input.policy_revision !== observed.policy_revision || input.schema_revision !== observed.schema_revision)) return { status: 'NO_PROOF', reason: 'stale evidence identity' };
  if (!input.worktree || !input.allowed_files?.length || input.dirty_state !== 'clean') return { status: 'BLOCKED', reason: 'preflight incomplete' };
  if (input.allowed_files.some((file) => !isSafeRelativePath(file)) || (input.scope ?? []).some((file) => !isSafeRelativePath(file))) return { status: 'BLOCKED', reason: 'unsafe path' };
  if ((input.scope ?? []).some((file) => !(input.allowed_files ?? []).includes(file))) return { status: 'BLOCKED', reason: 'scope outside allowlist' };
  return { status: 'PASS', reason: 'contract fields accepted' };
}

export function evaluateGate(input) {
  const allowed = new Set(['PASS', 'FAIL', 'BLOCKED', 'NO_PROOF']);
  if (!allowed.has(input.status)) return { status: 'BLOCKED', reason: 'invalid gate state' };
  const identity = input.evidence ?? {};
  if (!input.reason || !identity.sha || !identity.tree || !identity.policy || !identity.schema) return { status: 'NO_PROOF', reason: 'missing reason or evidence identity' };
  if (input.status !== 'PASS') return { status: input.status, reason: input.reason };
  const required = ['ARCH-001', 'ARCH-002', 'GOV-001', 'GOV-002', 'GOV-003'];
  if (!Array.isArray(input.reports) || input.reports.length !== required.length) return { status: 'NO_PROOF', reason: 'W0 reports incomplete' };
  const reports = new Map(input.reports.map((report) => [report.blocker, report]));
  for (const blocker of required) {
    const report = reports.get(blocker);
    if (!report) return { status: 'NO_PROOF', reason: `${blocker} report missing` };
    if (report.sha !== identity.sha || report.tree !== identity.tree || report.policy !== identity.policy || report.schema !== identity.schema) return { status: 'NO_PROOF', reason: `${blocker} report identity stale` };
    if (report.status === 'BLOCKED') return { status: 'BLOCKED', reason: `${blocker} report blocked` };
    if (report.status === 'FAIL') return { status: 'FAIL', reason: `${blocker} report failed` };
    if (report.status !== 'PASS') return { status: 'NO_PROOF', reason: `${blocker} report incomplete` };
  }
  return { status: 'PASS', reason: input.reason };
}

if (process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) {
  const command = process.argv[2];
  const result = command === 'queue' ? validateQueue() : command === 'architecture' ? validateArchitectureGraph() : { status: 'BLOCKED', errors: ['unknown command'] };
  console.log(JSON.stringify(result, null, 2));
  process.exitCode = result.status === 'PASS' ? 0 : 1;
}
