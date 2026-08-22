#!/usr/bin/env node
import { readFile, stat } from 'node:fs/promises';
import { resolve, sep } from 'node:path';

const ASSET_REFERENCE = /(?:src|href)="([^"]+)"/g;

export async function verifyDesktopFrontendAssets(distDir) {
  const distRoot = resolve(distDir);
  const indexPath = resolve(distRoot, 'index.html');
  const html = await readFile(indexPath, 'utf8');
  const errors = [];

  for (const match of html.matchAll(ASSET_REFERENCE)) {
    const reference = match[1];
    if (reference.startsWith(('http:', 'https:', 'data:', '#'))) continue;
    if (reference.startsWith('/') || reference.startsWith('\\')) {
      errors.push(`asset reference must be relative: ${reference}`);
      continue;
    }

    const target = resolve(distRoot, reference);
    if (target !== distRoot && !target.startsWith(`${distRoot}${sep}`)) {
      errors.push(`asset reference escapes dist: ${reference}`);
      continue;
    }
    try {
      await stat(target);
    } catch {
      errors.push(`asset reference is missing: ${reference}`);
    }
  }

  return { ok: errors.length === 0, errors };
}

async function main() {
  const distDir = process.argv[2] ?? 'frontend/dist';
  const result = await verifyDesktopFrontendAssets(distDir);
  if (!result.ok) {
    for (const error of result.errors) console.error(error);
    process.exitCode = 1;
  }
}

if (import.meta.url === new URL(process.argv[1], 'file:').href) {
  main().catch((error) => {
    console.error(`desktop frontend asset verification failed: ${error.message}`);
    process.exitCode = 1;
  });
}
