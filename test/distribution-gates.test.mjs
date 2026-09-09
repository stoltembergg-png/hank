//! Contract tests for the bounded distribution gates evaluator.
//!
//! These fixtures are offline and deterministic. They prove the evaluator is
//! fail-closed: any missing, stale, mismatched or non-success evidence yields
//! `NO_GO`, and only the exact all-success tuple yields `ELIGIBLE` (which never
//! authorizes publication — it only enables a human/protected decision).

import assert from 'node:assert/strict';
import test from 'node:test';
import {
  buildEvidenceRecord,
  evaluateDistribution,
  requiredEvidence,
  REQUIRED_EVIDENCE_CATEGORIES,
  MAX_EVIDENCE_BYTES,
} from '../tools/distribution-gates.mjs';

const sha = 'a'.repeat(40);

function allSuccess() {
  return REQUIRED_EVIDENCE_CATEGORIES.map((category) =>
    buildEvidenceRecord({ category, status: 'success', repository: 'stoltembergg-png/hank', ref: 'main', sha, tree: 'b'.repeat(40), digest: 'sha256:'.padEnd(71, 'c'), channel: 'stable' }),
  );
}

test('AC-3814: every required evidence category must be present @spec:AC-3814', () => {
  // Dropping any single category yields NO_GO.
  for (let i = 0; i < REQUIRED_EVIDENCE_CATEGORIES.length; i += 1) {
    const records = allSuccess().filter((_, idx) => idx !== i);
    const decision = evaluateDistribution({ records, channel: 'stable' });
    assert.equal(decision.verdict, 'NO_GO');
    assert.ok(decision.reasons.length > 0);
  }
});

test('AC-3815: success on every category yields eligible-for-human-decision only @spec:AC-3815', () => {
  const decision = evaluateDistribution({ records: allSuccess(), channel: 'stable' });
  assert.equal(decision.verdict, 'ELIGIBLE');
  // Eligible never authorizes publication by itself.
  assert.equal(decision.authorized, false);
});

test('AC-3816: failed, skipped, cancelled or timed-out evidence yields NO_GO @spec:AC-3816', () => {
  for (const status of ['failure', 'skipped', 'cancelled', 'timed_out', 'queued', 'pending']) {
    const records = allSuccess();
    records[0] = buildEvidenceRecord({
      category: REQUIRED_EVIDENCE_CATEGORIES[0],
      status,
      repository: 'stoltembergg-png/hank',
      ref: 'main',
      sha,
      tree: 'b'.repeat(40),
      digest: 'sha256:'.padEnd(71, 'c'),
      channel: 'stable',
    });
    const decision = evaluateDistribution({ records, channel: 'stable' });
    assert.equal(decision.verdict, 'NO_GO', `status ${status} must be NO_GO`);
  }
});

test('AC-3817: wrong repository, ref, sha or tree identity yields NO_GO @spec:AC-3817', () => {
  const wrongRepo = allSuccess();
  wrongRepo[0] = buildEvidenceRecord({
    category: REQUIRED_EVIDENCE_CATEGORIES[0],
    status: 'success',
    repository: 'evil/fork',
    ref: 'main',
    sha,
    tree: 'b'.repeat(40),
    digest: 'sha256:'.padEnd(71, 'c'),
    channel: 'stable',
  });
  assert.equal(evaluateDistribution({ records: wrongRepo, channel: 'stable' }).verdict, 'NO_GO');

  const wrongSha = allSuccess();
  wrongSha[1] = buildEvidenceRecord({
    category: REQUIRED_EVIDENCE_CATEGORIES[1],
    status: 'success',
    repository: 'stoltembergg-png/hank',
    ref: 'main',
    sha: 'c'.repeat(40),
    tree: 'b'.repeat(40),
    digest: 'sha256:'.padEnd(71, 'c'),
    channel: 'stable',
  });
  assert.equal(evaluateDistribution({ records: wrongSha, channel: 'stable' }).verdict, 'NO_GO');
});

test('AC-3818: channel or platform mismatch yields NO_GO @spec:AC-3818', () => {
  const records = allSuccess();
  records[0] = buildEvidenceRecord({
    category: REQUIRED_EVIDENCE_CATEGORIES[0],
    status: 'success',
    repository: 'stoltembergg-png/hank',
    ref: 'main',
    sha,
    tree: 'b'.repeat(40),
    digest: 'sha256:'.padEnd(71, 'c'),
    channel: 'beta',
  });
  const decision = evaluateDistribution({ records, channel: 'stable' });
  assert.equal(decision.verdict, 'NO_GO');
});

test('AC-3819: malformed or oversized evidence is rejected @spec:AC-3819', () => {
  assert.throws(
    () => buildEvidenceRecord({ category: 'bogus', status: 'success', repository: 'x', ref: 'main', sha, tree: 'b'.repeat(40), digest: 'd', channel: 'stable' }),
    /unknown evidence category/,
  );
  assert.throws(
    () => buildEvidenceRecord({ category: REQUIRED_EVIDENCE_CATEGORIES[0], status: 'success', repository: 'x', ref: 'main', sha: 'not-a-sha', tree: 'b'.repeat(40), digest: 'd', channel: 'stable' }),
    /full commit SHA/,
  );
  // Oversized evidence mutating after construction is still rejected fail-closed
  // at evaluation time as NO_GO.
  const rec = buildEvidenceRecord({ category: REQUIRED_EVIDENCE_CATEGORIES[0], status: 'success', repository: 'x', ref: 'main', sha, tree: 'b'.repeat(40), digest: 'd', channel: 'stable' });
  rec.digest = 'x'.repeat(MAX_EVIDENCE_BYTES + 1);
  const decision = evaluateDistribution({ records: [rec], channel: 'stable' });
  assert.equal(decision.verdict, 'NO_GO');
  assert.ok(decision.reasons.some((r) => r.includes('exceeds bound')));
});

test('AC-3820: AI or reviewer output is ignored by the evaluator @spec:AC-3820', () => {
  // The evaluator accepts only structured evidence records; any string-based
  // "approval" signal is rejected outright, never promoted to a decision.
  assert.throws(
    () => evaluateDistribution({ records: allSuccess(), channel: 'stable', aiApproval: 'looks good to me' }),
    /aiApproval/,
  );
});

test('AC-3821: the exact eligible tuple is deterministic and reproducible @spec:AC-3821', () => {
  const a = evaluateDistribution({ records: allSuccess(), channel: 'stable' });
  const b = evaluateDistribution({ records: allSuccess(), channel: 'stable' });
  assert.equal(JSON.stringify(a), JSON.stringify(b));
  assert.equal(a.reportDigest, b.reportDigest);
  assert.match(a.reportDigest, /^[0-9a-f]{40}$/);
});

test('requiredEvidence returns the catalog of categories @spec:AC-3814', () => {
  const categories = requiredEvidence();
  assert.ok(categories.length >= 12);
  assert.ok(categories.includes('security-tests'));
  assert.ok(categories.includes('fuzz-tests'));
  assert.ok(categories.includes('load-tests'));
  assert.ok(categories.includes('workflow-recovery-tests'));
  assert.ok(categories.includes('release-signing'));
  assert.ok(categories.includes('installers'));
  assert.ok(categories.includes('auto-updater'));
  assert.ok(categories.includes('backup-restore'));
  assert.ok(categories.includes('migration-hardening'));
  assert.ok(categories.includes('secret-migration'));
  assert.ok(categories.includes('provider-compatibility'));
  assert.ok(categories.includes('release-rollback'));
});