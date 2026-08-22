import { existsSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

const defaultCargo = join(homedir(), '.cargo', 'bin', process.platform === 'win32' ? 'cargo.exe' : 'cargo');
const cargoCmd = existsSync(defaultCargo) ? defaultCargo : 'cargo';

const commands = [
  {
    label: 'Prerelease contract tests',
    command: process.execPath,
    args: ['--test', 'test/release-prerelease.js'],
  },
  {
    label: 'W0 contract tests',
    command: process.execPath,
    args: ['--test', 'test/w0-contract-closure.spec.test.js'],
  },
  {
    label: 'ONP bootstrap contract tests',
    command: process.execPath,
    args: ['--test', 'test/onp-bootstrap-paths.js'],
  },
  {
    label: 'Python SDK tests',
    command: process.platform === 'win32' ? 'python.exe' : 'python3',
    args: ['-m', 'unittest', 'discover', '-s', 'python/tests', '-p', 'test_*.py'],
  },
  {
    label: 'Rust workspace tests',
    command: cargoCmd,
    args: ['test', '--workspace', '--locked'],
  },
];

if (existsSync('frontend/package.json')) {
  commands.push({
    label: 'Frontend Vitest tests',
    command: process.platform === 'win32' ? 'npm.cmd' : 'npm',
    args: ['--prefix', 'frontend', 'run', 'test'],
  });
}

if (existsSync('apps/desktop/src-tauri/Cargo.toml')) {
  commands.push({
    label: 'Tauri acceptance tests',
    command: cargoCmd,
    args: ['test', '--manifest-path', 'apps/desktop/src-tauri/Cargo.toml', '--locked'],
  });
}

for (const { label, command, args } of commands) {
  console.log(`\n▶ ${label}: ${command} ${args.join(' ')}`);
  const result = spawnSync(command, args, { stdio: 'inherit', shell: process.platform === 'win32' });

  if (result.error) {
    console.error(`✖ ${label} could not start: ${result.error.message}`);
    process.exit(1);
  }

  if (result.status !== 0) {
    console.error(`✖ ${label} failed with exit code ${result.status ?? 1}`);
    process.exit(result.status ?? 1);
  }
}

console.log('\n✔ Selected Node, Rust, and Tauri test suites passed');
