#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync, statSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const HEX_SHA = /^[0-9a-f]{40}$/;
const SEMVER = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/;
const TAG = /^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)-dev\.[0-9a-f]{40}$/;
const DIGEST = /^[0-9a-f]{64}$/;
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

export function assertRequiredCheckCoverage({ protectedNames, configuredNames }) {
  const protectedSet = new Set(protectedNames);
  const configuredSet = new Set(configuredNames);
  const missing = protectedNames.filter((name) => !configuredSet.has(name));
  const extra = configuredNames.filter((name) => !protectedSet.has(name));
  if (missing.length || extra.length || protectedSet.size !== configuredSet.size) {
    throw new Error(`required check coverage mismatch: ${JSON.stringify({ missing, extra })}`);
  }
  return true;
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

function validateArtifactDigests(artifacts, artifactDigests) {
  if (artifactDigests === undefined) return undefined;
  if (!artifactDigests || typeof artifactDigests !== 'object' || Array.isArray(artifactDigests)) {
    throw new Error('artifact digests must be an object');
  }
  const artifactSet = new Set(artifacts);
  const entries = Object.entries(artifactDigests);
  if (entries.length === 0) throw new Error('artifact digests cannot be empty');
  for (const [name, digest] of entries) {
    if (!artifactSet.has(name)) throw new Error(`artifact digest is not declared: ${name}`);
    if (!DIGEST.test(digest)) throw new Error(`invalid artifact digest: ${name}`);
  }
  return Object.fromEntries(entries.sort(([left], [right]) => left.localeCompare(right)));
}

function resolveArtifactPath(directory, name) {
  if (typeof name !== 'string' || name.length === 0 || name.length > 240 || path.isAbsolute(name)) {
    throw new Error(`invalid artifact path: ${name}`);
  }
  const root = path.resolve(directory);
  const candidate = path.resolve(root, name);
  if (candidate !== root && !candidate.startsWith(`${root}${path.sep}`)) {
    throw new Error(`artifact path escapes directory: ${name}`);
  }
  return candidate;
}

export function buildArtifactDigests({ directory, names }) {
  if (typeof directory !== 'string' || !Array.isArray(names) || names.length === 0) {
    throw new Error('artifact digest input is incomplete');
  }
  const uniqueNames = [...new Set(names)];
  if (uniqueNames.length !== names.length) throw new Error('artifact digest names must be unique');
  const digests = {};
  for (const name of uniqueNames) {
    const file = resolveArtifactPath(directory, name);
    if (!statSync(file).isFile()) throw new Error(`artifact is not a file: ${name}`);
    digests[name] = createHash('sha256').update(readFileSync(file)).digest('hex');
  }
  return Object.fromEntries(Object.entries(digests).sort(([left], [right]) => left.localeCompare(right)));
}

export function verifyArtifactDigests({ manifest, directory }) {
  if (!manifest || !Array.isArray(manifest.artifacts)) throw new Error('manifest artifacts are missing');
  const declared = manifest.artifactDigests;
  if (!declared || typeof declared !== 'object' || Array.isArray(declared)) {
    throw new Error('manifest artifact digests are missing');
  }
  const names = Object.keys(declared);
  const actual = buildArtifactDigests({ directory, names });
  for (const name of names) {
    if (actual[name] !== declared[name]) throw new Error(`artifact digest mismatch: ${name}`);
  }
  return { verified: names.length, artifacts: names };
}

export function buildManifest({ tag, version, sha, tree, card, classification, relatedPullRequests, artifacts, artifactDigests, changelog, testInstructions }) {
  const normalized = normalizeVersion(version);
  if (!HEX_SHA.test(sha ?? '') || !HEX_SHA.test(tree ?? '')) throw new Error('release identity requires full commit and tree SHA');
  if (!TAG.test(tag) || tag !== `v${normalized}`) throw new Error('manifest tag/version mismatch');
  if (!/^PR-\d+$/.test(card)) throw new Error('manifest requires logical PR card');
  if (!Array.isArray(artifacts) || artifacts.length === 0) throw new Error('manifest requires downloadable artifacts');
  const normalizedArtifactDigests = validateArtifactDigests(artifacts, artifactDigests);
  return {
    schemaVersion: 1, tag, version: normalized, prerelease: true, stable: false, sha, tree,
    card, classification, relatedPullRequests, artifacts,
    ...(normalizedArtifactDigests ? { artifactDigests: normalizedArtifactDigests } : {}),
    changelog, testInstructions,
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
  const promotedArtifacts = manifest.artifacts.map((artifact) => artifact.replaceAll(prereleaseTag, stableTag));
  const promotedDigests = manifest.artifactDigests
    ? Object.fromEntries(Object.entries(manifest.artifactDigests).map(([artifact, digest]) => [artifact.replaceAll(prereleaseTag, stableTag), digest]))
    : undefined;
  return {
    ...manifest,
    tag: stableTag,
    version,
    prerelease: false,
    stable: true,
    milestone,
    artifacts: promotedArtifacts,
    ...(promotedDigests ? { artifactDigests: promotedDigests } : {}),
    provenance: {
      ...manifest.provenance,
      source: 'main',
      exactCommit: manifest.sha,
      tagImmutable: true,
      promotedFromTag: prereleaseTag,
    },
  };
}

export function assertChangelogIdentity({ range, sha, tree, headSha, headTree }) {
  if (!range || typeof range !== 'string') throw new Error('changelog requires an explicit commit range');
  const match = range.match(/^(.+)\.\.([0-9a-f]{40})$/);
  if (!match || !HEX_SHA.test(sha ?? '') || !HEX_SHA.test(tree ?? '')) {
    throw new Error('changelog range must end with a full commit SHA and include release identity');
  }
  if (match[2] !== sha || sha !== headSha || tree !== headTree) {
    throw new Error('changelog range and release identity must match HEAD');
  }
  return true;
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
  if (command === 'verify-artifacts') {
    const manifest = JSON.parse(readFileSync(arg('--manifest'), 'utf8'));
    const result = verifyArtifactDigests({ manifest, directory: arg('--directory') });
    process.stdout.write(`verified artifact digests: ${result.verified}\n`);
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
    const headSha = git(['rev-parse', 'HEAD']);
    const headTree = git(['rev-parse', 'HEAD^{tree}']);
    const sha = arg('--sha');
    const tree = arg('--tree');
    const range = arg('--range');
    assertChangelogIdentity({ range, sha, tree, headSha, headTree });
    const subjects = git(['log', range, '--format=%s']).split('\n').filter(Boolean);
    process.stdout.write(renderReleaseNotes({ tag: arg('--tag'), sha, card: arg('--card'), classification: classifyCommits(subjects), changelog: subjects.map((s) => `- ${s}`).join('\n'), testInstructions: 'Download the release artifact, verify the manifest SHA, and run the documented checks.', relatedPullRequests: (arg('--prs') ?? '').split(',').filter(Boolean) }));
    return;
  }
  throw new Error('usage: tag|verify-version|checks|classify|manifest|verify-artifacts|promote-manifest|milestone-version|changelog');
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) main();
