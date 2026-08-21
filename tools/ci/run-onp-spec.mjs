#!/usr/bin/env node
import { readFileSync } from 'node:fs';
import { resolve, relative, sep } from 'node:path';
import { pathToFileURL } from 'node:url';
import { onpRootFrom, portableTextSha256 } from './onp-bootstrap.mjs';

const root = onpRootFrom(import.meta.url);
const manifestPath = resolve(root, 'manifest.json');

export function verifyManifest(manifest = JSON.parse(readFileSync(manifestPath, 'utf8'))) {
  if (manifest.tool !== 'onp-spec-driven' || manifest.version !== '3.6.0') {
    throw new Error('unsupported or unpinned onp-spec tool manifest');
  }
  for (const [relativePath, expected] of Object.entries(manifest.files ?? {})) {
    const absolute = resolve(root, relativePath);
    const resolvedRelative = relative(root, absolute);
    if (!resolvedRelative || resolvedRelative.startsWith(`..${sep}`) || resolvedRelative === '..') {
      throw new Error(`unsafe ONP manifest path: ${relativePath}`);
    }
    const actual = portableTextSha256(readFileSync(absolute));
    if (actual !== expected) {
      throw new Error(`ONP tool checksum mismatch: ${relativePath}`);
    }
  }
  return true;
}

try {
  verifyManifest();
  await import(pathToFileURL(resolve(root, 'scripts/onp-spec.mjs')).href);
} catch (error) {
  console.error(`ONP bootstrap failed closed: ${error.message}`);
  process.exitCode = 1;
}
