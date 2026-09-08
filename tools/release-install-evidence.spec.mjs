import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { validateInstallSmokeReports } from './release-install-evidence.mjs';

const commit = 'a'.repeat(40);
const tree = 'b'.repeat(40);

function report(platform, overrides = {}) {
  return {
    status: 'passed',
    expectedCommit: commit,
    expectedTree: tree,
    releaseTag: 'v1.0.0',
    platform,
    artifactDigests: { release: 'sha256:release' },
    installerDigest: platform.startsWith('windows-') ? 'a'.repeat(64) : null,
    appImageDigest: platform.startsWith('linux-') ? 'b'.repeat(64) : null,
    dmgDigest: platform.startsWith('macos-') ? 'c'.repeat(64) : null,
    uninstall: platform === 'windows-x86_64' || platform === 'macos-aarch64' ? 'passed' : 'not_applicable_portable',
    upgradeRollback: 'passed',
    ...overrides,
  };
}

test('release promotion accepts complete per-platform install evidence', () => {
  assert.deepEqual(
    validateInstallSmokeReports([
      report('windows-x86_64'),
      report('linux-x86_64'),
      report('macos-aarch64'),
    ], { commit, tree }),
    { status: 'PASS', platforms: ['linux-x86_64', 'macos-aarch64', 'windows-x86_64'] },
  );
});

test('release promotion rejects missing, stale, and limited evidence', () => {
  assert.throws(
    () => validateInstallSmokeReports([report('windows-x86_64')], { commit, tree }),
    /missing install smoke evidence: linux-x86_64/,
  );
  assert.throws(
    () => validateInstallSmokeReports([report('windows-x86_64'), report('linux-x86_64')], { commit, tree }),
    /missing install smoke evidence: macos-aarch64/,
  );
  assert.throws(
    () => validateInstallSmokeReports([
      report('windows-x86_64', { expectedTree: 'c'.repeat(40) }),
      report('linux-x86_64'),
      report('macos-aarch64'),
    ], { commit, tree }),
    /identity mismatch: windows-x86_64/,
  );
  assert.throws(
    () => validateInstallSmokeReports([
      report('windows-x86_64', { upgradeRollback: 'PASS_LIMITED' }),
      report('linux-x86_64'),
      report('macos-aarch64'),
    ], { commit, tree }),
    /upgrade\/rollback evidence is not PASS: windows-x86_64/,
  );
  assert.throws(
    () => validateInstallSmokeReports([
      report('windows-x86_64'),
      report('linux-x86_64'),
      report('macos-aarch64', { dmgDigest: null }),
    ], { commit, tree }),
    /DMG digest is missing: macos-aarch64/,
  );
  assert.throws(
    () => validateInstallSmokeReports([
      report('windows-x86_64', { releaseTag: 'v0.0.1' }),
      report('linux-x86_64'),
      report('macos-aarch64'),
    ], { commit, tree, releaseTag: 'v1.0.0' }),
    /release tag mismatch: windows-x86_64/,
  );
});

test('stable promotion workflow consumes immutable per-platform evidence', () => {
  const root = new URL('../', import.meta.url);
  const workflow = readFileSync(fileURLToPath(new URL('.github/workflows/release-milestone.yml', root)), 'utf8');
  assert.match(workflow, /release-install-smoke-\$\{PRERELEASE_TAG\}/);
  assert.match(workflow, /release-install-smoke-linux-\$\{PRERELEASE_TAG\}/);
  assert.match(workflow, /gh run download/);
  assert.match(workflow, /validateInstallSmokeReports/);
  assert.match(workflow, /requireUpgradeRollback: true/);
});
