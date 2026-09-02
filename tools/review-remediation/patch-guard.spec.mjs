import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync, mkdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

import {
  MAX_PATCH_BYTES,
  MAX_PATCH_FILES,
  MAX_PATCH_LINES,
  assertAllowedPatchPaths,
  applyAndValidatePatch,
  validatePatchText,
} from './patch-guard.mjs';

const validPatch = [
  'diff --git a/src/value.txt b/src/value.txt',
  '--- a/src/value.txt',
  '+++ b/src/value.txt',
  '@@ -1,1 +1,1 @@',
  '-before',
  '+after',
  '',
].join('\n');

test('accepts a bounded source patch and reports changed files and line counts', () => {
  const result = validatePatchText(validPatch);

  assert.deepEqual(result.files, ['src/value.txt']);
  assert.equal(result.addedLines, 1);
  assert.equal(result.deletedLines, 1);
  assert.equal(result.totalChangedLines, 2);
  assert.match(result.digest, /^[0-9a-f]{64}$/);
});

test('rejects traversal, absolute, Windows, forbidden, and trusted-helper paths', () => {
  const paths = [
    '../secret.txt',
    '/absolute.txt',
    'src\\value.txt',
    '.github/workflows/build.yml',
    '.github/actions/check/action.yml',
    '.env',
    'config/credentials.json',
    '.github/CODEOWNERS',
    'tools/review-remediation/contracts.mjs',
  ];

  for (const path of paths) {
    const patch = validPatch.replaceAll('src/value.txt', path);
    assert.throws(() => validatePatchText(patch), (error) => error.code.startsWith('PATCH_'), path);
  }
  assert.throws(() => assertAllowedPatchPaths(['src/value.txt', 'tools/review-remediation-agent.mjs']), (error) => error.code === 'PATCH_FORBIDDEN_PATH');
});

test('rejects binary, symlink, submodule, rename, and mode metadata', () => {
  const forbiddenPatches = [
    `${validPatch}Binary files a/src/value.txt and b/src/value.txt differ\n`,
    validPatch.replace('--- a/src/value.txt', 'new file mode 120000\n--- a/src/value.txt'),
    validPatch.replace('--- a/src/value.txt', 'new file mode 160000\n--- a/src/value.txt'),
    validPatch.replace('diff --git a/src/value.txt b/src/value.txt', 'diff --git a/src/value.txt b/src/renamed.txt\nrename from src/value.txt\nrename to src/renamed.txt'),
  ];

  for (const patch of forbiddenPatches) {
    assert.throws(() => validatePatchText(patch), (error) => error.code.startsWith('PATCH_'));
  }
});

test('rejects malformed patches and configured size limits', () => {
  assert.throws(() => validatePatchText('not a patch'), (error) => error.code === 'PATCH_MALFORMED');
  assert.throws(() => validatePatchText(validPatch.replace('@@ -1,1 +1,1 @@', '@@ -1,1 +1,1 @@\n+bad\u0000')), (error) => error.code === 'PATCH_FORBIDDEN_CONTENT');

  const manyFiles = Array.from({ length: MAX_PATCH_FILES + 1 }, (_, index) => [
    `diff --git a/src/${index}.txt b/src/${index}.txt`,
    `--- a/src/${index}.txt`,
    `+++ b/src/${index}.txt`,
    '@@ -0,0 +1,1 @@',
    '+new',
  ].join('\n')).join('\n');
  assert.throws(() => validatePatchText(manyFiles), (error) => error.code === 'PATCH_TOO_MANY_FILES');

  const manyLines = [
    'diff --git a/src/value.txt b/src/value.txt',
    '--- a/src/value.txt',
    '+++ b/src/value.txt',
    `@@ -1,1 +1,${MAX_PATCH_LINES + 1} @@`,
    ...Array.from({ length: MAX_PATCH_LINES + 1 }, (_, index) => `+line-${index}`),
  ].join('\n');
  assert.throws(() => validatePatchText(manyLines), (error) => error.code === 'PATCH_TOO_MANY_LINES');

  assert.throws(() => validatePatchText(`diff --git a/src/value.txt b/src/value.txt\n${'x'.repeat(MAX_PATCH_BYTES)}`), (error) => error.code === 'PATCH_TOO_LARGE');
});

function git(workspace, args) {
  return execFileSync('git', args, { cwd: workspace, encoding: 'utf8', windowsHide: true });
}

test('applies a valid patch only after git applicability and whitespace checks', async () => {
  const workspace = mkdtempSync(join(tmpdir(), 'hank-review-guard-'));
  try {
    mkdirSync(join(workspace, 'src'));
    writeFileSync(join(workspace, 'src', 'value.txt'), 'before\n');
    git(workspace, ['init', '-q']);
    git(workspace, ['config', 'core.autocrlf', 'false']);
    git(workspace, ['config', 'user.email', 'test@example.invalid']);
    git(workspace, ['config', 'user.name', 'Test Fixture']);
    git(workspace, ['add', 'src/value.txt']);
    git(workspace, ['commit', '-qm', 'fixture']);
    const patchFile = join(workspace, 'remediation.patch');
    writeFileSync(patchFile, validPatch.replace('-before', '-before').replace('+after', '+after\n'));

    const result = await applyAndValidatePatch({ workspace, patchFile });
    assert.deepEqual(result.files, ['src/value.txt']);
    assert.equal(readFileSync(join(workspace, 'src', 'value.txt'), 'utf8').replaceAll('\r\n', '\n'), 'after\n');
    assert.match(result.treeDigest, /^[0-9a-f]{64}$/);
  } finally {
    rmSync(workspace, { recursive: true, force: true });
  }
});

test('rejects whitespace errors during application', async () => {
  const workspace = mkdtempSync(join(tmpdir(), 'hank-review-guard-whitespace-'));
  try {
    mkdirSync(join(workspace, 'src'));
    writeFileSync(join(workspace, 'src', 'value.txt'), 'before\n');
    git(workspace, ['init', '-q']);
    git(workspace, ['config', 'core.autocrlf', 'false']);
    git(workspace, ['config', 'user.email', 'test@example.invalid']);
    git(workspace, ['config', 'user.name', 'Test Fixture']);
    git(workspace, ['add', 'src/value.txt']);
    git(workspace, ['commit', '-qm', 'fixture']);
    const patchFile = join(workspace, 'remediation.patch');
    writeFileSync(patchFile, validPatch.replace('+after', '+after '));

    await assert.rejects(applyAndValidatePatch({ workspace, patchFile }), (error) => error.code === 'PATCH_APPLY_FAILED');
  } finally {
    rmSync(workspace, { recursive: true, force: true });
  }
});
