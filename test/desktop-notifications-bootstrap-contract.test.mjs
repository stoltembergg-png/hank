import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8');

test('desktop bootstrap wires the bounded notification worker @spec:AC-1295 @spec:AC-1296', () => {
  const bootstrap = read('apps/desktop/src-tauri/src/main.rs');
  const adapter = read('apps/desktop/src-tauri/src/notifications.rs');
  const runtime = read('crates/agent-runtime/src/notifications.rs');

  assert.match(bootstrap, /\.plugin\(tauri_plugin_notification::init\(\)\)/);
  assert.match(bootstrap, /NotificationWorker::new\(/);
  assert.match(bootstrap, /TauriNotificationSink::new\(app\.handle\(\)\.clone\(\)\)/);
  assert.match(adapter, /pub fn request_permission\(&self\)/);
  assert.match(adapter, /fn permission\(&self\) -> RuntimePermissionState/);
  assert.doesNotMatch(runtime, /tauri/);
});
