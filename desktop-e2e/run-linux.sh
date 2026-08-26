#!/usr/bin/env bash
set -euo pipefail

: "${HANK_DESKTOP_BIN:?HANK_DESKTOP_BIN is required}"
: "${HANK_E2E_APP_DATA_DIR:?HANK_E2E_APP_DATA_DIR is required}"
: "${HANK_DESKTOP_E2E_ARTIFACTS:?HANK_DESKTOP_E2E_ARTIFACTS is required}"

port="${HANK_WEBDRIVER_PORT:-4444}"
mkdir -p "$HANK_DESKTOP_E2E_ARTIFACTS"

tauri_driver_pid=''
cleanup() {
  status=$?
  if [[ -n "$tauri_driver_pid" ]] && kill -0 "$tauri_driver_pid" 2>/dev/null; then
    kill "$tauri_driver_pid" 2>/dev/null || true
    wait "$tauri_driver_pid" 2>/dev/null || true
  fi
  exit "$status"
}
trap cleanup EXIT

tauri-driver --port "$port" --native-driver /usr/bin/WebKitWebDriver \
  >"$HANK_DESKTOP_E2E_ARTIFACTS/tauri-driver.log" 2>&1 &
tauri_driver_pid=$!

for attempt in $(seq 1 60); do
  if curl --fail --silent "http://127.0.0.1:${port}/status" >/dev/null 2>&1; then
    break
  fi
  if ! kill -0 "$tauri_driver_pid" 2>/dev/null; then
    cat "$HANK_DESKTOP_E2E_ARTIFACTS/tauri-driver.log" >&2
    exit 1
  fi
  sleep 1
done

curl --fail --silent "http://127.0.0.1:${port}/status" >/dev/null
npm --prefix desktop-e2e test
