#!/usr/bin/env bash
set -euo pipefail

: "${HANK_DESKTOP_BIN:?HANK_DESKTOP_BIN is required}"
: "${HANK_E2E_APP_DATA_DIR:?HANK_E2E_APP_DATA_DIR is required}"
: "${HANK_DESKTOP_E2E_ARTIFACTS:?HANK_DESKTOP_E2E_ARTIFACTS is required}"

if [[ "${HANK_UPDATER_E2E:-0}" == '1' && -z "${HANK_UPDATER_PUBLIC_KEY_DER_B64:-}" ]]; then
  node_binary="${HANK_NODE_BIN:-node}"
  updater_fixture="$($node_binary desktop-e2e/updater-fixture.mjs)"
  export HANK_UPDATER_PUBLIC_KEY_DER_B64="$($node_binary --input-type=module -e "const fixture=JSON.parse(process.argv[1]); process.stdout.write(fixture.publicKeyDerB64)" "$updater_fixture")"
  export HANK_UPDATER_PRIVATE_KEY_DER_B64="$($node_binary --input-type=module -e "const fixture=JSON.parse(process.argv[1]); process.stdout.write(fixture.privateKeyDerB64)" "$updater_fixture")"
  export HANK_UPDATER_CURRENT_VERSION="1"
  export HANK_UPDATER_EVENT="workflow_dispatch"
  export HANK_UPDATER_WORKFLOW="release.yml"
  export HANK_UPDATER_POLICY="updater-v1"
  export HANK_UPDATER_KEY_ID="e2e-fixture-v1"
  export HANK_UPDATER_CHANNEL="stable"
fi

port="${HANK_WEBDRIVER_PORT:-4444}"
mkdir -p "$HANK_DESKTOP_E2E_ARTIFACTS" "$HANK_E2E_APP_DATA_DIR"

frontend_pid=''
tauri_driver_pid=''
cleanup() {
  status=$?
  set +e
  if [[ -n "$tauri_driver_pid" ]] && kill -0 "$tauri_driver_pid" 2>/dev/null; then
    kill "$tauri_driver_pid" 2>/dev/null
    wait "$tauri_driver_pid" 2>/dev/null
  fi
  if [[ -n "$frontend_pid" ]] && kill -0 "$frontend_pid" 2>/dev/null; then
    kill "$frontend_pid" 2>/dev/null
    wait "$frontend_pid" 2>/dev/null
  fi
  exit "$status"
}
trap cleanup EXIT

npm --prefix frontend run preview -- --host 127.0.0.1 --port 1420 \
  >"$HANK_DESKTOP_E2E_ARTIFACTS/frontend-preview.log" 2>&1 &
frontend_pid=$!
for attempt in $(seq 1 60); do
  if curl --fail --silent "http://127.0.0.1:1420/" >/dev/null 2>&1; then break; fi
  if ! kill -0 "$frontend_pid" 2>/dev/null; then
    cat "$HANK_DESKTOP_E2E_ARTIFACTS/frontend-preview.log" >&2
    exit 1
  fi
  sleep 1
done
curl --fail --silent "http://127.0.0.1:1420/" >/dev/null

# Safari's WebDriver is the native WebKit driver on macOS. Enabling it is
# idempotent on hosted runners and keeps local execution explicit.
safaridriver --enable >/dev/null 2>&1 || true
tauri-driver --port "$port" --native-driver /usr/bin/safaridriver \
  >"$HANK_DESKTOP_E2E_ARTIFACTS/tauri-driver.log" 2>&1 &
tauri_driver_pid=$!

for attempt in $(seq 1 60); do
  if curl --fail --silent "http://127.0.0.1:${port}/status" >/dev/null 2>&1; then break; fi
  if ! kill -0 "$tauri_driver_pid" 2>/dev/null; then
    cat "$HANK_DESKTOP_E2E_ARTIFACTS/tauri-driver.log" >&2
    exit 1
  fi
  sleep 1
done
curl --fail --silent "http://127.0.0.1:${port}/status" >/dev/null
npm --prefix desktop-e2e test
