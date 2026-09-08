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
dmg="$release_root/hank-${HANK_RELEASE_TAG}-aarch64.dmg"
sbom="$release_root/SBOM.spdx.json"
signing_metadata="$release_root/release-signing-metadata.json"
report_path="${HANK_INSTALL_SMOKE_REPORT:-$release_root/install-smoke-report.json}"
status=FAIL
exit_code=1
upgrade_rollback=NO_PROOF
profile_preserved=FAIL
mount_point=''

sha256() { shasum -a 256 "$1" | awk '{print $1}'; }

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
  if [[ -n "$mount_point" ]]; then hdiutil detach "$mount_point" -force >/dev/null 2>&1 || true; fi
  mkdir -p "$(dirname "$report_path")"
  printf '{"status":"%s","releaseTag":"%s","expectedCommit":"%s","expectedTree":"%s","platform":"macos-aarch64","dmg":"hank-%s-aarch64.dmg","dmgDigest":"%s","sbom":"SBOM.spdx.json","sbomDigest":"%s","uninstall":"%s","profilePreserved":"%s","upgradeRollback":"%s"}\n' \
    "$status" "$HANK_RELEASE_TAG" "$HANK_EXPECTED_COMMIT" "$HANK_EXPECTED_TREE" "$HANK_RELEASE_TAG" \
    "$(sha256 "$dmg" 2>/dev/null)" "$(sha256 "$sbom" 2>/dev/null)" \
    "$([[ "$code" -eq 0 ]] && echo passed || echo failed)" "$profile_preserved" "$upgrade_rollback" > "$report_path"
  exit "$code"
}
trap cleanup EXIT

for required in "$manifest" "$manifest_sha" "$checksums" "$archive" "$installer" "$appimage" "$dmg" "$sbom" "$signing_metadata" \
  "$release_root/hank-${HANK_RELEASE_TAG}.tar.gz.attestation.json" \
  "$release_root/hank-${HANK_RELEASE_TAG}-setup.exe.attestation.json" \
  "$release_root/hank-${HANK_RELEASE_TAG}-x86_64.AppImage.attestation.json" \
  "$release_root/hank-${HANK_RELEASE_TAG}-aarch64.dmg.attestation.json"; do test -f "$required"; done
shasum -a 256 -c "$manifest_sha"
shasum -a 256 -c "$checksums"
node "$PWD/tools/release-prerelease.mjs" verify-artifacts --manifest "$manifest" --directory "$release_root"
node "$PWD/tools/release-artifact-signing.mjs" verify \
  --directory "$release_root" --attestationDirectory "$release_root" \
  --artifacts "hank-${HANK_RELEASE_TAG}.tar.gz,hank-${HANK_RELEASE_TAG}-setup.exe,hank-${HANK_RELEASE_TAG}-x86_64.AppImage,hank-${HANK_RELEASE_TAG}-aarch64.dmg" \
  --commit "$HANK_EXPECTED_COMMIT" --tree "$HANK_EXPECTED_TREE"
sbom_version=$(jq -r '.version' "$manifest")
node "$PWD/tools/release-sbom.mjs" verify --file "$sbom" --commit "$HANK_EXPECTED_COMMIT" --tree "$HANK_EXPECTED_TREE" --version "$sbom_version"
node --input-type=module - "$manifest" "$HANK_RELEASE_TAG" "$HANK_EXPECTED_COMMIT" "$HANK_EXPECTED_TREE" <<'NODE'
import fs from 'node:fs';
const [manifestPath, expectedTag, expectedCommit, expectedTree] = process.argv.slice(2);
const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
if (manifest.tag !== expectedTag || manifest.prerelease !== true || manifest.stable !== false) throw new Error('release manifest state mismatch');
if (manifest.sha !== expectedCommit || manifest.tree !== expectedTree) throw new Error('release manifest identity mismatch');
NODE

install_root=$(mktemp -d "${TMPDIR:-/tmp}/hank-release-install.XXXXXX")
mount_point=$(mktemp -d "${TMPDIR:-/tmp}/hank-release-mount.XXXXXX")
hdiutil attach "$dmg" -nobrowse -readonly -mountpoint "$mount_point" >/dev/null
app_source=$(find "$mount_point" -maxdepth 1 -type d -name '*.app' -print -quit)
test -n "$app_source"
cp -R "$app_source" "$install_root/Hank Desktop.app"
hdiutil detach "$mount_point" >/dev/null
mount_point=''
app_binary=$(find "$install_root/Hank Desktop.app/Contents/MacOS" -maxdepth 1 -type f -print -quit)
test -n "$app_binary"
chmod +x "$app_binary"

export HANK_DESKTOP_BIN="$app_binary"
export HANK_E2E_APP_DATA_DIR="${HANK_E2E_APP_DATA_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/hank-release-e2e-data.XXXXXX")}"
export HANK_DESKTOP_E2E_ARTIFACTS="${HANK_DESKTOP_E2E_ARTIFACTS:-$(mktemp -d "${TMPDIR:-/tmp}/hank-release-e2e-artifacts.XXXXXX")}"
mkdir -p "$HANK_E2E_APP_DATA_DIR" "$HANK_DESKTOP_E2E_ARTIFACTS"
profile_marker="$HANK_E2E_APP_DATA_DIR/release-smoke-profile-marker"
export HANK_E2E_ALLOW_RELEASE_DATA_DIR=1
export HANK_E2E_MOCK_PROVIDER=1
export HANK_WEBDRIVER_PORT="${HANK_WEBDRIVER_PORT:-4444}"
export HANK_EXPECTED_COMMIT_SHA="$HANK_EXPECTED_COMMIT"
export HANK_EXPECTED_TREE_SHA="$HANK_EXPECTED_TREE"
export HANK_REQUIRE_ARTIFACT_PROVENANCE=1
export HANK_UPDATER_E2E="${HANK_UPDATER_E2E:-0}"
if [[ "$HANK_UPDATER_E2E" == '1' && -f "$release_root/hank-${HANK_RELEASE_TAG}-macos-updater.json" ]]; then
  export HANK_UPDATER_RELEASE_BUNDLE="$release_root/hank-${HANK_RELEASE_TAG}-macos-updater.json"
  export HANK_UPDATER_RELEASE_ARTIFACT="$dmg"
  export HANK_UPDATER_RELEASE_SIGNING_METADATA="$release_root/release-signing-metadata.json"
elif [[ "$HANK_UPDATER_E2E" == '1' && "${HANK_UPDATER_REQUIRE_RELEASE_BUNDLE:-0}" == '1' ]]; then
  echo 'protected updater bundle is required but missing' >&2
  exit 1
fi

set +e
bash "$PWD/desktop-e2e/run-macos.sh"
exit_code=$?
set -e
if [[ "$exit_code" -ne 0 ]]; then exit "$exit_code"; fi
printf '%s\n' 'preserve-profile' > "$profile_marker"
# A DMG has no uninstaller; clean-room uninstall is the explicit removal of
# the copied app bundle. The target was created by mktemp under a validated
# temporary root, so no user-controlled path is ever removed.
tmp_root=$(cd "${TMPDIR:-/tmp}" && pwd -P)
case "$install_root" in
  "$tmp_root"/hank-release-install.*) rm -r -- "$install_root" ;;
  *) echo "refusing to remove unexpected install root: $install_root" >&2; exit 1 ;;
esac
test ! -e "$install_root"
install_root=''
test -f "$profile_marker"
profile_preserved=PASS
status=PASS
exit_code=0
echo "release install smoke launch: PASS ($HANK_RELEASE_TAG)"
