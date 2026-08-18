import { existsSync } from 'node:fs';
import { spawnSync } from 'node:child_process';

const commands = [
  {
    label: 'W0 contract tests',
    command: process.execPath,
    args: ['--test', 'test/w0-contract-closure.spec.test.js'],
  },
  {
    label: 'Rust workspace tests',
    command: 'cargo',
    args: ['test', '--workspace'],
  },
];

if (existsSync('apps/desktop/src-tauri/Cargo.toml')) {
  commands.push({
    label: 'Tauri acceptance tests',
    command: 'cargo',
    args: ['test', '--manifest-path', 'apps/desktop/src-tauri/Cargo.toml', '--locked'],
  });
}

for (const { label, command, args } of commands) {
  console.log(`\n▶ ${label}: ${command} ${args.join(' ')}`);
  const result = spawnSync(command, args, { stdio: 'inherit' });

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
