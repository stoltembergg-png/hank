import assert from 'node:assert/strict';
import test from 'node:test';
import { readFile } from 'node:fs/promises';
import { resolveDesktopDevCommand } from '../tools/run-desktop-dev.mjs';

const projectRoot = '/workspace/hank';

test('desktop development launcher runs Tauri CLI from the Tauri crate directory', () => {
  const command = resolveDesktopDevCommand(projectRoot, 'linux');

  assert.equal(command.cwd, `${projectRoot}/apps/desktop/src-tauri`);
  assert.equal(command.args[0], 'dev');
  assert.equal(command.command, `${projectRoot}/frontend/node_modules/.bin/tauri`);
});

test('Tauri dev hook starts Vite from the explicit frontend directory', async () => {
  const config = JSON.parse(await readFile('apps/desktop/src-tauri/tauri.conf.json', 'utf8'));

  assert.deepEqual(config.build.beforeDevCommand, {
    script: 'npm run dev',
    cwd: '../../../frontend',
    wait: false,
  });
  assert.deepEqual(config.build.beforeBuildCommand, {
    script: 'npm run build',
    cwd: '../../../frontend',
  });
});
