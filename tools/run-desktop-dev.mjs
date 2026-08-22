#!/usr/bin/env node
import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export function resolveDesktopDevCommand(projectRoot, platform = process.platform) {
  const root = resolve(projectRoot);
  const executable = platform === 'win32' ? 'tauri.cmd' : 'tauri';
  return {
    command: resolve(root, 'frontend', 'node_modules', '.bin', executable),
    args: ['dev'],
    cwd: resolve(root, 'apps', 'desktop', 'src-tauri'),
  };
}

function main() {
  const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
  const launch = resolveDesktopDevCommand(root);
  if (!existsSync(launch.command)) {
    console.error('Tauri CLI is unavailable; run npm --prefix frontend ci first.');
    process.exitCode = 1;
    return;
  }
  const child = spawn(launch.command, launch.args, {
    cwd: launch.cwd,
    stdio: 'inherit',
  });
  child.once('error', (error) => {
    console.error(`desktop dev launcher failed: ${error.message}`);
    process.exitCode = 1;
  });
  child.once('exit', (code, signal) => {
    process.exitCode = code ?? (signal ? 1 : 0);
  });
}

if (import.meta.url === new URL(process.argv[1], 'file:').href) main();
