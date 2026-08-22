import assert from 'node:assert/strict';
import test from 'node:test';
import { resolveDesktopDevCommand } from '../tools/run-desktop-dev.mjs';

test('desktop development launcher runs Tauri CLI from the Tauri crate directory', () => {
  const command = resolveDesktopDevCommand('/workspace/hank', 'linux');

  assert.equal(command.cwd, '/workspace/hank/apps/desktop/src-tauri');
  assert.equal(command.args[0], 'dev');
  assert.equal(command.command, '/workspace/hank/frontend/node_modules/.bin/tauri');
});
