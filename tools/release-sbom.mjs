#!/usr/bin/env node
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { normalizeVersion } from './release-prerelease.mjs';

const GIT_ID = /^[0-9a-f]{40}$/;
const PACKAGE_VERSION = /^\S{1,160}$/;
const CREATED = '1970-01-01T00:00:00Z';
const TOOL_VERSION = 'hank-release-sbom/1';
const DEFAULT_NPM_LOCKFILES = ['frontend/package-lock.json', 'desktop-e2e/package-lock.json'];

function requiredString(value, field) {
  if (typeof value !== 'string' || value.length === 0 || value.length > 256) throw new Error(`${field} is missing`);
  return value;
}

function validateIdentity({ commit, tree, version }) {
  if (!GIT_ID.test(commit ?? '') || !GIT_ID.test(tree ?? '')) throw new Error('release identity requires full commit and tree SHA');
  return { commit, tree, version: normalizeVersion(version) };
}

function resolveInput(root, relative, kind) {
  if (typeof relative !== 'string' || relative.length === 0 || path.isAbsolute(relative)) throw new Error(`invalid ${kind} path: ${relative}`);
  const rootPath = path.resolve(root);
  const candidate = path.resolve(rootPath, relative);
  if (candidate !== rootPath && !candidate.startsWith(`${rootPath}${path.sep}`)) throw new Error(`${kind} path escapes root: ${relative}`);
  try {
    if (!statSync(candidate).isFile()) throw new Error(`${kind} is not a file: ${relative}`);
  } catch (error) {
    if (error?.message?.includes(`${kind} is not a file`)) throw error;
    throw new Error(`${kind} is missing: ${relative}`);
  }
  return candidate;
}

function packageId(source, location, name, version) {
  const digest = createHash('sha256').update(`${source}\0${location}\0${name}\0${version}`).digest('hex').slice(0, 24);
  return `SPDXRef-${source}-${digest}`;
}

function packageRecord({ source, location, name, version }) {
  requiredString(name, 'package name');
  if (!PACKAGE_VERSION.test(version ?? '')) throw new Error(`package version is missing: ${name}`);
  return {
    SPDXID: packageId(source, location, name, version),
    name,
    versionInfo: version,
    packageFileName: `${source}:${location}`,
    downloadLocation: 'NOASSERTION',
    filesAnalyzed: false,
    licenseConcluded: 'NOASSERTION',
    licenseDeclared: 'NOASSERTION',
  };
}

export function parseCargoLock(text) {
  if (typeof text !== 'string') throw new Error('Cargo.lock is not text');
  const records = [];
  let current = null;
  const flush = () => {
    if (!current) return;
    if (!current.name) throw new Error('Cargo.lock package name is missing');
    records.push(packageRecord({ source: 'cargo', location: 'Cargo.lock', name: current.name, version: current.version }));
    current = null;
  };
  for (const line of text.split(/\r?\n/)) {
    if (line.trim() === '[[package]]') {
      flush();
      current = {};
      continue;
    }
    if (!current) continue;
    const name = line.match(/^name\s*=\s*"([^"\\]*(?:\\.[^"\\]*)*)"\s*$/)?.[1];
    const version = line.match(/^version\s*=\s*"([^"\\]*(?:\\.[^"\\]*)*)"\s*$/)?.[1];
    if (name !== undefined) current.name = name;
    if (version !== undefined) current.version = version;
  }
  flush();
  if (records.length === 0) throw new Error('Cargo.lock contains no packages');
  return records;
}

function npmPackageName(location, entry) {
  if (typeof entry?.name === 'string' && entry.name.length > 0) return entry.name;
  const marker = 'node_modules/';
  const index = location.lastIndexOf(marker);
  if (index < 0) throw new Error(`npm package name is missing: ${location}`);
  const tail = location.slice(index + marker.length).split('/');
  if (tail[0]?.startsWith('@')) return tail.slice(0, 2).join('/');
  return tail[0];
}

export function parseNpmLock(text, source) {
  let lock;
  try { lock = JSON.parse(text); } catch { throw new Error(`${source} is not valid JSON`); }
  if (!lock || typeof lock !== 'object' || !lock.packages || typeof lock.packages !== 'object') throw new Error(`${source} packages are missing`);
  const records = [];
  for (const [location, entry] of Object.entries(lock.packages)) {
    if (location === '') continue;
    if (!entry || typeof entry !== 'object') throw new Error(`${source} package entry is invalid: ${location}`);
    const name = npmPackageName(location, entry);
    records.push(packageRecord({ source: 'npm', location: `${source}:${location}`, name, version: entry.version }));
  }
  return records;
}

function sortPackages(packages) {
  return [...packages].sort((left, right) =>
    left.name.localeCompare(right.name) || left.versionInfo.localeCompare(right.versionInfo) || left.packageFileName.localeCompare(right.packageFileName) || left.SPDXID.localeCompare(right.SPDXID));
}

export function buildSbom({ root = process.cwd(), commit, tree, version, npmLockfiles = DEFAULT_NPM_LOCKFILES } = {}) {
  const identity = validateIdentity({ commit, tree, version });
  const cargoPath = resolveInput(root, 'Cargo.lock', 'Cargo.lock');
  if (!Array.isArray(npmLockfiles) || npmLockfiles.length === 0) throw new Error('npm lockfiles are missing');
  const packages = parseCargoLock(readFileSync(cargoPath, 'utf8'));
  for (const relative of npmLockfiles) {
    const file = resolveInput(root, relative, 'npm lockfile');
    packages.push(...parseNpmLock(readFileSync(file, 'utf8'), relative));
  }
  const sorted = sortPackages(packages);
  const documentDescribes = sorted.map((pkg) => pkg.SPDXID);
  return {
    spdxVersion: 'SPDX-2.3',
    dataLicense: 'CC0-1.0',
    SPDXID: 'SPDXRef-DOCUMENT',
    name: 'Hank Release SBOM',
    documentNamespace: `https://hank.invalid/sbom/${identity.commit}/${identity.tree}`,
    creationInfo: {
      created: CREATED,
      creators: [`Tool: ${TOOL_VERSION}`],
      comment: `sourceCommit=${identity.commit}; sourceTree=${identity.tree}; version=${identity.version}`,
    },
    documentDescribes,
    packages: sorted,
    relationships: sorted.map((pkg) => ({
      spdxElementId: 'SPDXRef-DOCUMENT',
      relationshipType: 'DESCRIBES',
      relatedSpdxElement: pkg.SPDXID,
    })),
  };
}

function parseProvenanceComment(comment) {
  const match = typeof comment === 'string'
    ? comment.match(/^sourceCommit=([0-9a-f]{40}); sourceTree=([0-9a-f]{40}); version=(\S+)$/)
    : null;
  if (!match) throw new Error('SBOM provenance is missing');
  return { commit: match[1], tree: match[2], version: match[3] };
}

export function verifySbom({ sbom, expectedCommit, expectedTree, expectedVersion } = {}) {
  if (!sbom || typeof sbom !== 'object' || sbom.spdxVersion !== 'SPDX-2.3' || sbom.SPDXID !== 'SPDXRef-DOCUMENT') throw new Error('SBOM schema is invalid');
  const expected = validateIdentity({ commit: expectedCommit, tree: expectedTree, version: expectedVersion });
  const actual = parseProvenanceComment(sbom.creationInfo?.comment);
  if (actual.commit !== expected.commit || actual.tree !== expected.tree || actual.version !== expected.version) throw new Error('SBOM provenance identity mismatch');
  if (sbom.documentNamespace !== `https://hank.invalid/sbom/${expected.commit}/${expected.tree}`) throw new Error('SBOM provenance namespace mismatch');
  if (sbom.creationInfo.created !== CREATED || JSON.stringify(sbom.creationInfo.creators) !== JSON.stringify([`Tool: ${TOOL_VERSION}`])) throw new Error('SBOM provenance metadata mismatch');
  if (!Array.isArray(sbom.packages) || sbom.packages.length === 0) throw new Error('SBOM packages are missing');
  const ids = sbom.packages.map((pkg) => pkg?.SPDXID);
  if (new Set(ids).size !== ids.length) throw new Error('SBOM package IDs must be unique');
  for (const pkg of sbom.packages) {
    if (!pkg || typeof pkg.name !== 'string' || !PACKAGE_VERSION.test(pkg.versionInfo ?? '') || pkg.downloadLocation !== 'NOASSERTION' || pkg.licenseConcluded !== 'NOASSERTION' || pkg.licenseDeclared !== 'NOASSERTION') {
      throw new Error(`SBOM package is invalid: ${pkg?.name ?? 'unknown'}`);
    }
  }
  const sorted = sortPackages(sbom.packages);
  if (JSON.stringify(sorted) !== JSON.stringify(sbom.packages)) throw new Error('SBOM packages are not deterministically sorted');
  if (JSON.stringify(sbom.documentDescribes) !== JSON.stringify(ids)) throw new Error('SBOM document relationships are incomplete');
  if (!Array.isArray(sbom.relationships) || sbom.relationships.length !== ids.length || sbom.relationships.some((relationship, index) => relationship.spdxElementId !== 'SPDXRef-DOCUMENT' || relationship.relationshipType !== 'DESCRIBES' || relationship.relatedSpdxElement !== ids[index])) {
    throw new Error('SBOM relationships are incomplete');
  }
  return true;
}

export function writeSbom({ output, ...options }) {
  requiredString(output, 'SBOM output');
  const sbom = buildSbom(options);
  const target = path.resolve(output);
  mkdirSync(path.dirname(target), { recursive: true });
  writeFileSync(target, `${JSON.stringify(sbom, null, 2)}\n`, { flag: 'w' });
  return sbom;
}

function arg(name) {
  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] : null;
}

function main() {
  const command = process.argv[2];
  if (command === 'generate') {
    const npmLockfiles = arg('--npm-lockfiles')?.split(',').filter(Boolean) ?? DEFAULT_NPM_LOCKFILES;
    const sbom = writeSbom({ root: arg('--root') ?? process.cwd(), output: arg('--output'), commit: arg('--commit'), tree: arg('--tree'), version: arg('--version'), npmLockfiles });
    process.stdout.write(`${JSON.stringify({ generated: true, packages: sbom.packages.length, output: path.resolve(arg('--output')) })}\n`);
    return;
  }
  if (command === 'verify') {
    const file = requiredString(arg('--file'), 'SBOM file');
    const sbom = JSON.parse(readFileSync(file, 'utf8'));
    verifySbom({ sbom, expectedCommit: arg('--commit'), expectedTree: arg('--tree'), expectedVersion: arg('--version') });
    process.stdout.write(JSON.stringify({ verified: true, packages: sbom.packages.length }) + '\n');
    return;
  }
  throw new Error('command must be generate or verify');
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  try { main(); } catch (error) { process.stderr.write(`${error instanceof Error ? error.message : 'SBOM generation failed'}\n`); process.exitCode = 1; }
}
