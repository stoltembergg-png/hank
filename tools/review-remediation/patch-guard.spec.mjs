import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, writeFileSync, mkdirSync, rmSync } from 'node:fs';
import { join, relative } from 'node:path';
import test from 'node:test';

import {
  MAX_PATCH_BYTES,
  MAX_PATCH_FILES,
  MAX_PATCH_LINES,
  assertAllowedPatchPaths,
  applyAndValidatePatch,
  validatePatchText,
  validateResultTree,
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

test('preserves a real top-level directory named b in unified diff paths', () => {
  const patch = validPatch.replaceAll('src/value.txt', 'b/src/value.txt');
  const result = validatePatchText(patch);

  assert.deepEqual(result.files, ['b/src/value.txt']);
  assert.throws(
    () => validatePatchText(patch.replace('+++ b/b/src/value.txt', '+++ a/b/src/value.txt')),
    (error) => error.code === 'PATCH_INVALID_PATH',
  );
});

test('allows double dots inside a filename while rejecting parent segments', () => {
  const dottedPatch = validPatch.replaceAll('src/value.txt', 'src/foo..txt');
  assert.deepEqual(validatePatchText(dottedPatch).files, ['src/foo..txt']);
  const traversalPatch = validPatch.replaceAll('src/value.txt', 'src/../value.txt');
  assert.throws(() => validatePatchText(traversalPatch), (error) => error.code === 'PATCH_INVALID_PATH');
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
  for (const path of ['.gitmodules', 'Cargo.toml', 'Cargo.lock', 'package.json', 'package-lock.json', 'pnpm-lock.yaml']) {
    assert.throws(() => assertAllowedPatchPaths([path]), (error) => error.code === 'PATCH_FORBIDDEN_PATH');
  }
});

test('rejects binary, symlink, submodule, rename, and mode metadata', () => {
  const forbiddenPatches = [
    `${validPatch}Binary files a/src/value.txt and b/src/value.txt differ\n`,
    validPatch.replace('--- a/src/value.txt', 'new file mode 120000\n--- a/src/value.txt'),
    validPatch.replace('--- a/src/value.txt', 'new file mode 160000\n--- a/src/value.txt'),
    validPatch.replace('--- a/src/value.txt', 'deleted file mode 120000\n--- a/src/value.txt'),
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

function workspaceInput(workspace) {
  return relative(process.cwd(), workspace);
}

function patchInput(patchFile) {
  return relative(process.cwd(), patchFile);
}

test('applies a valid patch only after git applicability and whitespace checks', async () => {
  const workspace = mkdtempSync(join(process.cwd(), '.hank-review-guard-'));
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
    writeFileSync(patchFile, validPatch.replace('+after', '+after\n'));

    await assert.rejects(
      applyAndValidatePatch({ workspace: workspaceInput(workspace), patchFile: patchInput(patchFile), expectedHeadSha: 'b'.repeat(40) }),
      (error) => error.code === 'PATCH_WORKSPACE_HEAD_MISMATCH',
    );
    assert.equal(readFileSync(join(workspace, 'src', 'value.txt'), 'utf8'), 'before\n');

    const result = await applyAndValidatePatch({ workspace: workspaceInput(workspace), patchFile: patchInput(patchFile) });
    assert.deepEqual(result.files, ['src/value.txt']);
    assert.equal(readFileSync(join(workspace, 'src', 'value.txt'), 'utf8').replaceAll('\r\n', '\n'), 'after\n');
    assert.match(result.treeDigest, /^[0-9a-f]{64}$/);
    assert.deepEqual(result.gates.map((gate) => gate.name), [
      'patch-applicability',
      'patch-boundaries',
      'whitespace',
      'semantic-syntax',
    ]);
  } finally {
    rmSync(workspace, { recursive: true, force: true });
  }
});

test('rejects syntax-invalid JavaScript and rolls the workspace back', async () => {
  const workspace = mkdtempSync(join(process.cwd(), '.hank-review-guard-syntax-'));
  try {
    mkdirSync(join(workspace, 'src'));
    writeFileSync(join(workspace, 'src', 'value.mjs'), 'const before = true;\n');
    git(workspace, ['init', '-q']);
    git(workspace, ['config', 'core.autocrlf', 'false']);
    git(workspace, ['config', 'user.email', 'test@example.invalid']);
    git(workspace, ['config', 'user.name', 'Test Fixture']);
    git(workspace, ['add', 'src/value.mjs']);
    git(workspace, ['commit', '-qm', 'fixture']);
    const patchFile = join(workspace, 'remediation.patch');
    writeFileSync(patchFile, [
      'diff --git a/src/value.mjs b/src/value.mjs',
      '--- a/src/value.mjs',
      '+++ b/src/value.mjs',
      '@@ -1,1 +1,1 @@',
      '-const before = true;',
      '+const = invalid;',
      '',
    ].join('\n'));

    await assert.rejects(
      applyAndValidatePatch({ workspace: workspaceInput(workspace), patchFile: patchInput(patchFile) }),
      (error) => error.code === 'PATCH_SEMANTIC_CHECK_FAILED',
    );
    assert.equal(readFileSync(join(workspace, 'src', 'value.mjs'), 'utf8'), 'const before = true;\n');
    assert.equal(git(workspace, ['status', '--porcelain', '--untracked-files=no']), '');
  } finally {
    rmSync(workspace, { recursive: true, force: true });
  }
});

test('accepts JavaScript deletions and unformatted Rust through the parser-only gate', async () => {
  const deletionWorkspace = mkdtempSync(join(process.cwd(), '.hank-review-guard-deletion-'));
  try {
    mkdirSync(join(deletionWorkspace, 'src'));
    writeFileSync(join(deletionWorkspace, 'src', 'value.mjs'), 'const before = true;\n');
    git(deletionWorkspace, ['init', '-q']);
    git(deletionWorkspace, ['config', 'core.autocrlf', 'false']);
    git(deletionWorkspace, ['config', 'user.email', 'test@example.invalid']);
    git(deletionWorkspace, ['config', 'user.name', 'Test Fixture']);
    git(deletionWorkspace, ['add', 'src/value.mjs']);
    git(deletionWorkspace, ['commit', '-qm', 'fixture']);
    const deletionPatchFile = join(deletionWorkspace, 'remediation.patch');
    writeFileSync(deletionPatchFile, [
      'diff --git a/src/value.mjs b/src/value.mjs',
      'deleted file mode 100644',
      '--- a/src/value.mjs',
      '+++ /dev/null',
      '@@ -1,1 +0,0 @@',
      '-const before = true;',
      '',
    ].join('\n'));

    const deletion = await applyAndValidatePatch({
      workspace: workspaceInput(deletionWorkspace),
      patchFile: patchInput(deletionPatchFile),
    });
    assert.deepEqual(deletion.files, ['src/value.mjs']);
    assert.equal(existsSync(join(deletionWorkspace, 'src', 'value.mjs')), false);
    assert.deepEqual(deletion.gates.map((gate) => gate.name), [
      'patch-applicability',
      'patch-boundaries',
      'whitespace',
      'semantic-syntax',
    ]);
  } finally {
    rmSync(deletionWorkspace, { recursive: true, force: true });
  }

  const rustWorkspace = mkdtempSync(join(process.cwd(), '.hank-review-guard-rust-'));
  try {
    mkdirSync(join(rustWorkspace, 'src'));
    writeFileSync(join(rustWorkspace, 'src', 'value.rs'), 'fn main() { println!("before"); }\n');
    git(rustWorkspace, ['init', '-q']);
    git(rustWorkspace, ['config', 'core.autocrlf', 'false']);
    git(rustWorkspace, ['config', 'user.email', 'test@example.invalid']);
    git(rustWorkspace, ['config', 'user.name', 'Test Fixture']);
    git(rustWorkspace, ['add', 'src/value.rs']);
    git(rustWorkspace, ['commit', '-qm', 'fixture']);
    const rustPatchFile = join(rustWorkspace, 'remediation.patch');
    writeFileSync(rustPatchFile, [
      'diff --git a/src/value.rs b/src/value.rs',
      '--- a/src/value.rs',
      '+++ b/src/value.rs',
      '@@ -1,1 +1,1 @@',
      '-fn main() { println!("before"); }',
      '+fn main(){println!("after");}',
      '',
    ].join('\n'));

    const rust = await applyAndValidatePatch({
      workspace: workspaceInput(rustWorkspace),
      patchFile: patchInput(rustPatchFile),
    });
    assert.deepEqual(rust.files, ['src/value.rs']);
    assert.match(readFileSync(join(rustWorkspace, 'src', 'value.rs'), 'utf8'), /after/);
    assert.equal(rust.gates.at(-1).name, 'semantic-syntax');
  } finally {
    rmSync(rustWorkspace, { recursive: true, force: true });
  }
});

test('rejects whitespace errors during application', async () => {
  const workspace = mkdtempSync(join(process.cwd(), '.hank-review-guard-whitespace-'));
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

    await assert.rejects(applyAndValidatePatch({ workspace: workspaceInput(workspace), patchFile: patchInput(patchFile) }), (error) => error.code === 'PATCH_APPLY_FAILED');
    assert.equal(readFileSync(join(workspace, 'src', 'value.txt'), 'utf8'), 'before\n');
    assert.equal(git(workspace, ['status', '--porcelain', '--untracked-files=no']), '');
  } finally {
    rmSync(workspace, { recursive: true, force: true });
  }
});

test('rejects modified ignored files instead of validating an unpublishable result', async () => {
  const workspace = mkdtempSync(join(process.cwd(), '.hank-review-guard-ignored-'));
  try {
    mkdirSync(join(workspace, 'ignored'));
    writeFileSync(join(workspace, '.gitignore'), 'ignored/\n');
    writeFileSync(join(workspace, 'ignored', 'value.txt'), 'before\n');
    git(workspace, ['init', '-q']);
    git(workspace, ['config', 'core.autocrlf', 'false']);
    git(workspace, ['config', 'user.email', 'test@example.invalid']);
    git(workspace, ['config', 'user.name', 'Test Fixture']);
    git(workspace, ['add', '.gitignore']);
    git(workspace, ['commit', '-qm', 'fixture']);
    const patchFile = join(workspace, 'remediation.patch');
    writeFileSync(patchFile, validPatch.replaceAll('src/value.txt', 'ignored/value.txt'));

    await assert.rejects(
      applyAndValidatePatch({ workspace: workspaceInput(workspace), patchFile: patchInput(patchFile) }),
      (error) => error.code === 'PATCH_RESULT_IGNORED',
    );
    assert.equal(readFileSync(join(workspace, 'ignored', 'value.txt'), 'utf8'), 'before\n');
    assert.equal(git(workspace, ['status', '--porcelain', '--untracked-files=no']), '');
  } finally {
    rmSync(workspace, { recursive: true, force: true });
  }
});

test('rejects ignored deletions before skipping an absent result path', () => {
  const workspace = mkdtempSync(join(process.cwd(), '.hank-review-guard-ignored-deletion-'));
  try {
    mkdirSync(join(workspace, 'ignored'));
    writeFileSync(join(workspace, '.gitignore'), 'ignored/\n');
    git(workspace, ['init', '-q']);
    git(workspace, ['config', 'core.autocrlf', 'false']);
    git(workspace, ['config', 'user.email', 'test@example.invalid']);
    git(workspace, ['config', 'user.name', 'Test Fixture']);
    git(workspace, ['add', '.gitignore']);
    git(workspace, ['commit', '-qm', 'fixture']);

    assert.throws(
      () => validateResultTree({ workspace: workspaceInput(workspace), beforeFiles: ['ignored/value.txt'], afterFiles: ['ignored/value.txt'] }),
      (error) => error.code === 'PATCH_RESULT_IGNORED',
    );
  } finally {
    rmSync(workspace, { recursive: true, force: true });
  }
});

test('rejects an allowlisted result file above the output size limit', () => {
  const workspace = mkdtempSync(join(process.cwd(), '.hank-review-guard-large-'));
  try {
    mkdirSync(join(workspace, 'src'));
    writeFileSync(join(workspace, 'src', 'value.txt'), 'x'.repeat(256 * 1024 + 1));
    assert.throws(
      () => validateResultTree({ workspace: workspaceInput(workspace), beforeFiles: ['src/value.txt'], afterFiles: ['src/value.txt'] }),
      (error) => error.code === 'PATCH_RESULT_TOO_LARGE',
    );
  } finally {
    rmSync(workspace, { recursive: true, force: true });
  }
});
