const DEFAULT_PLATFORMS = ['linux-x86_64', 'windows-x86_64'];

function isPass(value) {
  return value === 'PASS' || value === 'passed';
}

function requireText(value, field, platform) {
  if (typeof value !== 'string' || value.length === 0 || value.length > 256) {
    throw new Error(`${field} is missing: ${platform}`);
  }
}

function requireDigest(value, field, platform) {
  if (typeof value !== 'string' || !/^(?:sha256:)?[0-9a-f]{64}$/i.test(value)) {
    throw new Error(`${field} is missing: ${platform}`);
  }
}

/**
 * Validates clean-room release install reports before stable promotion.
 * Reports are untrusted workflow artifacts and must be bound to the exact
 * source identity and include a completed upgrade/rollback exercise.
 */
export function validateInstallSmokeReports(
  reports,
  { commit, tree, releaseTag, platforms = DEFAULT_PLATFORMS, requireUpgradeRollback = true } = {},
) {
  requireText(commit, 'expected commit', 'release');
  requireText(tree, 'expected tree', 'release');
  if (releaseTag !== undefined) requireText(releaseTag, 'expected release tag', 'release');
  if (!Array.isArray(platforms) || platforms.length === 0 || new Set(platforms).size !== platforms.length) {
    throw new Error('release install evidence platform policy is invalid');
  }
  if (!Array.isArray(reports)) throw new Error('release install evidence reports are missing');

  const byPlatform = new Map();
  for (const candidate of reports) {
    if (!candidate || typeof candidate !== 'object' || Array.isArray(candidate)) {
      throw new Error('release install evidence report is malformed');
    }
    const platform = candidate.platform;
    if (typeof platform !== 'string' || byPlatform.has(platform)) {
      throw new Error(`duplicate or malformed install smoke evidence: ${String(platform)}`);
    }
    byPlatform.set(platform, candidate);
  }

  for (const platform of platforms) {
    const report = byPlatform.get(platform);
    if (!report) throw new Error(`missing install smoke evidence: ${platform}`);
    if (report.expectedCommit !== commit || report.expectedTree !== tree) {
      throw new Error(`identity mismatch: ${platform}`);
    }
    if (releaseTag !== undefined && report.releaseTag !== releaseTag) {
      throw new Error(`release tag mismatch: ${platform}`);
    }
    if (!isPass(report.status)) throw new Error(`install smoke status is not PASS: ${platform}`);
    if (requireUpgradeRollback && !isPass(report.upgradeRollback)) {
      throw new Error(`upgrade/rollback evidence is not PASS: ${platform}`);
    }
    if (platform === 'windows-x86_64') {
      requireDigest(report.installerDigest, 'installer digest', platform);
      if (!isPass(report.uninstall)) throw new Error(`uninstall evidence is not PASS: ${platform}`);
    } else if (platform === 'linux-x86_64') {
      requireDigest(report.appImageDigest, 'AppImage digest', platform);
    }
  }

  return { status: 'PASS', platforms: [...platforms].sort() };
}

export { DEFAULT_PLATFORMS };
