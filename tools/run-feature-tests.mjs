import { existsSync, readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { basename, dirname, join, relative } from 'node:path';

const feature = process.argv[2];
if (!feature) {
  console.error('usage: node tools/run-feature-tests.mjs <feature>');
  process.exit(2);
}

const root = process.cwd();
const spec = join(root, '.spec', 'features', feature, 'spec.md');
if (!existsSync(spec)) {
  console.error(`feature spec not found: ${feature}`);
  process.exit(2);
}

const acs = new Set([...readFileSync(spec, 'utf8').matchAll(/AC-(\d{3,})/g)].map((m) => `AC-${m[1]}`));
const files = spawnSync('git', ['ls-files', '-co', '--exclude-standard'], {
  cwd: root,
  encoding: 'utf8',
}).stdout.split(/\r?\n/).filter(Boolean).filter((file) =>
  !file.startsWith('.hermes/') && !file.startsWith('node_modules/') &&
  /(?:^test\/|^tests\/|\/tests\/|\.test\.|_test\.|\.spec\.)/.test(file)
);
const tagged = files.filter((file) => {
  const text = readFileSync(join(root, file), 'utf8');
  return [...text.matchAll(/@spec:(AC-\d{3,})/g)].some((m) => acs.has(m[1]));
});

if (tagged.length === 0) {
  console.error(`no tagged test files found for ${feature}`);
  process.exit(1);
}

function cargoPackage(file) {
  let dir = join(root, dirname(file));
  while (dir.startsWith(root)) {
    const manifest = join(dir, 'Cargo.toml');
    if (existsSync(manifest)) {
      const match = readFileSync(manifest, 'utf8').match(/^name\s*=\s*"([^"]+)"/m);
      if (match) return { manifest, package: match[1] };
    }
    const parent = dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  return null;
}

const commands = [];
for (const file of tagged) {
  const ext = basename(file).split('.').pop();
  if (ext === 'js' || ext === 'mjs' || ext === 'cjs') {
    commands.push({ command: process.execPath, args: ['--test', file], label: file });
  } else if (ext === 'ts' || ext === 'tsx') {
    commands.push({ command: process.platform === 'win32' ? 'npm.cmd' : 'npm', args: ['--prefix', 'frontend', 'test', '--', '--run', file], label: file });
  } else if (ext === 'rs') {
    const pkg = cargoPackage(file);
    if (!pkg) continue;
    const target = /\/tests\//.test(file) ? basename(file, '.rs') : null;
    const args = ['test', '-p', pkg.package];
    if (target) args.push('--test', target);
    args.push('--locked');
    commands.push({ command: 'cargo', args, label: file });
  } else if (ext === 'py') {
    commands.push({ command: process.platform === 'win32' ? 'python.exe' : 'python3', args: ['-m', 'unittest', file], label: file });
  }
}

const unique = new Map(commands.map((item) => [`${item.command}\0${item.args.join('\0')}`, item]));
for (const { command, args, label } of unique.values()) {
  console.log(`▶ ${feature}: ${label} — ${command} ${args.join(' ')}`);
  const result = spawnSync(command, args, { cwd: root, stdio: 'inherit', shell: process.platform === 'win32' });
  if (result.error || result.status !== 0) {
    console.error(`✖ ${label} failed with exit code ${result.status ?? 1}`);
    process.exit(result.status ?? 1);
  }
}
console.log(`✔ ${feature}: ${unique.size} feature-scoped test command(s) passed`);
