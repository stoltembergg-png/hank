#!/usr/bin/env node
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const [, , baselinePath, ...reportSpecs] = process.argv;
if (!baselinePath || reportSpecs.length === 0) {
  console.error('usage: check-rust-advisories.mjs <baseline.json> <report.json::tree.txt>...');
  process.exit(2);
}

const baseline = JSON.parse(readFileSync(baselinePath, 'utf8'));
assert.equal(baseline.schema_version, 1, 'unsupported baseline schema');
assert.equal(baseline.exceptions.length, 1, 'baseline must contain exactly one exception');
const exception = baseline.exceptions[0];
assert.equal(exception.advisory, 'RUSTSEC-2024-0429');
assert.deepEqual(exception.aliases, ['GHSA-wrw7-89jp-8q8g']);
assert.equal(exception.dependency, 'glib');
assert.equal(exception.classification, 'UPSTREAM_TRANSITIVE_DEPENDENCY');
assert.equal(exception.status, 'OPEN / ACCEPTED UPSTREAM RISK');
assert.equal(exception.mitigation_status, 'NO_COMPATIBLE_UPSTREAM_FIX_AVAILABLE');

const allowed = new Set([exception.advisory, ...exception.aliases]);
const findings = new Map();
const unreachableFindings = new Map();
let auditFailures = 0;
let observedFindings = 0;

for (const spec of reportSpecs) {
  const [reportPath, treePath] = spec.split('::');
  assert.ok(reportPath && treePath, `invalid report spec: ${spec}`);
  const report = JSON.parse(readFileSync(reportPath, 'utf8'));
  const reachable = new Set();
  const packagePattern = /([A-Za-z0-9][A-Za-z0-9_-]*) v([0-9][^\s()]*)/g;
  for (const line of readFileSync(treePath, 'utf8').split('\n')) {
    for (const match of line.matchAll(packagePattern)) reachable.add(`${match[1]}@${match[2]}`);
  }
  const statusPath = `${reportPath}.exit`;
  const auditStatus = Number.parseInt(readFileSync(statusPath, 'utf8').trim(), 10);
  assert.ok(Number.isInteger(auditStatus), `${statusPath}: invalid cargo-audit exit status`);
  auditFailures += auditStatus === 0 ? 0 : 1;

  const collect = (entries) => {
    for (const entry of entries ?? []) {
      const id = entry?.advisory?.id;
      const pkg = entry?.package;
      if (typeof id !== 'string' || typeof pkg?.name !== 'string' || typeof pkg?.version !== 'string') continue;
      observedFindings += 1;
      const target = reachable.has(`${pkg.name}@${pkg.version}`) ? findings : unreachableFindings;
      target.set(id, (target.get(id) ?? 0) + 1);
    }
  };
  collect(report.vulnerabilities?.list);
  collect(report.warnings?.unsound);
  const unmaintained = report.warnings?.unmaintained?.length ?? 0;
  console.log(`INFORMATIONAL_UNMAINTAINED_WARNINGS: ${unmaintained}`);
}

const unknown = [...findings.keys()].filter((id) => !allowed.has(id));
if (unknown.length > 0) {
  console.error(`NEW_SECURITY_REGRESSIONS: ${unknown.join(', ')}`);
  process.exit(1);
}
if (findings.size === 0 && auditFailures > 0 && observedFindings === 0) {
  console.error('cargo-audit failed without a classified advisory; refusing baseline acceptance');
  process.exit(1);
}

const known = [...findings.entries()].map(([id, count]) => `${id} (${count})`).join(', ') || 'none';
const unreachable = [...unreachableFindings.entries()].map(([id, count]) => `${id} (${count})`).join(', ') || 'none';
console.log(`KNOWN_UPSTREAM_RISKS: ${known}`);
console.log(`UNREACHABLE_AUDIT_FINDINGS: ${unreachable}`);
console.log(`NEW_SECURITY_REGRESSIONS: 0`);
console.log(`AUDIT_REPORTS: ${reportSpecs.length}`);
console.log('SECURITY_GATE: PASS_WITH_EXPLICIT_UPSTREAM_EXCEPTION');
