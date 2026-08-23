#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

const HEX_SHA = /^[0-9a-f]{40}$/;
const SEMVER = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/;
const TAG = /^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)-dev\.[0-9a-f]{40}$/;
const TYPES = new Map([
  ['feat', 'functional'], ['fix', 'functional'], ['perf', 'functional'], ['refactor', 'functional'],
  ['docs', 'documentation'], ['ci', 'CI'], ['build', 'dependency'], ['chore', 'dependency'],
  ['deps', 'dependency'], ['dependabot', 'dependency'], ['test', 'functional'],
]);

export function normalizeVersion(value) {
  const raw = String(value ?? '').replace(/^v/, '');
  if (!SEMVER.test(raw)) throw new Error(`invalid release version: ${value}`);
  return raw;
}

export function buildPrereleaseTag({ baseVersion, sha }) {
  const version = normalizeVersion(baseVersion);
  if (version.includes('-') || version.includes('+')) throw new Error('base version must be stable semver');
  if (!HEX_SHA.test(sha ?? '')) throw new Error('release identity requires full commit SHA');
  return `v${version}-dev.${sha}`;
}

export function assertTagAvailable(tag, existingTags = []) {
  if (!TAG.test(tag)) throw new Error(`invalid prerelease tag: ${tag}`);
  if (existingTags.includes(tag)) throw new Error(`release tag already exists: ${tag}`);
  return true;
}

function extractJsonVersion(json, source) {
  if (!json || typeof json.version !== 'string') throw new Error(`manifest version missing: ${source}`);
  return normalizeVersion(json.version);
}

export function verifyVersionConsistency({ cargoToml, desktopCargoToml, frontendPackage, tauriConfig, releaseManifest, appSource }) {
  const cargo = cargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
  const desktop = desktopCargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
  const app = appSource.match(/APP_VERSION\s*=\s*['"]([^'"]+)['"]/)?.[1];
  const versions = {
    cargo: normalizeVersion(cargo),
    desktopCargo: normalizeVersion(desktop),
    frontend: extractJsonVersion(frontendPackage, 'frontend/package.json'),
    tauri: extractJsonVersion(tauriConfig, 'tauri.conf.json'),
    releaseManifest: extractJsonVersion(releaseManifest, 'release-manifest.json'),
    app: normalizeVersion(app),
  };
  const unique = new Set(Object.values(versions));
  if (unique.size !== 1) throw new Error(`version manifest divergence: ${JSON.stringify(versions)}`);
  return versions;
}

export function assertPostMergeChecks({ checks, required }) {
  const byName = new Map(checks.map((check) => [check.name, check]));
  for (const name of required) {
    const check = byName.get(name);
    if (!check) throw new Error(`post-merge check missing: ${name}`);
    if (check.status !== 'completed' || check.conclusion !== 'success') {
      throw new Error(`post-merge check not successful: ${name} (${check.status}/${check.conclusion ?? 'pending'})`);
    }
  }
  return true;
}

export function assertPublishPermission({ contents }) {
  if (contents !== 'write') throw new Error('release publication requires contents: write permission');
  return true;
}

export function classifyCommits(commits) {
  const classes = new Set();
  for (const subject of commits) {
    const type = subject.match(/^([a-z]+)(?:\([^)]*\))?!?:/i)?.[1]?.toLowerCase();
    classes.add(TYPES.get(type) ?? 'functional');
  }
  return [...classes].sort();
}

export function policyDecision(classes, { publishDocumentation = false, publishCi = false, publishDependencies = false } = {}) {
  if (classes.includes('functional')) return { publish: true, reason: 'functional update' };
  if (classes.includes('documentation') && publishDocumentation) return { publish: true, reason: 'documentation policy enabled' };
  if (classes.includes('CI') && publishCi) return { publish: true, reason: 'CI policy enabled' };
  if (classes.includes('dependency') && publishDependencies) return { publish: true, reason: 'dependency policy enabled' };
  return { publish: false, reason: 'policy excludes non-functional update' };
}

export function decideIdempotentRelease({ tagExists, releaseExists, existingTarget, expectedSha, existingManifestDigest, expectedManifestDigest }) {
  if (!tagExists && !releaseExists) return { action: 'create' };
  if (tagExists && !releaseExists) throw new Error('tag exists without release; refusing to overwrite or reuse tag');
  if (existingTarget !== expectedSha) throw new Error('existing release points to a different commit');
  if (existingManifestDigest && existingManifestDigest !== expectedManifestDigest) throw new Error('existing release manifest differs');
  return { action: 'noop', reason: 'matching release already exists' };
}

export function buildRollbackPlan({ tag, releaseId, sha }) {
  if (!TAG.test(tag) || !HEX_SHA.test(sha ?? '')) throw new Error('rollback requires valid immutable release identity');
  return { tag, releaseId: String(releaseId), sha, action: 'delete-release-and-tag', destructive: true, requiresExplicitApproval: true };
}

export function renderReleaseNotes({ tag, sha, card, classification, changelog, testInstructions, relatedPullRequests = [] }) {
  if (!TAG.test(tag) || !HEX_SHA.test(sha ?? '') || !/^PR-\d+$/.test(card)) throw new Error('release provenance is incomplete');
  return [
    `# ${tag}`,
    '',
    '**Prerelease — not a stable release.**',
    '',
    `- Exact main commit: \`${sha}\``,
    `- Logical card: ${card}`,
    `- Classification: ${classification.join(', ')}`,
    `- Related PRs: ${relatedPullRequests.length ? relatedPullRequests.map((n) => `#${n}`).join(', ') : 'none'}`,
    '',
    '## Changelog', '', changelog.trim(), '',
    '## Test this version', '', testInstructions.trim(), '',
  ].join('\n');
}

export function buildManifest({ tag, version, sha, tree, card, classification, relatedPullRequests, artifacts, changelog, testInstructions }) {
  const normalized = normalizeVersion(version);
  if (!HEX_SHA.test(sha ?? '') || !HEX_SHA.test(tree ?? '')) throw new Error('release identity requires full commit and tree SHA');
  if (!TAG.test(tag) || tag !== `v${normalized}`) throw new Error('manifest tag/version mismatch');
  if (!/^PR-\d+$/.test(card)) throw new Error('manifest requires logical PR card');
  if (!Array.isArray(artifacts) || artifacts.length === 0) throw new Error('manifest requires downloadable artifacts');
  return {
    schemaVersion: 1, tag, version: normalized, prerelease: true, stable: false, sha, tree,
    card, classification, relatedPullRequests, artifacts, changelog, testInstructions,
    provenance: { source: 'main', exactCommit: sha, tagImmutable: true },
  };
}

export function milestoneVersion({ config, milestone }) {
  if (!config || config.schemaVersion !== 1 || !Array.isArray(config.milestones)) {
    throw new Error('milestone configuration is invalid');
  }
  const entry = config.milestones.find((candidate) => candidate.id === milestone);
  if (!entry) throw new Error(`milestone is not configured: ${milestone}`);
  return normalizeVersion(entry.version);
}

export function buildMilestoneReleaseManifest({ manifest, stableVersion, milestone }) {
  const version = normalizeVersion(stableVersion);
  if (!manifest || manifest.prerelease !== true || manifest.stable === true) {
    throw new Error('manifest is not a prerelease');
  }
  if (manifest.version !== `${version}-dev.${manifest.sha}`) {
    throw new Error('prerelease manifest version does not match stable version');
  }
  if (manifest.tag !== `v${manifest.version}`) throw new Error('prerelease manifest tag/version mismatch');
  if (!HEX_SHA.test(manifest.sha ?? '') || !HEX_SHA.test(manifest.tree ?? '')) {
    throw new Error('release identity requires full commit and tree SHA');
  }
  if (!Array.isArray(manifest.artifacts) || manifest.artifacts.length === 0) {
    throw new Error('stable promotion requires downloadable artifacts');
  }
  if (!/^M\d+(?:-M\d+)?$/.test(milestone ?? '')) throw new Error('invalid milestone identifier');
  const prereleaseTag = manifest.tag;
  const stableTag = `v${version}`;
  return {
    ...manifest,
    tag: stableTag,
    version,
    prerelease: false,
    stable: true,
    milestone,
    artifacts: manifest.artifacts.map((artifact) => artifact.replaceAll(prereleaseTag, stableTag)),
    provenance: {
      ...manifest.provenance,
      source: 'main',
      exactCommit: manifest.sha,
      tagImmutable: true,
      promotedFromTag: prereleaseTag,
    },
  };
}

function git(args) { return execFileSync('git', args, { encoding: 'utf8' }).trim(); }
function arg(name) { const i = process.argv.indexOf(name); return i >= 0 ? process.argv[i + 1] : null; }

function main() {
  const command = process.argv[2];
  if (command === 'tag') {
    process.stdout.write(`${buildPrereleaseTag({ baseVersion: arg('--version'), sha: arg('--sha') })}\n`);
    return;
  }
  if (command === 'verify-version') {
    const root = process.cwd();
    const read = (path) => readFileSync(`${root}/${path}`, 'utf8');
    const versions = verifyVersionConsistency({
      cargoToml: read('Cargo.toml'), desktopCargoToml: read('apps/desktop/src-tauri/Cargo.toml'),
      frontendPackage: JSON.parse(read('frontend/package.json')), tauriConfig: JSON.parse(read('apps/desktop/src-tauri/tauri.conf.json')),
      releaseManifest: JSON.parse(read('release-manifest.json')), appSource: read('frontend/src/version.ts'),
    });
    process.stdout.write(`${JSON.stringify(versions)}\n`);
    return;
  }
  if (command === 'checks') {
    const checks = JSON.parse(readFileSync(arg('--file'), 'utf8'));
    assertPostMergeChecks({ checks, required: (arg('--required') ?? '').split(',').filter(Boolean) });
    process.stdout.write('post-merge checks: PASS\n');
    return;
  }
  if (command === 'classify') {
    const subjects = readFileSync(arg('--file'), 'utf8').split('\n').filter(Boolean);
    process.stdout.write(`${JSON.stringify(classifyCommits(subjects))}\n`);
    return;
  }
  if (command === 'manifest') {
    const input = JSON.parse(readFileSync(arg('--input'), 'utf8'));
    const output = JSON.stringify(buildManifest(input), null, 2) + '\n';
    const destination = arg('--output');
    if (destination) writeFileSync(destination, output);
    else process.stdout.write(output);
    return;
  }
  if (command === 'promote-manifest') {
    const input = JSON.parse(readFileSync(arg('--input'), 'utf8'));
    const output = JSON.stringify(buildMilestoneReleaseManifest({
      manifest: input,
      stableVersion: arg('--version'),
      milestone: arg('--milestone'),
    }), null, 2) + '\n';
    const destination = arg('--output');
    if (destination) writeFileSync(destination, output);
    else process.stdout.write(output);
    return;
  }
  if (command === 'milestone-version') {
    const config = JSON.parse(readFileSync(arg('--config') ?? 'release-milestones.json', 'utf8'));
    process.stdout.write(`${milestoneVersion({ config, milestone: arg('--milestone') })}\n`);
    return;
  }
  if (command === 'changelog') {
    const sha = git(['rev-parse', 'HEAD']);
    const subjects = git(['log', '-20', '--format=%s']).split('\n').filter(Boolean);
    process.stdout.write(renderReleaseNotes({ tag: arg('--tag'), sha, card: arg('--card'), classification: classifyCommits(subjects), changelog: subjects.map((s) => `- ${s}`).join('\n'), testInstructions: 'Download the release artifact, verify the manifest SHA, and run the documented checks.', relatedPullRequests: (arg('--prs') ?? '').split(',').filter(Boolean) }));
    return;
  }
  throw new Error('usage: tag|verify-version|checks|classify|manifest|promote-manifest|milestone-version|changelog');
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) main();
