import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = new URL('../', import.meta.url);
const prerelease = readFileSync(fileURLToPath(new URL('.github/workflows/release-prerelease.yml', root)), 'utf8');
const milestone = readFileSync(fileURLToPath(new URL('.github/workflows/release-milestone.yml', root)), 'utf8');
const macosSmoke = readFileSync(fileURLToPath(new URL('desktop-e2e/install-smoke-macos.sh', root)), 'utf8');
const linuxSmoke = readFileSync(fileURLToPath(new URL('desktop-e2e/install-smoke-linux.sh', root)), 'utf8');
const windowsSmoke = readFileSync(fileURLToPath(new URL('desktop-e2e/install-smoke-windows.ps1', root)), 'utf8');

test('prerelease builds, signs, publishes, and smoke-tests the macOS release lane', () => {
  assert.match(prerelease, /macos-package:/);
  assert.match(prerelease, /runs-on: macos-14/);
  assert.match(prerelease, /--bundles dmg/);
  assert.match(prerelease, /hank-\$\{TAG\}-aarch64\.dmg/);
  assert.match(prerelease, /macos-install-smoke:/);
  assert.match(prerelease, /install-smoke-macos\.sh/);
  assert.match(prerelease, /release-install-smoke-macos-/);
  assert.match(prerelease, /hank-\$\{TAG\}-aarch64\.dmg/);
});

test('stable promotion downloads, verifies, and publishes the macOS DMG', () => {
  assert.match(milestone, /hank-\$\{PRERELEASE_TAG\}-aarch64\.dmg/);
  assert.match(milestone, /hank-\$\{stable_tag\}-aarch64\.dmg/);
  assert.match(milestone, /release-install-smoke-macos-\$\{PRERELEASE_TAG\}/);
  assert.match(milestone, /validateInstallSmokeReports\(reports, \{ commit, tree, releaseTag: process\.env\.PRERELEASE_TAG, requireUpgradeRollback: true \}\)/);
});

test('macOS smoke seeds profile before E2E and verifies it survives app removal', () => {
  const seed = macosSmoke.indexOf("printf '%s\\n' 'preserve-profile' > \"$profile_marker\"");
  const e2e = macosSmoke.indexOf('bash "$PWD/desktop-e2e/run-macos.sh"');
  const verify = macosSmoke.indexOf('test "$(<"$profile_marker")" = \'preserve-profile\'');
  assert.ok(seed >= 0, 'profile marker must be seeded');
  assert.ok(e2e > seed, 'profile marker must exist before the lifecycle E2E');
  assert.ok(verify > e2e, 'profile marker must be checked after app removal');
  assert.equal((macosSmoke.match(/preserve-profile/g) ?? []).length, 2, 'marker should be seeded once and verified once');
});

test('all native install smokes prove profile preservation before reporting PASS', () => {
  const macSeed = macosSmoke.indexOf("printf '%s\\n' 'preserve-profile' > \"$profile_marker\"");
  const macE2e = macosSmoke.indexOf('bash "$PWD/desktop-e2e/run-macos.sh"');
  assert.ok(macSeed >= 0 && macE2e > macSeed);

  const linuxSeed = linuxSmoke.indexOf("printf '%s\\n' 'preserve-profile' > \"$profile_marker\"");
  const linuxE2e = linuxSmoke.indexOf('xvfb-run -a');
  const linuxVerify = linuxSmoke.indexOf('profile_preserved=PASS');
  assert.ok(linuxSeed >= 0 && linuxE2e > linuxSeed && linuxVerify > linuxE2e);
  assert.match(linuxSmoke, /profilePreserved/);

  const windowsSeed = windowsSmoke.indexOf('release-smoke-profile-marker');
  const windowsLaunch = windowsSmoke.indexOf("Start-Process -FilePath $installedExecutable");
  const windowsVerify = windowsSmoke.indexOf("$report.profilePreserved = 'passed'");
  assert.ok(windowsSeed >= 0 && windowsLaunch > windowsSeed && windowsVerify > windowsLaunch);
  assert.match(windowsSmoke, /profilePreserved/);
});
