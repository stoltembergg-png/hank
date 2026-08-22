import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { verifyDesktopFrontendAssets } from '../tools/verify-desktop-frontend-assets.mjs';

test('rejects absolute and missing frontend asset references for desktop packaging', async () => {
  const root = await mkdtemp(join(tmpdir(), 'hank-desktop-assets-'));
  const dist = join(root, 'dist');
  await mkdir(dist);
  await writeFile(
    join(dist, 'index.html'),
    '<script type="module" src="/assets/missing.js"></script>',
  );

  const result = await verifyDesktopFrontendAssets(dist);

  assert.equal(result.ok, false);
  assert.deepEqual(result.errors, [
    'asset reference must be relative: /assets/missing.js',
  ]);
});

test('accepts relative frontend assets that exist under dist', async () => {
  const root = await mkdtemp(join(tmpdir(), 'hank-desktop-assets-'));
  const dist = join(root, 'dist');
  await mkdir(join(dist, 'assets'), { recursive: true });
  await writeFile(
    join(dist, 'index.html'),
    '<link rel="stylesheet" href="./assets/app.css"><script src="./assets/app.js"></script>',
  );
  await writeFile(join(dist, 'assets', 'app.css'), '');
  await writeFile(join(dist, 'assets', 'app.js'), '');

  const result = await verifyDesktopFrontendAssets(dist);

  assert.deepEqual(result, { ok: true, errors: [] });
});
