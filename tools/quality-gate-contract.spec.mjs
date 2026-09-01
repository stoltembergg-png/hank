import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const root = new URL('../', import.meta.url);
const read = (path) => readFileSync(new URL(path, root), 'utf8');

const rustWorkflow = read('.github/workflows/build-rust.yml');
const frontendWorkflow = read('.github/workflows/build-frontend.yml');
const tauriWorkflow = read('.github/workflows/build-tauri.yml');
const codeqlWorkflow = read('.github/workflows/codeql.yml');
const qualityIntegrityWorkflow = read('.github/workflows/quality-integrity.yml');

function assertCommand(workflow, command, file) {
  assert.match(workflow, new RegExp(command.replace(/[.*+?^${}()|[\\]\\\\]/g, '\\\\$&')), `${file}: missing ${command}`);
}

function assertJobStepRun(workflow, jobName, expectedRunSubstring, file) {
  const escaped = expectedRunSubstring.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const stepRegex = new RegExp(
    '- name: ' + jobName + '[\\s\\S]*?run:\\s*' + escaped
  );
  assert.match(workflow, stepRegex, `${file}: job ${jobName} missing run containing "${expectedRunSubstring}"`);
}

test('Rust workflow has fail-closed quality commands', () => {
  assertCommand(rustWorkflow, 'cargo fmt --all -- --check', 'build-rust.yml');
  assertCommand(rustWorkflow, 'cargo test --workspace --locked', 'build-rust.yml');
  assertCommand(rustWorkflow, 'cargo clippy --workspace --all-targets --locked -- -D warnings', 'build-rust.yml');
  assert.doesNotMatch(rustWorkflow, /continue-on-error\s*:\s*true/);
});

test('Frontend workflow has explicit lint, typecheck, test, and build gates', () => {
  assertCommand(frontendWorkflow, 'npm run lint', 'build-frontend.yml');
  assertCommand(frontendWorkflow, 'npm run typecheck', 'build-frontend.yml');
  assertCommand(frontendWorkflow, 'npm run test', 'build-frontend.yml');
  assertCommand(frontendWorkflow, 'npm run build', 'build-frontend.yml');
  assertCommand(frontendWorkflow, 'npm audit --audit-level=high', 'build-frontend.yml');
  assert.doesNotMatch(frontendWorkflow, /continue-on-error\s*:\s*true/);
});

test('Tauri workflow retains native check, format, and acceptance gates', () => {
  assertCommand(tauriWorkflow, 'cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml --locked', 'build-tauri.yml');
  assertCommand(tauriWorkflow, 'cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml -- --check', 'build-tauri.yml');
  assertCommand(tauriWorkflow, 'cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked', 'build-tauri.yml');
  assert.doesNotMatch(tauriWorkflow, /continue-on-error\s*:\s*true/);
});

test('CodeQL workflow is pinned, scoped, and fail-closed', () => {
  assert.match(codeqlWorkflow, /security-events:\s*write/);
  assert.match(codeqlWorkflow, /languages:\s*\$\{\{\s*matrix\.language\s*\}\}/);
  assert.match(codeqlWorkflow, /github\/codeql-action\/init@[0-9a-f]{40}/);
  assert.match(codeqlWorkflow, /github\/codeql-action\/autobuild@[0-9a-f]{40}/);
  assert.match(codeqlWorkflow, /github\/codeql-action\/analyze@[0-9a-f]{40}/);
  assert.match(codeqlWorkflow, /matrix\.language == 'rust'/);
  assert.doesNotMatch(codeqlWorkflow, /continue-on-error\s*:\s*true/);
});

test('quality integrity validates the hardened automated reviewer policy', () => {
  assertJobStepRun(
    qualityIntegrityWorkflow,
    'Validate automated reviewer policy',
    'node --test tools/reviewer-policy-check.spec.mjs',
    'quality-integrity.yml',
  );
});