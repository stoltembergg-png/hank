#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import { writeFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

const VERSION = /^v?\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;

export function buildManifest({ version, sha, tree }) {
  if (!VERSION.test(version || '')) throw new Error(`invalid release version: ${version}`);
  if (!/^[0-9a-f]{40}$/.test(sha || '') || !/^[0-9a-f]{40}$/.test(tree || '')) {
    throw new Error('release identity requires full commit and tree SHA');
  }
  return {
    version: version.startsWith('v') ? version.slice(1) : version,
    sha,
    tree,
    mode: 'dry-run',
    publish: false,
    sign: false,
    stopBeforePublish: true,
    artifacts: [],
  };
}

function main() {
  const versionIndex = process.argv.indexOf('--version');
  const outputIndex = process.argv.indexOf('--output');
  const version = versionIndex >= 0 ? process.argv[versionIndex + 1] : null;
  const output = outputIndex >= 0 ? process.argv[outputIndex + 1] : null;
  const sha = execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
  const tree = execFileSync('git', ['rev-parse', 'HEAD^{tree}'], { encoding: 'utf8' }).trim();
  const manifest = `${JSON.stringify(buildManifest({ version, sha, tree }), null, 2)}\n`;
  if (output) writeFileSync(output, manifest);
  else process.stdout.write(manifest);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) main();
