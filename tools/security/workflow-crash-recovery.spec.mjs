import assert from 'node:assert/strict';
import { cpSync, mkdtempSync, mkdirSync, writeFileSync, existsSync } from 'node:fs';
import { execFileSync, spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

const source = new URL('./workflow-crash-recovery-feature-tests.mjs', import.meta.url);

function git(root, ...args) {
  return execFileSync('git', ['-C', root, ...args], { encoding: 'utf8' });
}
function fixture() {
  const root = mkdtempSync(join(tmpdir(), 'workflow-recovery-'));
  mkdirSync(join(root, 'tools/security'), { recursive: true });
  cpSync(source, join(root, 'tools/security/workflow-crash-recovery-feature-tests.mjs'));
  writeFileSync(join(root, 'tracked.txt'), 'baseline\n');
  git(root, 'init', '-q');
  git(root, 'config', 'user.email', 'test@example.invalid');
  git(root, 'config', 'user.name', 'test');
  git(root, 'add', '.');
  git(root, 'commit', '-qm', 'baseline');
  return root;
}

// @spec:AC-1053
test('runner rejects tracked mutation before Cargo and rejects rename origin', () => {
  const root = fixture();
  writeFileSync(join(root, 'tracked.txt'), 'mutated\n');
  const cargo = join(root, 'cargo-stub');
  writeFileSync(cargo, `#!/bin/sh\nprintf ran > ${join(root, 'cargo-ran')}\nexit 0\n`);
  const run = spawnSync(process.execPath, [join(root, 'tools/security/workflow-crash-recovery-feature-tests.mjs')], {
    env: { ...process.env, HANK_RUNNER_ROOT: root, HANK_ALLOW_GENERATED_EVIDENCE: '1', PATH: `${root}:${process.env.PATH}` },
    encoding: 'utf8',
  });
  assert.notEqual(run.status, 0);
  assert.equal(existsSync(join(root, 'cargo-ran')), false);

  git(root, 'restore', '.');
  mkdirSync(join(root, 'outside'), { recursive: true });
  writeFileSync(join(root, 'outside/original.txt'), 'x\n');
  git(root, 'add', '.');
  git(root, 'commit', '-qm', 'outside');
  mkdirSync(join(root, '.spec/verification'), { recursive: true });
  git(root, 'mv', 'outside/original.txt', '.spec/verification/current.txt');
  const renamed = spawnSync(process.execPath, [join(root, 'tools/security/workflow-crash-recovery-feature-tests.mjs')], {
    env: { ...process.env, HANK_RUNNER_ROOT: root, HANK_ALLOW_GENERATED_EVIDENCE: '1', PATH: `${root}:${process.env.PATH}` },
    encoding: 'utf8',
  });
  assert.notEqual(renamed.status, 0);
  assert.equal(existsSync(join(root, 'cargo-ran')), false);
});
