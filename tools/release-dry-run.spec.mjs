import assert from 'node:assert/strict';
import test from 'node:test';
import { buildManifest } from './release-dry-run.mjs';

const sha = '1'.repeat(40);
const tree = '2'.repeat(40);

test('builds a non-publishing release manifest', () => {
  const manifest = buildManifest({ version: 'v1.2.3-rc.1', sha, tree });
  assert.deepEqual(manifest, {
    version: '1.2.3-rc.1', sha, tree, mode: 'dry-run', publish: false,
    sign: false, stopBeforePublish: true, artifacts: [],
  });
});

test('rejects invalid versions and incomplete identity', () => {
  assert.throws(() => buildManifest({ version: 'latest', sha, tree }), /invalid release version/);
  assert.throws(() => buildManifest({ version: '1.0.0', sha: 'deadbeef', tree }), /full commit/);
});
