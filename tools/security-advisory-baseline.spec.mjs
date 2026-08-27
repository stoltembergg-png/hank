import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const root = new URL('../', import.meta.url);
const read = (path) => readFileSync(new URL(path, root), 'utf8');
const baseline = JSON.parse(read('security/advisory-baseline.json'));
const exception = baseline.exceptions[0];
const workflow = read('.github/workflows/security-advisory.yml');
const dependabot = read('.github/dependabot.yml');
const record = read('docs/security/advisory-exceptions/RUSTSEC-2024-0429.md');

test('glib exception is singular, granular, and explicitly upstream', () => {
  assert.equal(baseline.schema_version, 1);
  assert.equal(baseline.exceptions.length, 1);
  assert.equal(exception.advisory, 'RUSTSEC-2024-0429');
  assert.deepEqual(exception.aliases, ['GHSA-wrw7-89jp-8q8g']);
  assert.equal(exception.dependency, 'glib');
  assert.equal(exception.classification, 'UPSTREAM_TRANSITIVE_DEPENDENCY');
  assert.equal(exception.status, 'OPEN / ACCEPTED UPSTREAM RISK');
  assert.equal(exception.mitigation_status, 'NO_COMPATIBLE_UPSTREAM_FIX_AVAILABLE');
  assert.equal(exception.patched_upstream, '>= 0.20.0');
  assert.match(record, /RUSTSEC-2024-0429/);
  assert.match(record, /DIRECT_USAGE`:\s*`NO/);
  assert.match(record, /REACHABLE_FROM_HANK_CODE`:\s*`UNKNOWN/);
  assert.match(record, /TRANSITIVE_ONLY`:\s*`YES/);
});

test('security workflow preserves audit output and rejects new advisories', () => {
  assert.match(workflow, /cargo audit --json --file Cargo\.lock/);
  assert.match(workflow, /cargo audit --json --file apps\/desktop\/src-tauri\/Cargo\.lock/);
  assert.match(workflow, /cargo tree --locked --prefix none --format/);
  assert.match(workflow, /cargo tree --manifest-path apps\/desktop\/src-tauri\/Cargo\.toml/);
  assert.match(workflow, /check-rust-advisories\.mjs security\/advisory-baseline\.json security\/reports\/root\.json::security\/reports\/root\.tree/);
  assert.match(workflow, /actions\/upload-artifact@[0-9a-f]{40}/);
  assert.match(workflow, /Require audit artifact digest/);
  assert.match(workflow, /require-artifact-digest\.sh/);
  assert.match(workflow, /if: always\(\)/);
  assert.doesNotMatch(workflow, /continue-on-error\s*:\s*true/);
  assert.doesNotMatch(workflow, /--ignore/);
});

test('Dependabot watches the native Tauri dependency chain', () => {
  const start = dependabot.indexOf('directory: /apps/desktop/src-tauri');
  assert.notEqual(start, -1);
  const block = dependabot.slice(start, dependabot.indexOf('\n  - package-ecosystem:', start));
  for (const dependency of ['tauri', 'tauri-runtime-wry', 'wry', 'gtk', 'glib', 'glib-sys', 'webkit2gtk']) {
    assert.match(block, new RegExp(`dependency-name: ${dependency}`));
  }
});
