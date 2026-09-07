import assert from 'node:assert/strict';
import test from 'node:test';
import { buildReport, readManifest, renderSvg, validateManifest } from './evidence-scope-contract.mjs';

const manifest = readManifest();
const summary = { plan: 3, passed: 3, failed: 0, skipped: 0 };
const identity = { sourceCommit: 'a'.repeat(40), sourceTree: 'b'.repeat(40), generatedAt: '2026-09-07T00:00:00.000Z' };

// @spec:AC-3001
 test('AC-3001: manifest has exactly three explicit contract/production states @spec:AC-3001', () => {
  assert.equal(validateManifest(manifest), true);
  assert.deepEqual(manifest.entries.map((entry) => entry.logicalCard), ['PR-261', 'PR-266', 'PR-267']);
  for (const entry of manifest.entries) {
    assert.equal(entry.contractStatus, 'PASS');
    assert.equal(entry.productionStatus, 'NO_PROOF');
  }
});

// @spec:AC-3002
 test('AC-3002: PR-261 records synthetic targets and no production parser proof @spec:AC-3002', () => {
  const entry = manifest.entries.find((candidate) => candidate.logicalCard === 'PR-261');
  assert.equal(entry.contractBoundary, 'offline-synthetic');
  assert.match(entry.provenClaims.join('\n'), /synthetic/i);
  assert.match(entry.notProven.join('\n'), /production parser/i);
});

// @spec:AC-3003
 test('AC-3003: PR-266 records ephemeral synthetic keys and no real signing proof @spec:AC-3003', () => {
  const entry = manifest.entries.find((candidate) => candidate.logicalCard === 'PR-266');
  assert.equal(entry.contractBoundary, 'offline-synthetic-key-fixtures');
  assert.match(entry.provenClaims.join('\n'), /synthetic key/i);
  assert.match(entry.notProven.join('\n'), /real release artifact/i);
});

// @spec:AC-3004
 test('AC-3004: PR-267 records contract-only platform support and no native installer proof @spec:AC-3004', () => {
  const entry = manifest.entries.find((candidate) => candidate.logicalCard === 'PR-267');
  assert.equal(entry.contractBoundary, 'contract-only-platform-matrix');
  assert.match(entry.notProven.join('\n'), /real installer/i);
  assert.match(entry.notProven.join('\n'), /platform/i);
});

// @spec:AC-3005
 test('AC-3005: report requires exact commit/tree and complete non-skipped results @spec:AC-3005', () => {
  const report = buildReport({ manifest, ...identity, testSummary: summary });
  assert.equal(report.sourceCommit, identity.sourceCommit);
  assert.equal(report.sourceTree, identity.sourceTree);
  assert.throws(() => buildReport({ manifest, ...identity, testSummary: { ...summary, skipped: 1 } }), /one passing result/);
  assert.throws(() => buildReport({ manifest, ...identity, sourceCommit: 'short', testSummary: summary }), /full Git commit/);
  for (const invalid of ['c'.repeat(41), 'd'.repeat(63)]) {
    assert.throws(() => buildReport({ manifest, ...identity, sourceTree: invalid, testSummary: summary }), /full Git commit/);
  }
});

// @spec:AC-3006
 test('AC-3006: visual report exposes PASS/NO_PROOF and a production boundary warning @spec:AC-3006', () => {
  const svg = renderSvg(buildReport({ manifest, ...identity, testSummary: summary }));
  assert.match(svg, /NOT PRODUCTION PROOF/);
  assert.match(svg, /PR-261/);
  assert.match(svg, /PR-266/);
  assert.match(svg, /PR-267/);
  assert.match(svg, /CONTRACT: PASS/);
  assert.match(svg, /PRODUCTION: NO_PROOF/);
});
