//! Bounded, fail-closed distribution gates evaluator (PR-270).
//!
//! Aggregates quality, security, compatibility, signing, installer, updater
//! and recovery evidence into a single distribution decision. The evaluator is
//! deterministic and offline: it accepts only structured evidence records and
//! yields `NO_GO` on any missing, stale, mismatched, oversized or non-success
//! evidence; the exact all-success tuple yields `ELIGIBLE`, which never
//! authorizes publication — it only enables a human/protected release decision.
//!
//! AI or reviewer prose is deliberately rejected as an input; only
//! machine-readable evidence is considered.

import { createHash } from 'node:crypto';

const HEX_SHA = /^[0-9a-f]{40}$/;
const CHANNELS = new Set(['stable', 'beta', 'canary']);

/** Maximum bytes accepted for a single evidence field value. */
export const MAX_EVIDENCE_BYTES = 256;

/** The evidence categories required for a distribution decision. */
export const REQUIRED_EVIDENCE_CATEGORIES = [
  'security-tests',
  'fuzz-tests',
  'load-tests',
  'workflow-recovery-tests',
  'agent-loop-tests',
  'provider-compatibility',
  'backup-restore',
  'migration-hardening',
  'secret-migration',
  'release-signing',
  'installers',
  'auto-updater',
  'release-rollback',
];

const CATEGORY_SET = new Set(REQUIRED_EVIDENCE_CATEGORIES);

/** Returns the catalog of required evidence categories. */
export function requiredEvidence() {
  return [...REQUIRED_EVIDENCE_CATEGORIES];
}

/**
 * Builds and validates a single structured evidence record.
 *
 * @param {{category: string, status: string, repository: string, ref: string,
 *   sha: string, tree: string, digest: string, channel: string}} value
 * @returns {object} the normalized record.
 */
export function buildEvidenceRecord({
  category,
  status,
  repository,
  ref,
  sha,
  tree,
  digest,
  channel,
}) {
  if (!CATEGORY_SET.has(category)) {
    throw new Error(`unknown evidence category: ${category}`);
  }
  if (!HEX_SHA.test(sha ?? '')) {
    throw new Error('release identity requires full commit SHA');
  }
  if (!HEX_SHA.test(tree ?? '')) {
    throw new Error('release identity requires full tree SHA');
  }
  if (!CHANNELS.has(channel)) {
    throw new Error(`unknown release channel: ${channel}`);
  }
  for (const value of [repository, ref, digest]) {
    if (typeof value !== 'string' || value.length === 0 || value.length > MAX_EVIDENCE_BYTES) {
      throw new Error('evidence exceeds bound');
    }
  }
  return {
    category,
    status: String(status ?? ''),
    repository,
    ref,
    sha,
    tree,
    digest,
    channel,
  };
}

/**
 * Evaluates a set of evidence records against a target distribution.
 *
 * @param {{records: object[], channel: string, aiApproval?: any}} input
 * @returns {{verdict: 'NO_GO'|'ELIGIBLE', authorized: false, reasons: string[],
 *   reportDigest: string}}
 */
export function evaluateDistribution({ records, channel, aiApproval }) {
  // AI/reviewer prose is never an acceptable approval input.
  if (aiApproval !== undefined) {
    throw new Error('aiApproval is not a valid distribution input');
  }
  if (!CHANNELS.has(channel)) {
    return { verdict: 'NO_GO', authorized: false, reasons: [`unknown channel: ${channel}`], reportDigest: '' };
  }

  const reasons = [];
  const byCategory = new Map();
  for (const record of records) {
    if (!CATEGORY_SET.has(record.category)) {
      reasons.push(`unknown evidence category: ${record.category}`);
      continue;
    }
    // Defense in depth: reject oversized or malformed fields even when a
    // record was mutated after construction.
    if (!HEX_SHA.test(record.sha ?? '')) {
      reasons.push(`invalid sha: ${record.category}`);
    }
    if (!HEX_SHA.test(record.tree ?? '')) {
      reasons.push(`invalid tree: ${record.category}`);
    }
    if (!CHANNELS.has(record.channel)) {
      reasons.push(`unknown channel: ${record.category}`);
    }
    for (const field of ['repository', 'ref', 'digest']) {
      const value = record[field];
      if (typeof value !== 'string' || value.length === 0 || value.length > MAX_EVIDENCE_BYTES) {
        reasons.push(`evidence exceeds bound: ${record.category}`);
      }
    }
    // Duplicate categories are malformed, not redundant.
    if (byCategory.has(record.category)) {
      reasons.push(`duplicate evidence category: ${record.category}`);
    }
    byCategory.set(record.category, record);
  }

  for (const category of REQUIRED_EVIDENCE_CATEGORIES) {
    if (!byCategory.has(category)) {
      reasons.push(`missing evidence: ${category}`);
    }
  }

  // Identity, status and channel checks apply to every present record.
  for (const record of byCategory.values()) {
    if (record.repository !== 'stoltembergg-png/hank') {
      reasons.push(`repository mismatch: ${record.category}`);
    }
    if (record.ref !== 'main') {
      reasons.push(`ref mismatch: ${record.category}`);
    }
    if (record.sha !== records[0]?.sha) {
      reasons.push(`sha mismatch: ${record.category}`);
    }
    if (record.tree !== records[0]?.tree) {
      reasons.push(`tree mismatch: ${record.category}`);
    }
    if (record.channel !== channel) {
      reasons.push(`channel mismatch: ${record.category}`);
    }
    if (record.status !== 'success') {
      reasons.push(`non-success status (${record.status}): ${record.category}`);
    }
  }

  const verdict = reasons.length === 0 ? 'ELIGIBLE' : 'NO_GO';
  const reportDigest = verdict === 'ELIGIBLE'
    ? createHash('sha1').update(JSON.stringify({ records, channel })).digest('hex')
    : '';

  return { verdict, authorized: false, reasons, reportDigest };
}

// CLI: print a JSON distribution decision for a manifest file.
if (process.argv[1] && process.argv[1].endsWith('distribution-gates.mjs') && process.argv[2] === 'evaluate') {
  const { readFileSync } = await import('node:fs');
  const input = JSON.parse(readFileSync(process.argv[3], 'utf8'));
  const decision = evaluateDistribution(input);
  process.stdout.write(JSON.stringify(decision));
  process.stdout.write('\n');
}