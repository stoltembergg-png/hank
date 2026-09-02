import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
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
