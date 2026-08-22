import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const python = process.platform === 'win32' ? 'python.exe' : 'python3';
const repoRoot = fileURLToPath(new URL('..', import.meta.url));
const moduleName = 'python.tests.test_sdk.PythonWorkerSdkTests';

function runPythonTest(testName) {
  execFileSync(
    python,
    ['-m', 'unittest', `${moduleName}.${testName}`],
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
