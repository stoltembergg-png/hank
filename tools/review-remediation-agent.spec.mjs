import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';
import { join, resolve } from 'node:path';
import test from 'node:test';

const cliPath = fileURLToPath(new URL('./review-remediation-agent.mjs', import.meta.url));

test('rejects JSON input outside the configured workspace before reading it', () => {
  const root = mkdtempSync(join(resolve(tmpdir()), 'hank-review-cli-'));
  const workspace = join(root, 'workspace');
  const outside = join(root, 'outside.json');
  mkdirSync(workspace);
  writeFileSync(outside, '{}');
  const environment = { ...process.env, GITHUB_WORKSPACE: workspace };
  delete environment.GITHUB_EVENT_PATH;

  try {
    assert.throws(
      () => execFileSync(process.execPath, [
        cliPath,
        'validate',
        '--input', outside,
        '--patch', join(workspace, 'proposal.patch'),
        '--workspace', workspace,
        '--output', join(workspace, 'validation.json'),
      ], { cwd: workspace, env: environment, encoding: 'utf8', windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] }),
      (error) => error.status === 1 && error.stderr.includes('CLI_INPUT_INVALID'),
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('reads JSON input whose filename contains double dots', () => {
  const root = mkdtempSync(join(resolve(tmpdir()), 'hank-review-cli-dotted-'));
  const workspace = join(root, 'workspace');
  mkdirSync(join(workspace, 'src'), { recursive: true });
  writeFileSync(join(workspace, 'src', 'value.txt'), 'before\n');
  const patch = [
    'diff --git a/src/value.txt b/src/value.txt',
    '--- a/src/value.txt',
    '+++ b/src/value.txt',
    '@@ -1,1 +1,1 @@',
    '-before',
    '+after',
    '',
  ].join('\n');
  writeFileSync(join(workspace, 'remediation.patch'), patch);
  const gitOptions = { cwd: workspace, encoding: 'utf8', windowsHide: true };
  execFileSync('git', ['init', '-q'], gitOptions);
  execFileSync('git', ['config', 'core.autocrlf', 'false'], gitOptions);
  execFileSync('git', ['config', 'user.email', 'test@example.invalid'], gitOptions);
  execFileSync('git', ['config', 'user.name', 'Test Fixture'], gitOptions);
  execFileSync('git', ['add', 'src/value.txt'], gitOptions);
  execFileSync('git', ['commit', '-qm', 'fixture'], gitOptions);
  const sourceSha = execFileSync('git', ['rev-parse', 'HEAD'], gitOptions).trim();
  writeFileSync(join(workspace, 'proposal..json'), JSON.stringify({
    status: 'PROPOSED',
    finding: {
      repository: 'local/test',
      pullRequest: 1,
      sourceBranch: 'feature',
      baseBranch: 'main',
      headSha: sourceSha,
      fingerprint: 'a'.repeat(64),
      path: 'src/value.txt',
    },
    cycle: 1,
    viability: 'VIABLE_PATCH',
    patchDigest: createHash('sha256').update(patch).digest('hex'),
  }));
  const environment = { ...process.env, GITHUB_WORKSPACE: workspace };
  try {
    execFileSync(process.execPath, [
      cliPath,
      'validate',
      '--input', 'proposal..json',
      '--patch', 'remediation.patch',
      '--workspace', '.',
      '--output', 'validation.json',
    ], { cwd: workspace, env: environment, encoding: 'utf8', windowsHide: true });
    const result = JSON.parse(readFileSync(join(workspace, 'validation.json'), 'utf8'));
    assert.equal(result.status, 'VALIDATED');
    assert.deepEqual(result.files, ['src/value.txt']);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
