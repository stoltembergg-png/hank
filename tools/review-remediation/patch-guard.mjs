import { promisify } from 'node:util';
import { createHash } from 'node:crypto';
import { execFile, execFileSync } from 'node:child_process';
import {
  lstatSync,
  readFileSync,
  readlinkSync,
  readdirSync,
  realpathSync,
} from 'node:fs';
import { isAbsolute, relative, resolve } from 'node:path';

import {
  MAX_PATCH_BYTES,
  MAX_PATCH_FILES,
  MAX_PATCH_LINES,
  MAX_RESULT_FILE_BYTES,
} from './contracts.mjs';

export { MAX_PATCH_BYTES, MAX_PATCH_FILES, MAX_PATCH_LINES, MAX_RESULT_FILE_BYTES } from './contracts.mjs';

const execFileAsync = promisify(execFile);
const CONTROL_CHARACTERS = /[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/;
const FORBIDDEN_METADATA = /^(?:old mode|new mode|new file mode|deleted file mode|similarity index|rename from|rename to|copy from|copy to)\s/m;
const FORBIDDEN_PATH = /^(?:\.github\/(?:workflows|actions)(?:\/|$)|\.git(?:\/|$)|\.env(?:\.|$)|tools\/review-remediation(?:\/|$))/i;

export class PatchGuardError extends Error {
  constructor(code) {
    super(code);
    this.name = 'PatchGuardError';
    this.code = code;
  }
}

function error(code) {
  return new PatchGuardError(code);
}

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function normalizePatchPath(value) {
  if (typeof value !== 'string' || value.length === 0 || value === '/dev/null') return null;
  if (value.startsWith('a/')) value = value.slice(2);
  if (value.startsWith('b/')) value = value.slice(2);
  if (value.includes('\\') || value.startsWith('/') || value.includes('\u0000') || CONTROL_CHARACTERS.test(value)) return null;
  const segments = value.split('/');
  if (segments.some((segment) => segment.length === 0 || segment === '.' || segment === '..')) return null;
  return value;
}

function isForbiddenPath(path) {
  const basename = path.split('/').at(-1) ?? '';
  return FORBIDDEN_PATH.test(path)
    || path === 'tools/review-remediation-agent.mjs'
    || /(?:^|\/)(?:CODEOWNERS|branch-protection|rulesets?)(?:\.|\/|$)/i.test(path)
    || /^(?:credentials?|secrets?|tokens?|auth(?:entication)?|id_rsa|id_ed25519|\.npmrc|\.pypirc)$/i.test(basename)
    || /\.(?:pem|key|p12|pfx)$/i.test(basename)
    || /(?:secret|credential|branch-protection|ruleset)/i.test(path);
}

export function assertAllowedPatchPaths(files) {
  if (!Array.isArray(files) || files.length === 0 || files.length > MAX_PATCH_FILES) throw error('PATCH_TOO_MANY_FILES');
  const seen = new Set();
  for (const value of files) {
    const path = normalizePatchPath(value);
    if (!path) throw error('PATCH_INVALID_PATH');
    if (isForbiddenPath(path)) throw error('PATCH_FORBIDDEN_PATH');
    if (seen.has(path)) throw error('PATCH_DUPLICATE_PATH');
    seen.add(path);
  }
  return [...seen];
}

function sectionData(patch) {
  const lines = patch.split(/\r?\n/);
  const starts = lines
    .map((line, index) => line.startsWith('diff --git ') ? index : -1)
    .filter((index) => index >= 0);
  if (starts.length === 0) throw error('PATCH_MALFORMED');
  return starts.map((start, index) => lines.slice(start, starts[index + 1] ?? lines.length));
}

function parseSection(lines) {
  const header = /^diff --git a\/(.+) b\/(.+)$/.exec(lines[0]);
  if (!header) throw error('PATCH_MALFORMED');
  const oldHeader = lines.find((line) => line.startsWith('--- '));
  const newHeader = lines.find((line) => line.startsWith('+++ '));
  if (!oldHeader || !newHeader || !lines.some((line) => line.startsWith('@@ '))) throw error('PATCH_MALFORMED');

  const headerOldPath = normalizePatchPath(header[1]);
  const headerNewPath = normalizePatchPath(header[2]);
  const oldPath = oldHeader.slice(4).trim() === '/dev/null' ? null : normalizePatchPath(oldHeader.slice(4).trim());
  const newPath = newHeader.slice(4).trim() === '/dev/null' ? null : normalizePatchPath(newHeader.slice(4).trim());
  if (!headerOldPath || !headerNewPath || (!oldPath && !newPath) || (oldPath && oldPath !== headerOldPath) || (newPath && newPath !== headerNewPath)) {
    throw error('PATCH_INVALID_PATH');
  }
  if (oldPath && newPath && oldPath !== newPath) throw error('PATCH_RENAME_FORBIDDEN');
  return { path: newPath ?? oldPath, lines };
}

export function validatePatchText(patch) {
  if (typeof patch !== 'string') throw error('PATCH_MALFORMED');
  if (Buffer.byteLength(patch, 'utf8') > MAX_PATCH_BYTES) throw error('PATCH_TOO_LARGE');
  if (CONTROL_CHARACTERS.test(patch)) throw error('PATCH_FORBIDDEN_CONTENT');
  if (/^(?:Binary files|GIT binary patch)/m.test(patch)) throw error('PATCH_BINARY_FORBIDDEN');
  if (FORBIDDEN_METADATA.test(patch)) throw error('PATCH_METADATA_FORBIDDEN');

  const sections = sectionData(patch);
  if (sections.length > MAX_PATCH_FILES) throw error('PATCH_TOO_MANY_FILES');
  const parsed = sections.map(parseSection);
  const files = assertAllowedPatchPaths(parsed.map(({ path }) => path));
  const addedLines = patch.split(/\r?\n/).filter((line) => line.startsWith('+') && !line.startsWith('+++')).length;
  const deletedLines = patch.split(/\r?\n/).filter((line) => line.startsWith('-') && !line.startsWith('---')).length;
  const totalChangedLines = addedLines + deletedLines;
  if (totalChangedLines > MAX_PATCH_LINES) throw error('PATCH_TOO_MANY_LINES');
  return {
    digest: sha256(patch),
    files,
    addedLines,
    deletedLines,
    totalChangedLines,
  };
}

function toPosixPath(value) {
  return value.replaceAll('\\', '/');
}

function pathWithin(root, candidate) {
  const relativePath = relative(root, candidate);
  return relativePath === '' || (!relativePath.startsWith('..') && !isAbsolute(relativePath));
}

function resolveWorkspaceRoot(workspace) {
  if (typeof workspace !== 'string' || workspace.length === 0 || workspace.includes('\u0000')) throw error('PATCH_INPUT_INVALID');
  const root = resolve(workspace);
  let stat;
  let canonical;
  try {
    stat = lstatSync(root);
    canonical = realpathSync(root);
  } catch {
    throw error('PATCH_INPUT_INVALID');
  }
  if (!stat.isDirectory() || stat.isSymbolicLink() || !pathWithin(root, canonical)) throw error('PATCH_INPUT_INVALID');
  return canonical;
}

function resolvePatchPath(workspace, patchFile) {
  if (typeof patchFile !== 'string' || patchFile.length === 0 || patchFile.includes('\u0000')) throw error('PATCH_INPUT_INVALID');
  const segments = patchFile.replaceAll('\\', '/').split('/');
  if (segments.includes('..')) throw error('PATCH_INPUT_INVALID');
  const patchPath = resolve(patchFile);
  const currentDirectory = resolve(process.cwd());
  if (!pathWithin(workspace, patchPath) && !pathWithin(currentDirectory, patchPath)) throw error('PATCH_INPUT_INVALID');
  return patchPath;
}

export function readPatchFile({ workspace, patchFile }) {
  const root = resolveWorkspaceRoot(workspace);
  const patchPath = resolvePatchPath(root, patchFile);
  try {
    const stat = lstatSync(patchPath);
    if (!stat.isFile() || stat.isSymbolicLink()) throw error('PATCH_INPUT_INVALID');
    if (stat.size > MAX_PATCH_BYTES) throw error('PATCH_TOO_LARGE');
    return readFileSync(patchPath, 'utf8');
  } catch (caught) {
    if (caught instanceof PatchGuardError) throw caught;
    throw error('PATCH_INPUT_INVALID');
  }
}

function snapshotWorkspace(workspace, allowlistedPaths = []) {
  const root = resolveWorkspaceRoot(workspace);
  const allowlisted = new Set(allowlistedPaths);
  const snapshot = new Map();

  function visit(directory) {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const absolute = resolve(directory, entry.name);
      const relativeCheck = relative(root, absolute);
      if (relativeCheck.startsWith('..') || isAbsolute(relativeCheck)) throw error('PATCH_RESULT_PATH_INVALID');
      const relativePath = toPosixPath(relativeCheck);
      if (!relativePath || relativePath === '.git' || relativePath.startsWith('.git/')) continue;
      if (entry.isDirectory()) {
        visit(absolute);
        continue;
      }
      const stat = lstatSync(absolute);
      if (entry.isSymbolicLink()) {
        snapshot.set(relativePath, { kind: 'symlink', value: readlinkSync(absolute), bytes: 0 });
        continue;
      }
      if (!entry.isFile()) {
        snapshot.set(relativePath, { kind: 'special', value: '', bytes: stat.size });
        continue;
      }
      const bytes = stat.size;
      if (allowlisted.has(relativePath) && bytes > MAX_RESULT_FILE_BYTES) throw error('PATCH_RESULT_TOO_LARGE');
      const digest = bytes <= MAX_RESULT_FILE_BYTES ? sha256(readFileSync(absolute)) : `size:${bytes}`;
      snapshot.set(relativePath, { kind: 'file', value: digest, bytes });
    }
  }

  visit(root);
  return snapshot;
}

function changedPaths(before, after) {
  const paths = new Set([...before.keys(), ...after.keys()]);
  return [...paths].filter((path) => JSON.stringify(before.get(path)) !== JSON.stringify(after.get(path))).sort();
}

function treeDigest(snapshot, files) {
  const rows = files
    .sort()
    .map((path) => `${path}\0${JSON.stringify(snapshot.get(path) ?? null)}`)
    .join('\n');
  return sha256(rows);
}

function isIgnoredResultPath(workspace, path) {
  try {
    execFileSync('git', ['check-ignore', '--quiet', '--', path], {
      cwd: workspace,
      windowsHide: true,
      shell: false,
      stdio: 'ignore',
    });
    return true;
  } catch (caught) {
    if (caught?.status === 1) return false;
    throw error('PATCH_IGNORE_CHECK_FAILED');
  }
}

export function validateResultTree({ workspace, beforeFiles, afterFiles }) {
  const allowed = assertAllowedPatchPaths(beforeFiles);
  const actual = Array.isArray(afterFiles) ? [...new Set(afterFiles)].sort() : Object.keys(afterFiles ?? {}).sort();
  if (actual.some((path) => !allowed.includes(path))) throw error('PATCH_RESULT_OUTSIDE_ALLOWLIST');
  const snapshot = snapshotWorkspace(workspace, allowed);
  for (const path of actual) {
    const entry = snapshot.get(path);
    if (!entry) continue;
    if (entry.kind !== 'file') throw error('PATCH_RESULT_SPECIAL_FILE');
    if (entry.bytes > MAX_RESULT_FILE_BYTES) throw error('PATCH_RESULT_TOO_LARGE');
    if (isIgnoredResultPath(resolveWorkspaceRoot(workspace), path)) throw error('PATCH_RESULT_IGNORED');
  }
  return { files: actual, treeDigest: treeDigest(snapshot, actual) };
}

async function runGit(workspace, args, code) {
  try {
    return await execFileAsync('git', args, {
      cwd: workspace,
      windowsHide: true,
      shell: false,
      maxBuffer: 64 * 1024,
    });
  } catch {
    throw error(code);
  }
}

export async function applyAndValidatePatch({ workspace, patchFile }) {
  if (typeof workspace !== 'string' || typeof patchFile !== 'string') throw error('PATCH_INPUT_INVALID');
  const root = resolveWorkspaceRoot(workspace);
  const patchPath = resolvePatchPath(root, patchFile);
  const patch = readPatchFile({ workspace: root, patchFile: patchPath });
  const metadata = validatePatchText(patch);
  const before = snapshotWorkspace(root, metadata.files);
  await runGit(root, ['apply', '--check', '--whitespace=error', '--', patchPath], 'PATCH_APPLY_FAILED');
  await runGit(root, ['apply', '--whitespace=error', '--', patchPath], 'PATCH_APPLY_FAILED');
  await runGit(root, ['diff', '--check'], 'PATCH_APPLY_FAILED');
  const after = snapshotWorkspace(root, metadata.files);
  const changed = changedPaths(before, after);
  const tree = validateResultTree({ workspace: root, beforeFiles: metadata.files, afterFiles: changed });
  return { ...metadata, ...tree };
}
