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
sbom="$release_root/SBOM.spdx.json"
report_path="${HANK_INSTALL_SMOKE_REPORT:-$release_root/install-smoke-report.json}"
status=FAIL
exit_code=1
upgrade_rollback=NO_PROOF
profile_preserved=FAIL
profile_marker=''

cleanup() {
  local code=$?
  set +e
  if [[ -f "${HANK_DESKTOP_E2E_ARTIFACTS:-}/updater-rollback-report.json" ]]; then
    if jq -e '.status == "PASS" and .evidenceScope == "protected-release-signed-artifact"' "${HANK_DESKTOP_E2E_ARTIFACTS}/updater-rollback-report.json" >/dev/null 2>&1; then
      upgrade_rollback=PASS
    elif jq -e '.status == "PASS" and .evidenceScope == "native-synthetic-signed-fixture"' "${HANK_DESKTOP_E2E_ARTIFACTS}/updater-rollback-report.json" >/dev/null 2>&1; then
      upgrade_rollback=PASS_LIMITED
    fi
  fi
  if [[ "$code" -eq 0 ]]; then
    if [[ -n "${HANK_E2E_APP_DATA_DIR:-}" && -d "$HANK_E2E_APP_DATA_DIR" ]]; then
      rm -rf -- "$HANK_E2E_APP_DATA_DIR"
    fi
    if [[ -n "${HANK_DESKTOP_E2E_ARTIFACTS:-}" && -d "$HANK_DESKTOP_E2E_ARTIFACTS" ]]; then
      rm -rf -- "$HANK_DESKTOP_E2E_ARTIFACTS"
    fi
  fi
  mkdir -p "$(dirname "$report_path")"
  printf '{"status":"%s","releaseTag":"%s","expectedCommit":"%s","expectedTree":"%s","platform":"linux-x86_64","appImage":"hank-%s-x86_64.AppImage","appImageDigest":"%s","sbom":"SBOM.spdx.json","sbomDigest":"%s","uninstall":"not_applicable_portable","profilePreserved":"%s","upgradeRollback":"%s"}\n' \
    "$status" "$HANK_RELEASE_TAG" "$HANK_EXPECTED_COMMIT" "$HANK_EXPECTED_TREE" "$HANK_RELEASE_TAG" \
    "$(sha256sum "$appimage" 2>/dev/null | awk '{print $1}')" \
    "$(sha256sum "$sbom" 2>/dev/null | awk '{print $1}')" "$profile_preserved" "$upgrade_rollback" > "$report_path"
  exit "$code"
}
trap cleanup EXIT

for required in "$manifest" "$manifest_sha" "$checksums" "$archive" "$installer" "$appimage" "$sbom"; do
  test -f "$required"
done

sha256sum -c "$manifest_sha"
sha256sum -c "$checksums"
node "$PWD/tools/release-prerelease.mjs" verify-artifacts --manifest "$manifest" --directory "$release_root"
node "$PWD/tools/release-artifact-signing.mjs" verify \
  --directory "$release_root" \
  --attestationDirectory "$release_root" \
  --artifacts "hank-${HANK_RELEASE_TAG}.tar.gz,hank-${HANK_RELEASE_TAG}-setup.exe,hank-${HANK_RELEASE_TAG}-x86_64.AppImage" \
  --commit "$HANK_EXPECTED_COMMIT" \
  --tree "$HANK_EXPECTED_TREE"
sbom_version=$(jq -r '.version' "$manifest")
node "$PWD/tools/release-sbom.mjs" verify \
  --file "$sbom" \
  --commit "$HANK_EXPECTED_COMMIT" \
  --tree "$HANK_EXPECTED_TREE" \
  --version "$sbom_version"
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
profile_marker="$HANK_E2E_APP_DATA_DIR/release-smoke-profile-marker"
printf '%s\n' 'preserve-profile' > "$profile_marker"
export HANK_E2E_ALLOW_RELEASE_DATA_DIR=1
export HANK_E2E_MOCK_PROVIDER=1
export HANK_WEBDRIVER_PORT="${HANK_WEBDRIVER_PORT:-4444}"
export HANK_EXPECTED_COMMIT_SHA="$HANK_EXPECTED_COMMIT"
export HANK_EXPECTED_TREE_SHA="$HANK_EXPECTED_TREE"
export HANK_REQUIRE_ARTIFACT_PROVENANCE=1
export HANK_UPDATER_E2E="${HANK_UPDATER_E2E:-0}"
if [[ "$HANK_UPDATER_E2E" == '1' && -f "$release_root/hank-${HANK_RELEASE_TAG}-linux-updater.json" ]]; then
  export HANK_UPDATER_RELEASE_BUNDLE="$release_root/hank-${HANK_RELEASE_TAG}-linux-updater.json"
  export HANK_UPDATER_RELEASE_ARTIFACT="$appimage"
  export HANK_UPDATER_RELEASE_SIGNING_METADATA="$release_root/release-signing-metadata.json"
elif [[ "$HANK_UPDATER_E2E" == '1' && "${HANK_UPDATER_REQUIRE_RELEASE_BUNDLE:-0}" == '1' ]]; then
  echo 'protected updater bundle is required but missing' >&2
  exit 1
fi

set +e
xvfb-run -a --server-args='-screen 0 1280x1024x24' bash "$PWD/desktop-e2e/run-linux.sh"
exit_code=$?
set -e
if [[ "$exit_code" -ne 0 ]]; then
  exit "$exit_code"
fi
test -f "$profile_marker"
test "$(<"$profile_marker")" = 'preserve-profile'
profile_preserved=PASS
status=PASS
exit_code=0
echo "release install smoke launch: PASS ($HANK_RELEASE_TAG)"
