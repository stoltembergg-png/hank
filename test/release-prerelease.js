import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  assertPostMergeChecks,
  assertPublishPermission,
  assertTagAvailable,
  buildManifest,
  buildPrereleaseTag,
  buildRollbackPlan,
  classifyCommits,
  decideIdempotentRelease,
  policyDecision,
  renderReleaseNotes,
  verifyVersionConsistency,
} from '../tools/release-prerelease.mjs';

const sha = 'a'.repeat(40);
const tree = 'b'.repeat(40);
const base = {
  cargoToml: 'version = "0.1.0"',
  desktopCargoToml: 'version = "0.1.0"',
  frontendPackage: { version: '0.1.0' },
  tauriConfig: { version: '0.1.0' },
  releaseManifest: { version: '0.1.0' },
  appSource: "export const APP_VERSION = '0.1.0';",
};

function tag() { return buildPrereleaseTag({ baseVersion: '0.1.0', sha }); }

test('AC-624: generates deterministic unique SemVer prerelease tag @spec:AC-624', () => {
  assert.equal(tag(), `v0.1.0-dev.${sha}`);
  assert.match(tag(), /^v0\.1\.0-dev\.[0-9a-f]{40}$/);
  assert.notEqual(tag(), buildPrereleaseTag({ baseVersion: '0.1.0', sha: 'c'.repeat(40) }));
});

test('AC-624/627: rejects invalid version and tag reuse @spec:AC-624 @spec:AC-627', () => {
  assert.throws(() => buildPrereleaseTag({ baseVersion: 'latest', sha }), /invalid release version/);
  assert.throws(() => buildPrereleaseTag({ baseVersion: '0.1.0-rc.1', sha }), /stable semver/);
  assert.throws(() => assertTagAvailable(tag(), [tag()]), /already exists/);
});

test('AC-625: rejects divergence across Cargo, frontend, Tauri, release manifest, and app display @spec:AC-625', () => {
  assert.deepEqual(verifyVersionConsistency(base), {
    cargo: '0.1.0', desktopCargo: '0.1.0', frontend: '0.1.0', tauri: '0.1.0', releaseManifest: '0.1.0', app: '0.1.0',
  });
  assert.throws(() => verifyVersionConsistency({ ...base, tauriConfig: { version: '0.1.1' } }), /divergence/);
});

test('AC-626: fails closed on missing or failed post-merge checks @spec:AC-626', () => {
  const checks = [
    { name: 'Build Rust', status: 'completed', conclusion: 'success' },
    { name: 'Build Frontend', status: 'completed', conclusion: 'failure' },
  ];
  assert.throws(() => assertPostMergeChecks({ checks, required: ['Build Rust', 'Build Frontend'] }), /not successful/);
  assert.throws(() => assertPostMergeChecks({ checks: checks.slice(0, 1), required: ['Build Rust', 'ONP SDD'] }), /missing/);
  assert.throws(() => assertPostMergeChecks({ checks: [{ name: 'Build Rust', status: 'queued', conclusion: null }], required: ['Build Rust'] }), /not successful/);
});

test('AC-627: rerun is idempotent only for the exact existing release @spec:AC-627', () => {
  assert.deepEqual(decideIdempotentRelease({ tagExists: false, releaseExists: false }), { action: 'create' });
  assert.deepEqual(decideIdempotentRelease({ tagExists: true, releaseExists: true, existingTarget: sha, expectedSha: sha, existingManifestDigest: 'x', expectedManifestDigest: 'x' }), { action: 'noop', reason: 'matching release already exists' });
  assert.throws(() => decideIdempotentRelease({ tagExists: true, releaseExists: false }), /without release/);
  assert.throws(() => decideIdempotentRelease({ tagExists: true, releaseExists: true, existingTarget: 'c'.repeat(40), expectedSha: sha }), /different commit/);
});

test('AC-630: publication requires least privilege and refuses permission failure @spec:AC-630', () => {
  assert.equal(assertPublishPermission({ contents: 'write' }), true);
  assert.throws(() => assertPublishPermission({ contents: 'read' }), /contents: write/);
  assert.throws(() => assertPublishPermission({ contents: undefined }), /contents: write/);
});

test('AC-630: workflow scopes write permission to publish and pins actions @spec:AC-630', () => {
  const workflow = readFileSync('.github/workflows/release-prerelease.yml', 'utf8');
  assert.match(workflow, /permissions:\s+contents: read/);
  assert.match(workflow, /publish:[\s\S]*?permissions:\s+contents: write/);
  assert.equal([...workflow.matchAll(/permissions:\s+contents: write/g)].length, 1);
  assert.doesNotMatch(workflow, /release-dry-run/);
  for (const action of workflow.matchAll(/uses:\s+([^\s#]+)@([^\s#]+)/g)) {
    assert.match(action[2], /^[0-9a-f]{40}$/);
  }
});

test('AC-629: classifies changes and applies release policy @spec:AC-629', () => {
  assert.deepEqual(classifyCommits(['feat(core): add x', 'fix: repair y']), ['functional']);
  assert.deepEqual(classifyCommits(['docs: explain x', 'ci: pin action']), ['CI', 'documentation']);
  assert.deepEqual(policyDecision(['functional']), { publish: true, reason: 'functional update' });
  assert.equal(policyDecision(['documentation']).publish, false);
  assert.equal(policyDecision(['documentation'], { publishDocumentation: true }).publish, true);
});

test('AC-628: generates immutable manifest and changelog with test instructions @spec:AC-628', () => {
  const releaseTag = tag();
  const changelog = '- feat(core): add prerelease pipeline';
  const instructions = 'Verify release-manifest.json SHA, then run cargo test --workspace --locked.';
  const manifest = buildManifest({ tag: releaseTag, version: '0.1.0-dev.' + sha, sha, tree, card: 'PR-200', classification: ['functional'], relatedPullRequests: [200], artifacts: ['hank.tar.gz'], changelog, testInstructions: instructions });
  assert.equal(manifest.prerelease, true);
  assert.equal(manifest.provenance.exactCommit, sha);
  assert.deepEqual(JSON.parse(JSON.stringify(manifest)), manifest);
  const notes = renderReleaseNotes({ tag: releaseTag, sha, card: 'PR-200', classification: ['functional'], changelog, testInstructions: instructions, relatedPullRequests: [200] });
  assert.match(notes, /Prerelease/);
  assert.match(notes, /PR-200/);
  assert.match(notes, /cargo test/);
});

test('AC-631: rollback is explicit, bounded, and does not silently delete anything @spec:AC-631', () => {
  assert.deepEqual(buildRollbackPlan({ tag: tag(), releaseId: 42, sha }), { tag: tag(), releaseId: '42', sha, action: 'delete-release-and-tag', destructive: true, requiresExplicitApproval: true });
  assert.throws(() => buildRollbackPlan({ tag: 'v0.1.0', releaseId: 42, sha }), /valid immutable/);
});
