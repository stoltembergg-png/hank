import assert from 'node:assert/strict';
import fs from 'node:fs';
import test from 'node:test';
import { validateArchitectureGraph } from './w0-contract-validator.mjs';

const read = (file) => fs.readFileSync(file, 'utf8');

test('architecture document names the normative graph and boundaries', () => {
  const docs = read('ARCHITECTURE.md');
  assert.match(docs, /architecture-graph\.json/);
  for (const term of ['agent-core', 'application-api', 'agent-runtime', 'infrastructure', 'tauri-shell', 'cli-adapter', 'fake-adapter']) {
    assert.match(docs, new RegExp(`\\b${term}\\b`));
  }
  assert.match(docs, /must not import Tauri|must not depend on Tauri/);
});

test('documented graph and negative fixtures remain valid', () => {
  const graph = JSON.parse(read('.planning/contracts/architecture-graph.json'));
  const fixtures = JSON.parse(read('.planning/contracts/architecture-graph.invalid-fixtures.json'));
  assert.equal(validateArchitectureGraph(graph).status, 'PASS');
  for (const fixture of fixtures) assert.equal(validateArchitectureGraph(fixture.graph).status, fixture.expected_status, fixture.id);
});
