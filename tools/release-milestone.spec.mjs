import assert from 'node:assert/strict';
import { generateKeyPairSync } from 'node:crypto';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import test from 'node:test';
import {
  buildArtifactDigests,
  buildManifest,
  buildMilestoneReleaseManifest,
} from './release-prerelease.mjs';
import { signArtifacts, verifyArtifacts } from './release-artifact-signing.mjs';

const commit = 'a'.repeat(40);
const tree = 'b'.repeat(40);
const prereleaseTag = `v1.0.0-dev.${commit}`;
const stableTag = 'v1.0.0';

function identity(channel, policy, workflow) {
  return {
    repository: 'stoltembergg-png/hank',
    event: 'workflow_dispatch',
    ref: 'refs/heads/main',
    commit,
    tree,
    workflow,
    policy,
    channel,
    os: 'multi',
  };
}

function writeFixture(directory, tag) {
  const names = [
    `hank-${tag}.tar.gz`,
    `hank-${tag}-setup.exe`,
    `hank-${tag}-x86_64.AppImage`,
    'SBOM.spdx.json',
  ];
  names.forEach((name, index) => writeFileSync(join(directory, name), `artifact-${index}`));
  return names;
}

test('stable promotion maps the manifest and requires fresh stable attestations', () => {
  const root = mkdtempSync(join(tmpdir(), 'hank-release-milestone-'));
  const prerelease = join(root, 'prerelease');
  const stable = join(root, 'stable');
  const stale = join(root, 'stale');
  for (const directory of [prerelease, stable, stale]) mkdirSync(directory);
  try {
    const prereleaseNames = writeFixture(prerelease, prereleaseTag);
    const signedNames = prereleaseNames.filter((name) => name !== 'SBOM.spdx.json');
    const manifest = buildManifest({
      tag: prereleaseTag,
      version: `1.0.0-dev.${commit}`,
      sha: commit,
      tree,
      card: 'PR-270',
      classification: ['functional'],
      relatedPullRequests: [270],
      artifacts: [
        ...prereleaseNames,
        ...signedNames.map((name) => `${name}.attestation.json`),
        'release-signing-metadata.json',
        'RELEASE_NOTES.md',
        'TEST_INSTRUCTIONS.md',
        'SHA256SUMS',
      ],
      artifactDigests: buildArtifactDigests({ directory: prerelease, names: prereleaseNames }),
      changelog: 'fixture',
      testInstructions: 'fixture',
    });
    const promoted = buildMilestoneReleaseManifest({
      manifest,
      stableVersion: '1.0.0',
      milestone: 'M16',
    });
    assert.equal(promoted.stable, true);
    assert.equal(promoted.prerelease, false);
    assert.equal(promoted.tag, stableTag);
    assert.equal(promoted.artifactDigests[`hank-${stableTag}.tar.gz`], manifest.artifactDigests[prereleaseNames[0]]);

    const stableNames = signedNames.map((name) => name.replace(prereleaseTag, stableTag));
    for (const [source, destination] of signedNames.map((name, index) => [name, stableNames[index]])) {
      writeFileSync(join(stable, destination), readFileSync(join(prerelease, source)));
      writeFileSync(join(stale, destination), readFileSync(join(prerelease, source)));
    }
    const { privateKey, publicKey } = generateKeyPairSync('ed25519');
    signArtifacts({
      directory: stale,
      attestationDirectory: stale,
      names: stableNames,
      identity: identity('prerelease', 'release-prerelease-v1', 'Publish testable prerelease'),
      signerKeyId: 'release-key-v1',
      privateKey,
    });
    for (const name of stableNames) {
      writeFileSync(join(stable, `${name}.attestation.json`), readFileSync(join(stale, `${name}.attestation.json`)));
    }
    writeFileSync(join(stable, 'release-signing-metadata.json'), readFileSync(join(stale, 'release-signing-metadata.json')));
    assert.throws(() => verifyArtifacts({
      directory: stable,
      attestationDirectory: stable,
      names: stableNames,
      identity: identity('stable', 'release-stable-v1', 'Publish stable milestone release'),
      signerKeyId: 'release-key-v1',
      publicKey,
    }), /workflow mismatch|policy mismatch|channel mismatch/);

    signArtifacts({
      directory: stable,
      attestationDirectory: stable,
      names: stableNames,
      identity: identity('stable', 'release-stable-v1', 'Publish stable milestone release'),
      signerKeyId: 'release-key-v1',
      privateKey,
    });
    assert.deepEqual(verifyArtifacts({
      directory: stable,
      attestationDirectory: stable,
      names: stableNames,
      identity: identity('stable', 'release-stable-v1', 'Publish stable milestone release'),
      signerKeyId: 'release-key-v1',
      publicKey,
    }), { verified: 3, artifacts: [...stableNames].sort() });
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
