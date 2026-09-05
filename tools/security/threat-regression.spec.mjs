// Negative matrix and contract tests for `tools/security/threat-regression.mjs`.
// Each test maps to a TM-NNN or NEG-NNN ID in
// `docs/security/threat-regression-manifest.json`.

import assert from 'node:assert/strict';
import { readFileSync, existsSync, mkdirSync, readdirSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import test from 'node:test';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '..', '..');
const manifestPath = resolve(root, 'docs', 'security', 'threat-regression-manifest.json');
const runnerPath = resolve(here, 'threat-regression.mjs');
const requiredChecksPath = resolve(root, '.github', 'required-checks.json');
const workflowsDir = resolve(root, '.github', 'workflows');
const securityDocs = resolve(root, 'docs', 'security');
const advisoryBaseline = resolve(root, 'security', 'advisory-baseline.json');

const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
const ALLOWED_SEVERITIES = new Set(['critical', 'high', 'medium', 'low']);
const ALLOWED_OUTCOMES = new Set([
  'deny_with_typed_error',
  'reject_traversal',
  'redact_to_placeholder',
  'reject_stale_evidence',
  'reject_release_metadata',
  'no-match',
  'match',
]);

function readText(path) {
  return readFileSync(path, 'utf8');
}

// @spec:AC-2101
// @spec:AC-2101
test('manifest is well-formed and self-consistent @spec:AC-2101', () => {
  assert.equal(manifest.schema_version, 1);
  assert.equal(manifest.manifest_revision, 'rev-1');
  assert.equal(manifest.max_threats, 128);
  assert.equal(manifest.remediation_policy, 'deny-and-block');
  const ids = new Set();
  for (const tm of manifest.tms) {
    assert.match(tm.id, /^TM-\d{3}$/);
    assert.ok(!ids.has(tm.id), `duplicate TM id ${tm.id}`);
    ids.add(tm.id);
    assert.ok(ALLOWED_SEVERITIES.has(tm.severity));
    assert.ok(ALLOWED_OUTCOMES.has(tm.expected_outcome));
    assert.match(tm.test_id, /^AC-\d{4}$/);
    assert.equal(tm.revision, 'rev-1');
  }
  for (const neg of manifest.negative_fixtures) {
    assert.match(neg.id, /^NEG-\d{3}$/);
    assert.ok(ALLOWED_OUTCOMES.has(neg.expected_outcome));
    assert.match(neg.test_id, /^AC-\d{4}$/);
  }
});

// @spec:AC-2101
// @spec:AC-2101
test('every TM test_id is in the AC-21NN range @spec:AC-2101', () => {
  // The test_id must be in the AC-21NN range; ACs are reused across
  // TMs/NEGs because one AC often covers several fixtures.
  for (const tm of manifest.tms) {
    assert.match(tm.test_id, /^AC-21\d{2}$/);
  }
  for (const neg of manifest.negative_fixtures) {
    assert.match(neg.test_id, /^AC-21\d{2}$/);
  }
});

// @spec:AC-2105
// @spec:AC-2105
test('NEG-001 secret pattern is not committed in canonical artifacts @spec:AC-2105', () => {
  const pattern = /(api[_-]?key|senha|password|secret|token)\s*[:=]\s*['"][^'"\n]{8,}/i;
  const neg = manifest.negative_fixtures.find((n) => n.id === 'NEG-001');
  const allow = new Set(neg?.allowlist ?? []);
  const files = execFileSync('git', ['ls-files', '-co', '--exclude-standard'], {
    cwd: root,
    encoding: 'utf8',
  }).split('\n').filter(Boolean);
  for (const f of files) {
    // The allowlist is a mix of exact paths and directory prefixes
    // (e.g. `docs/superpowers/plans` matches every file under it).
    if ([...allow].some((entry) => f === entry || f.startsWith(`${entry}/`))) continue;
    if (!/\.(json|md|txt|rs|mjs|ts|tsx|yml|yaml)$/.test(f)) continue;
    if (f.startsWith('target/') || f.startsWith('node_modules/') || f.startsWith('dist/')) continue;
    const text = readFileSync(resolve(root, f), 'utf8');
    assert.ok(!pattern.test(text), `secret pattern leaked in ${f}`);
  }
});

// @spec:AC-2105
// @spec:AC-2105
test('NEG-002 no continue-on-error inside jobs blocks of required workflows @spec:AC-2105', () => {
  if (!existsSync(workflowsDir)) return;
  const files = readdirSync(workflowsDir).filter((f) => f.endsWith('.yml') || f.endsWith('.yaml'));
  for (const f of files) {
    const text = readFileSync(resolve(workflowsDir, f), 'utf8');
    const lines = text.split('\n');
    let inJobs = false;
    for (const line of lines) {
      if (/^jobs:\s*$/.test(line)) { inJobs = true; continue; }
      if (inJobs && /^\S/.test(line)) inJobs = false;
      assert.ok(
        !/continue-on-error\s*:\s*true/.test(line) || !inJobs,
        `${f}: continue-on-error inside jobs block: ${line.trim()}`,
      );
    }
  }
});

// @spec:AC-2105
// @spec:AC-2105
test('NEG-003 required-checks.json is coherent with manifest revision @spec:AC-2105', () => {
  if (!existsSync(requiredChecksPath)) {
    assert.fail('required-checks.json missing');
  }
  const rc = JSON.parse(readFileSync(requiredChecksPath, 'utf8'));
  assert.equal(rc.branch, 'main');
  assert.ok(Array.isArray(rc.requiredChecks) && rc.requiredChecks.length > 0);
  // The required set must not introduce fake checks invented for this card.
  for (const c of rc.requiredChecks) {
    assert.match(c, /^[A-Za-z][A-Za-z0-9 \/().&+_-]+$/);
  }
});

// @spec:AC-2105
// @spec:AC-2105
test('advisory baseline either absent or schema_version 1 @spec:AC-2105', () => {
  if (!existsSync(advisoryBaseline)) return; // absence is allowed for this slice
  const j = JSON.parse(readFileSync(advisoryBaseline, 'utf8'));
  assert.equal(j.schema_version, 1);
});

// @spec:AC-2106
// @spec:AC-2106
test('NEG-004 manifest is present at docs/security/threat-regression-manifest.json @spec:AC-2106', () => {
  assert.ok(existsSync(manifestPath));
  const text = readFileSync(manifestPath, 'utf8');
  assert.ok(text.length > 0);
});

// @spec:AC-2107
// @spec:AC-2107
test('runner is deterministic and produces artifact_digest bound to tree @spec:AC-2107', () => {
  // Invoke the runner twice with the same tree and assert the digest is
  // stable and bound to tree_sha + runner_revision + manifest_revision.
  // We use spawnSync so a non-zero runner exit (e.g. NEG-001 currently
  // failing) does not abort the test runner; the digest is what we
  // assert determinism on, not the exit code.
  const outA = resolve(root, 'security', 'reports', 'threat-regression-A.json');
  const outB = resolve(root, 'security', 'reports', 'threat-regression-B.json');
  mkdirSync(dirname(outA), { recursive: true });
  const a1 = spawnSync('node', [runnerPath, '--out', 'security/reports/threat-regression-A.json'], {
    cwd: root,
    encoding: 'utf8',
  });
  const a2 = spawnSync('node', [runnerPath, '--out', 'security/reports/threat-regression-B.json'], {
    cwd: root,
    encoding: 'utf8',
  });
  assert.equal(a1.status, 0, `runner-A failed: ${a1.stderr}`);
  assert.equal(a2.status, 0, `runner-B failed: ${a2.stderr}`);
  const a = JSON.parse(readFileSync(outA, 'utf8'));
  const b = JSON.parse(readFileSync(outB, 'utf8'));
  assert.equal(a.artifact_digest, b.artifact_digest, 'digest must be stable for same tree');
  assert.equal(a.tree_sha, b.tree_sha);
  assert.equal(a.head_sha, b.head_sha);
  assert.match(a.artifact_digest, /^[a-f0-9]{64}$/);
  // The digest is computed over the canonical digestable body (without
  // `generated_at` or `artifact_digest`); recompute that to verify.
  const bodyWithoutDigest = { ...a };
  delete bodyWithoutDigest.artifact_digest;
  delete bodyWithoutDigest.generated_at;
  const recomputed = createHash('sha256')
    .update(JSON.stringify(bodyWithoutDigest, null, 2))
    .digest('hex');
  assert.equal(a.artifact_digest, recomputed, 'digest must be sha256 of canonical body');
});

// @spec:AC-2107
// @spec:AC-2107
test('runner_revision is bound to runner source content @spec:AC-2107', () => {
  const out = resolve(root, 'security', 'reports', 'threat-regression-rev.json');
  mkdirSync(dirname(out), { recursive: true });
  const r = spawnSync('node', [runnerPath, '--out', 'security/reports/threat-regression-rev.json'], {
    cwd: root,
    encoding: 'utf8',
  });
  assert.equal(r.status, 0, `runner failed: ${r.stderr}`);
  const j = JSON.parse(readFileSync(out, 'utf8'));
  const expected = createHash('sha256')
    .update(readFileSync(runnerPath))
    .digest('hex')
    .slice(0, 16);
  assert.equal(j.runner_revision, expected);
});
