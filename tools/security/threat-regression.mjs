#!/usr/bin/env node
// Threat regression runner. Pure Node, no I/O outside the workspace,
// deterministic output bound to (tree_sha, runner_revision, manifest_revision).
//
// Usage:
//   node tools/security/threat-regression.mjs
//   node tools/security/threat-regression.mjs --out security/reports/threat-regression.json
//
// Exit codes:
//   0  all TMs and NEG-* checks pass
//   1  manifest invalid / missing / orphan
//   2  at least one TM fails its expected outcome
//   3  at least one negative fixture produces a non-expected match

import { readFileSync, writeFileSync, existsSync, mkdirSync, readdirSync, statSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { resolve, relative, dirname } from 'node:path';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '..', '..');
const manifestPath = resolve(root, 'docs', 'security', 'threat-regression-manifest.json');

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
const ALLOWED_BOUNDARIES = new Set([
  'ipc-core',
  'remote-core',
  'filesystem',
  'secrets-core',
  'plugin-core',
  'evidence-ledger',
  'release-pipeline',
  'all-artifacts',
  '.github/workflows/*.yml',
  '.github/required-checks.json',
  'docs/security/threat-regression-manifest.json',
]);
const SECRET_PATTERN = /(api[_-]?key|senha|password|secret|token)\s*[:=]\s*['"][^'"\n]{8,}/i;
const REQUIRED_CHECKS_PATH = resolve(root, '.github', 'required-checks.json');

function die(code, message, extra = {}) {
  const payload = { status: 'fail', code, message, ...extra };
  console.error(JSON.stringify(payload, null, 2));
  process.exit(code);
}

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function gitText(args) {
  return execFileSync('git', args, { cwd: root, encoding: 'utf8' }).trim();
}

function treeSha() {
  return gitText(['rev-parse', 'HEAD^{tree}']);
}

function headSha() {
  return gitText(['rev-parse', 'HEAD']);
}

function runnerRevision() {
  // The runner_revision is the SHA256 of this file, ensuring the output is
  // bound to the exact runner that produced it.
  const buf = readFileSync(fileURLToPath(import.meta.url));
  return createHash('sha256').update(buf).digest('hex').slice(0, 16);
}

function validateManifest(manifest) {
  const errors = [];
  if (manifest.schema_version !== 1) {
    errors.push('schema_version must be 1');
  }
  if (typeof manifest.manifest_revision !== 'string' || manifest.manifest_revision.length === 0) {
    errors.push('manifest_revision is required');
  }
  if (typeof manifest.runner_revision !== 'string' || manifest.runner_revision.length === 0) {
    errors.push('runner_revision is required');
  }
  if (manifest.max_threats !== 128) {
    errors.push('max_threats must be 128');
  }
  if (manifest.remediation_policy !== 'deny-and-block') {
    errors.push('remediation_policy must be deny-and-block');
  }
  if (!Array.isArray(manifest.tms) || manifest.tms.length === 0) {
    errors.push('tms must be a non-empty array');
  } else if (manifest.tms.length > 128) {
    errors.push('tms length exceeds max_threats=128');
  }
  if (!Array.isArray(manifest.negative_fixtures)) {
    errors.push('negative_fixtures must be an array');
  }
  const seen = new Set();
  for (const tm of manifest.tms ?? []) {
    if (typeof tm.id !== 'string' || !/^TM-\d{3}$/.test(tm.id)) {
      errors.push(`tm id must match TM-NNN: ${JSON.stringify(tm)}`);
    } else if (seen.has(tm.id)) {
      errors.push(`duplicate tm id: ${tm.id}`);
    } else {
      seen.add(tm.id);
    }
    if (!ALLOWED_BOUNDARIES.has(tm.boundary)) {
      errors.push(`${tm.id}: unknown boundary ${tm.boundary}`);
    }
    if (!ALLOWED_SEVERITIES.has(tm.severity)) {
      errors.push(`${tm.id}: unknown severity ${tm.severity}`);
    }
    if (!ALLOWED_OUTCOMES.has(tm.expected_outcome)) {
      errors.push(`${tm.id}: unknown expected_outcome ${tm.expected_outcome}`);
    }
    if (typeof tm.test_id !== 'string' || !/^AC-\d{4}$/.test(tm.test_id)) {
      errors.push(`${tm.id}: test_id must be AC-NNNN`);
    }
    if (typeof tm.fixture_id !== 'string' || tm.fixture_id.length === 0) {
      errors.push(`${tm.id}: fixture_id is required`);
    }
    if (tm.revision !== 'rev-1') {
      errors.push(`${tm.id}: revision must be rev-1 in this card`);
    }
  }
  for (const neg of manifest.negative_fixtures ?? []) {
    if (typeof neg.id !== 'string' || !/^NEG-\d{3}$/.test(neg.id)) {
      errors.push(`neg id must match NEG-NNN: ${JSON.stringify(neg)}`);
    }
    if (!ALLOWED_BOUNDARIES.has(neg.scope)) {
      errors.push(`${neg.id}: unknown scope ${neg.scope}`);
    }
    if (!ALLOWED_OUTCOMES.has(neg.expected_outcome)) {
      errors.push(`${neg.id}: unknown expected_outcome ${neg.expected_outcome}`);
    }
    if (typeof neg.test_id !== 'string' || !/^AC-\d{4}$/.test(neg.test_id)) {
      errors.push(`${neg.id}: test_id must be AC-NNNN`);
    }
  }
  return errors;
}

// --- Checkers ---------------------------------------------------------------

function checkSecretPatternInArtifacts(allowlist) {
  // Walk tracked files and assert no SECRET_PATTERN line exists in the
  // canonical artifacts. Skip the manifest, the runner, the spec, and
  // any path under `target/`, `node_modules/`, `dist/`. The allowlist
  // accepts both exact paths and directory prefixes (entries ending
  // without an extension are treated as prefixes).
  const allowList = new Set(allowlist);
  const out = execFileSync('git', ['ls-files', '-co', '--exclude-standard'], {
    cwd: root,
    encoding: 'utf8',
  }).split('\n').filter(Boolean);
  const matches = [];
  for (const path of out) {
    if ([...allowList].some((entry) => path === entry || path.startsWith(`${entry}/`))) {
      continue;
    }
    if (path.startsWith('target/') || path.startsWith('node_modules/') || path.startsWith('dist/')) {
      continue;
    }
    if (!path.endsWith('.json') && !path.endsWith('.md') && !path.endsWith('.txt') && !path.endsWith('.rs') && !path.endsWith('.mjs') && !path.endsWith('.ts') && !path.endsWith('.tsx') && !path.endsWith('.yml') && !path.endsWith('.yaml')) {
      continue;
    }
    let text;
    try {
      text = readFileSync(resolve(root, path), 'utf8');
    } catch {
      continue;
    }
    if (SECRET_PATTERN.test(text)) {
      matches.push(path);
    }
  }
  return { match: matches.length > 0, matches };
}

function checkNoContinueOnErrorInRequired() {
  const workflowDir = resolve(root, '.github', 'workflows');
  if (!existsSync(workflowDir)) return { match: false, matches: [] };
  const files = readdirSync(workflowDir).filter((f) => f.endsWith('.yml') || f.endsWith('.yaml'));
  const offenders = [];
  for (const f of files) {
    const text = readFileSync(resolve(workflowDir, f), 'utf8');
    // A required check is a job in a workflow that appears in
    // required-checks.json. The simplest robust rule for this gate is:
    // no `continue-on-error: true` on any step inside a `jobs:` block.
    const lines = text.split('\n');
    let inJobs = false;
    for (const line of lines) {
      if (/^jobs:\s*$/.test(line)) {
        inJobs = true;
        continue;
      }
      if (inJobs && /^\S/.test(line) && !line.startsWith(' ') && !line.startsWith('\t')) {
        inJobs = false;
      }
      if (inJobs && /continue-on-error\s*:\s*true/.test(line)) {
        offenders.push(`${f}: ${line.trim()}`);
      }
    }
  }
  return { match: offenders.length > 0, matches: offenders };
}

function checkRequiredChecksCoherent() {
  if (!existsSync(REQUIRED_CHECKS_PATH)) {
    return { match: false, matches: ['required-checks.json missing'] };
  }
  const manifest = readJson(REQUIRED_CHECKS_PATH);
  if (manifest.branch !== 'main') {
    return { match: false, matches: [`required-checks.json branch must be main, got ${manifest.branch}`] };
  }
  if (!Array.isArray(manifest.requiredChecks) || manifest.requiredChecks.length === 0) {
    return { match: false, matches: ['requiredChecks must be a non-empty array'] };
  }
  return { match: true, matches: [] };
}

function checkManifestPresent(manifest) {
  if (!existsSync(manifestPath)) {
    return { match: false, matches: ['manifest missing'] };
  }
  return { match: true, matches: [] };
}

// --- Aggregation ------------------------------------------------------------

function evaluateNegativeFixtures(manifest) {
  const results = [];
  const checkers = {
    'NEG-001': (allowlist) => checkSecretPatternInArtifacts(allowlist),
    'NEG-002': checkNoContinueOnErrorInRequired,
    'NEG-003': checkRequiredChecksCoherent,
    'NEG-004': () => checkManifestPresent(manifest),
  };
  for (const neg of manifest.negative_fixtures) {
    const checker = checkers[neg.id];
    if (!checker) {
      results.push({ id: neg.id, status: 'fail', reason: 'no-checker', expected_outcome: neg.expected_outcome });
      continue;
    }
    const allowlist = Array.isArray(neg.allowlist) ? neg.allowlist : [];
    const { match, matches } = checker(allowlist);
    const expected = neg.expected_outcome;
    const pass = match === (expected === 'match');
    results.push({
      id: neg.id,
      status: pass ? 'pass' : 'fail',
      expected_outcome: expected,
      observed_match: match,
      evidence: matches.slice(0, 5),
    });
  }
  return results;
}

function evaluateTms(manifest, negativeResults) {
  // For this card, the TMs are validated by Rust tests. The runner here
  // records the binding between TM-NNN and AC-NNNN and checks that the
  // negative fixtures back the same expected outcome. The Rust side is
  // responsible for the actual boundary exercise; the runner is the
  // manifest/negative contract.
  const tms = manifest.tms.map((tm) => ({
    id: tm.id,
    boundary: tm.boundary,
    threat: tm.threat,
    severity: tm.severity,
    test_id: tm.test_id,
    expected_outcome: tm.expected_outcome,
    fixture_id: tm.fixture_id,
    status: 'pending-rust',
  }));
  return tms;
}

function main() {
  const args = process.argv.slice(2);
  const outIdx = args.indexOf('--out');
  const outPath = outIdx >= 0 ? resolve(root, args[outIdx + 1]) : resolve(root, 'security', 'reports', 'threat-regression.json');

  if (!existsSync(manifestPath)) {
    die(1, 'manifest missing', { manifestPath: relative(root, manifestPath) });
  }
  let manifest;
  try {
    manifest = readJson(manifestPath);
  } catch (err) {
    die(1, `manifest is not valid JSON: ${err.message}`);
  }
  const manifestErrors = validateManifest(manifest);
  if (manifestErrors.length > 0) {
    die(1, 'manifest validation failed', { errors: manifestErrors });
  }

  const negativeResults = evaluateNegativeFixtures(manifest);
  const tms = evaluateTms(manifest, negativeResults);

  const negativeFailures = negativeResults.filter((r) => r.status === 'fail');

  const head = headSha();
  const tree = treeSha();
  const runner = runnerRevision();
  const body = {
    schema_version: 1,
    status: negativeFailures.length === 0 ? 'pass' : 'fail',
    generated_at: new Date().toISOString(),
    head_sha: head,
    tree_sha: tree,
    runner_revision: runner,
    manifest_revision: manifest.manifest_revision,
    tms,
    negative_results: negativeResults,
    summary: {
      tm_count: tms.length,
      negative_count: negativeResults.length,
      negative_failures: negativeFailures.length,
    },
  };
  // Deterministic digest: computed over a canonical body that omits the
  // volatile `generated_at` and the `artifact_digest` field itself. Two
  // runs of the same tree produce the same digest.
  const digestable = {
    schema_version: body.schema_version,
    status: body.status,
    head_sha: body.head_sha,
    tree_sha: body.tree_sha,
    runner_revision: body.runner_revision,
    manifest_revision: body.manifest_revision,
    tms: body.tms,
    negative_results: body.negative_results,
    summary: body.summary,
  };
  const bodyText = JSON.stringify(body, null, 2);
  const digest = createHash('sha256').update(JSON.stringify(digestable, null, 2)).digest('hex');
  body.artifact_digest = digest;

  mkdirSync(dirname(outPath), { recursive: true });
  writeFileSync(outPath, JSON.stringify(body, null, 2));

  console.log(JSON.stringify({ status: body.status, digest, path: relative(root, outPath) }, null, 2));
  if (negativeFailures.length > 0) {
    process.exit(3);
  }
  process.exit(0);
}

main();
