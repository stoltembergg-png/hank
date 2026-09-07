#!/usr/bin/env bash
set -euo pipefail

: "${HANK_RELEASE_DIR:?HANK_RELEASE_DIR is required}"
: "${HANK_RELEASE_TAG:?HANK_RELEASE_TAG is required}"
: "${HANK_EXPECTED_COMMIT:?HANK_EXPECTED_COMMIT is required}"
: "${HANK_EXPECTED_TREE:?HANK_EXPECTED_TREE is required}"
: "${HANK_NODE_BIN:?HANK_NODE_BIN is required}"

release_root=$(cd "$HANK_RELEASE_DIR" && pwd)
manifest="$release_root/release-manifest.json"
manifest_sha="$release_root/manifest.sha256"
checksums="$release_root/SHA256SUMS"
archive="$release_root/hank-${HANK_RELEASE_TAG}.tar.gz"
installer="$release_root/hank-${HANK_RELEASE_TAG}-setup.exe"
appimage="$release_root/hank-${HANK_RELEASE_TAG}-x86_64.AppImage"
report_path="${HANK_INSTALL_SMOKE_REPORT:-$release_root/install-smoke-report.json}"
status=FAIL
exit_code=1

cleanup() {
  local code=$?
  set +e
  if [[ "$code" -eq 0 ]]; then
    if [[ -n "${HANK_E2E_APP_DATA_DIR:-}" && -d "$HANK_E2E_APP_DATA_DIR" ]]; then
      rm -rf -- "$HANK_E2E_APP_DATA_DIR"
    fi
    if [[ -n "${HANK_DESKTOP_E2E_ARTIFACTS:-}" && -d "$HANK_DESKTOP_E2E_ARTIFACTS" ]]; then
      rm -rf -- "$HANK_DESKTOP_E2E_ARTIFACTS"
    fi
  fi
  mkdir -p "$(dirname "$report_path")"
  printf '{"status":"%s","releaseTag":"%s","expectedCommit":"%s","expectedTree":"%s","platform":"linux-x86_64","appImage":"hank-%s-x86_64.AppImage","appImageDigest":"%s","upgradeRollback":"NO_PROOF"}\n' \
    "$status" "$HANK_RELEASE_TAG" "$HANK_EXPECTED_COMMIT" "$HANK_EXPECTED_TREE" "$HANK_RELEASE_TAG" \
    "$(sha256sum "$appimage" 2>/dev/null | awk '{print $1}')" > "$report_path"
  exit "$code"
}
trap cleanup EXIT

for required in "$manifest" "$manifest_sha" "$checksums" "$archive" "$installer" "$appimage"; do
  test -f "$required"
done

sha256sum -c "$manifest_sha"
sha256sum -c "$checksums"
node "$PWD/tools/release-prerelease.mjs" verify-artifacts --manifest "$manifest" --directory "$release_root"
node --input-type=module - "$manifest" "$HANK_RELEASE_TAG" "$HANK_EXPECTED_COMMIT" "$HANK_EXPECTED_TREE" <<'NODE'
import fs from 'node:fs';
const [manifestPath, expectedTag, expectedCommit, expectedTree] = process.argv.slice(2);
const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
if (manifest.tag !== expectedTag || manifest.prerelease !== true || manifest.stable !== false) throw new Error('release manifest state mismatch');
if (manifest.sha !== expectedCommit || manifest.tree !== expectedTree) throw new Error('release manifest identity mismatch');
NODE

chmod +x "$appimage"
export HANK_DESKTOP_BIN="$appimage"
export HANK_E2E_APP_DATA_DIR="${HANK_E2E_APP_DATA_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/hank-release-e2e-data.XXXXXX")}"
export HANK_DESKTOP_E2E_ARTIFACTS="${HANK_DESKTOP_E2E_ARTIFACTS:-$(mktemp -d "${TMPDIR:-/tmp}/hank-release-e2e-artifacts.XXXXXX")}"
export HANK_E2E_ALLOW_RELEASE_DATA_DIR=1
export HANK_E2E_MOCK_PROVIDER=1
export HANK_WEBDRIVER_PORT="${HANK_WEBDRIVER_PORT:-4444}"
export HANK_EXPECTED_COMMIT_SHA="$HANK_EXPECTED_COMMIT"
export HANK_EXPECTED_TREE_SHA="$HANK_EXPECTED_TREE"
export HANK_REQUIRE_ARTIFACT_PROVENANCE=1

set +e
xvfb-run -a --server-args='-screen 0 1280x1024x24' bash "$PWD/desktop-e2e/run-linux.sh"
exit_code=$?
set -e
if [[ "$exit_code" -ne 0 ]]; then
  exit "$exit_code"
fi
status=PASS
exit_code=0
echo "release install smoke launch: PASS ($HANK_RELEASE_TAG)"
