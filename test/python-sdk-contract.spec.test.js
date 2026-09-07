import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { existsSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const repoRoot = fileURLToPath(new URL('..', import.meta.url));
const moduleName = 'python.tests.test_sdk.PythonWorkerSdkTests';

function resolvePythonCommand() {
  if (process.env.HANK_PYTHON?.trim()) {
    return { executable: process.env.HANK_PYTHON.trim(), args: [] };
  }
  if (process.platform !== 'win32') {
    return { executable: 'python3', args: [] };
  }

  const roots = [
    process.env.LOCALAPPDATA && path.join(process.env.LOCALAPPDATA, 'Programs', 'Python'),
    process.env.ProgramFiles && path.join(process.env.ProgramFiles, 'Python'),
    process.env.USERPROFILE && path.join(process.env.USERPROFILE, 'AppData', 'Local', 'Programs', 'Python'),
  ].filter(Boolean);
  for (const root of roots) {
    let entries;
    try {
      entries = readdirSync(root, { withFileTypes: true })
        .filter((entry) => entry.isDirectory() && /^Python\d+(?:\.\d+)?$/i.test(entry.name))
        .sort((left, right) => right.name.localeCompare(left.name, undefined, { numeric: true }));
    } catch {
      continue;
    }
    for (const entry of entries) {
      const executable = path.join(root, entry.name, 'python.exe');
      if (existsSync(executable)) return { executable, args: [] };
    }
  }

  const launcher = spawnSync('where.exe', ['py.exe'], { stdio: 'ignore' });
  if (launcher.status === 0) return { executable: 'py.exe', args: ['-3'] };
  return { executable: 'python.exe', args: [] };
}

const python = resolvePythonCommand();

function runPythonTest(testName) {
  execFileSync(
    python.executable,
    [...python.args, '-m', 'unittest', `${moduleName}.${testName}`],
    { cwd: repoRoot, stdio: 'pipe', encoding: 'utf8' },
  );
}

test('AC-699: SDK handshake and request are correlated @spec:AC-699', () => {
  assert.doesNotThrow(() => runPythonTest('test_handshake_and_request_require_bounded_context'));
});

test('AC-700: invalid SDK inputs fail before write @spec:AC-700', () => {
  assert.doesNotThrow(() => runPythonTest('test_invalid_context_and_oversized_payload_fail_before_write'));
});

test('AC-701: SDK cancel and shutdown follow protocol @spec:AC-701', () => {
  assert.doesNotThrow(() => runPythonTest('test_cancel_is_notification_and_shutdown_is_correlated'));
});

test('AC-702: SDK errors are bounded and redacted @spec:AC-702', () => {
  assert.doesNotThrow(() => runPythonTest('test_protocol_error_is_redacted'));
});

test('AC-703: SDK does not grant execution @spec:AC-703', () => {
  assert.doesNotThrow(() => runPythonTest('test_handshake_and_request_require_bounded_context'));
});
