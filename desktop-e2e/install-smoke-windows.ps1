param(
  [Parameter(Mandatory = $true)][string]$ReleaseDirectory,
  [Parameter(Mandatory = $true)][string]$ReleaseTag,
  [Parameter(Mandatory = $true)][string]$ExpectedCommit,
  [Parameter(Mandatory = $true)][string]$ExpectedTree,
  [string]$ReportPath = ''
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$releaseRoot = (Resolve-Path -LiteralPath $ReleaseDirectory).Path
$manifestPath = Join-Path $releaseRoot 'release-manifest.json'
$manifestHashPath = Join-Path $releaseRoot 'manifest.sha256'
$checksumsPath = Join-Path $releaseRoot 'SHA256SUMS'
$installerPath = Join-Path $releaseRoot "hank-$ReleaseTag-setup.exe"
$appImagePath = Join-Path $releaseRoot "hank-$ReleaseTag-x86_64.AppImage"
$sbomPath = Join-Path $releaseRoot 'SBOM.spdx.json'
$signingMetadataPath = Join-Path $releaseRoot 'release-signing-metadata.json'
$signingAttestationPaths = @(
  (Join-Path $releaseRoot "hank-$ReleaseTag.tar.gz.attestation.json"),
  (Join-Path $releaseRoot "hank-$ReleaseTag-setup.exe.attestation.json"),
  (Join-Path $releaseRoot "hank-$ReleaseTag-x86_64.AppImage.attestation.json")
)
$reportPath = if ($ReportPath) { [IO.Path]::GetFullPath($ReportPath) } else { Join-Path $releaseRoot 'install-smoke-report.json' }
$report = [ordered]@{
  status = 'failed'
  releaseTag = $ReleaseTag
  expectedCommit = $ExpectedCommit
  expectedTree = $ExpectedTree
  platform = 'windows-x86_64'
  installer = "hank-$ReleaseTag-setup.exe"
  artifactDigests = $null
  installerDigest = $null
  sbom = 'SBOM.spdx.json'
  sbomDigest = $null
  installedExecutable = $null
  uninstall = 'pending'
  profilePreserved = 'pending'
  upgradeRollback = 'NO_PROOF'
  error = $null
}
foreach ($required in @($manifestPath, $manifestHashPath, $checksumsPath, $installerPath, $appImagePath, $sbomPath, $signingMetadataPath) + $signingAttestationPaths) {
  if (-not (Test-Path -LiteralPath $required -PathType Leaf)) { throw "release asset is missing: $required" }
}

function Assert-ChecksumFile {
  param([Parameter(Mandatory = $true)][string]$ChecksumFile)
  $seen = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
  foreach ($line in Get-Content -LiteralPath $ChecksumFile) {
    if ([string]::IsNullOrWhiteSpace($line)) { continue }
    $parts = $line.Trim() -split '\s+', 2
    if ($parts.Count -ne 2 -or $parts[0] -notmatch '^[0-9a-fA-F]{64}$') { throw "invalid checksum entry: $line" }
    $name = $parts[1].TrimStart('*').Trim()
    if (-not $seen.Add($name)) { throw "duplicate checksum entry: $name" }
    $asset = [IO.Path]::GetFullPath((Join-Path $releaseRoot $name))
    $relative = [IO.Path]::GetRelativePath($releaseRoot, $asset)
    if ($relative.StartsWith('..') -or [IO.Path]::IsPathRooted($relative)) { throw "checksum path escapes release directory: $name" }
    if (-not (Test-Path -LiteralPath $asset -PathType Leaf)) { throw "checksum asset is missing: $name" }
    $actual = (Get-FileHash -LiteralPath $asset -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $parts[0].ToLowerInvariant()) { throw "checksum mismatch: $name" }
  }
  if ($seen.Count -eq 0) { throw "checksum file is empty: $ChecksumFile" }
}

Assert-ChecksumFile -ChecksumFile $manifestHashPath
Assert-ChecksumFile -ChecksumFile $checksumsPath

$manifest = Get-Content -Raw -LiteralPath $manifestPath | ConvertFrom-Json
if ($manifest.tag -ne $ReleaseTag -or $manifest.prerelease -ne $true -or $manifest.stable -ne $false) {
  throw "release manifest is not the expected prerelease: $($manifest.tag)"
}
if ($manifest.sha -ne $ExpectedCommit -or $manifest.tree -ne $ExpectedTree) {
  throw "release manifest identity mismatch: expected $ExpectedCommit/$ExpectedTree, got $($manifest.sha)/$($manifest.tree)"
}
$report.artifactDigests = $manifest.artifactDigests
$report.installerDigest = (Get-FileHash -LiteralPath $installerPath -Algorithm SHA256).Hash.ToLowerInvariant()
$report.sbomDigest = (Get-FileHash -LiteralPath $sbomPath -Algorithm SHA256).Hash.ToLowerInvariant()

$nodeBinary = $env:HANK_NODE_BIN
if (-not $nodeBinary) {
  $nodeCommand = Get-Command node.exe -ErrorAction SilentlyContinue
  if ($nodeCommand) { $nodeBinary = $nodeCommand.Source }
}
if (-not $nodeBinary -or -not (Test-Path -LiteralPath $nodeBinary -PathType Leaf)) { throw 'Node.js executable is required for release install smoke' }
& $nodeBinary (Join-Path $repositoryRoot 'tools/release-prerelease.mjs') verify-artifacts --manifest $manifestPath --directory $releaseRoot
if ($LASTEXITCODE -ne 0) { throw "manifest artifact verification failed with exit code $LASTEXITCODE" }
& $nodeBinary (Join-Path $repositoryRoot 'tools/release-artifact-signing.mjs') verify `
  --directory $releaseRoot `
  --attestationDirectory $releaseRoot `
  --artifacts "hank-$ReleaseTag.tar.gz,hank-$ReleaseTag-setup.exe,hank-$ReleaseTag-x86_64.AppImage" `
  --commit $ExpectedCommit `
  --tree $ExpectedTree
if ($LASTEXITCODE -ne 0) { throw "release signing verification failed with exit code $LASTEXITCODE" }
& $nodeBinary (Join-Path $repositoryRoot 'tools/release-sbom.mjs') verify `
  --file $sbomPath `
  --commit $ExpectedCommit `
  --tree $ExpectedTree `
  --version $manifest.version
if ($LASTEXITCODE -ne 0) { throw "SBOM provenance verification failed with exit code $LASTEXITCODE" }

$installRoot = Join-Path $env:RUNNER_TEMP ("hank-release-install-smoke-" + [guid]::NewGuid().ToString('N'))
$profileRoot = Join-Path $env:RUNNER_TEMP ("hank-release-install-profile-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $installRoot, $profileRoot | Out-Null
$profileMarker = Join-Path $profileRoot 'release-smoke-profile-marker'
[IO.File]::WriteAllText($profileMarker, 'preserve-profile')
$installedExecutable = Join-Path $installRoot 'hank-desktop.exe'
$uninstaller = Join-Path $installRoot 'uninstall.exe'
$desktopProcess = $null
$devtoolsPort = 0
$updaterData = $null
$updaterArtifacts = $null
$listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
try {
  $arguments = @('/S', "/D=$installRoot")
  $installerProcess = Start-Process -FilePath $installerPath -ArgumentList $arguments -Wait -PassThru
  if ($installerProcess.ExitCode -ne 0) { throw "NSIS installation failed with exit code $($installerProcess.ExitCode)" }
  if (-not (Test-Path -LiteralPath $installedExecutable -PathType Leaf)) { throw "installed executable not found: $installedExecutable" }
  if (-not (Test-Path -LiteralPath $uninstaller -PathType Leaf)) { throw "uninstaller not found: $uninstaller" }

  $listener.Start()
  $devtoolsPort = ([System.Net.IPEndPoint]$listener.LocalEndpoint).Port
  $listener.Stop()
  $desktopProcess = Start-Process -FilePath $installedExecutable `
    -WorkingDirectory $installRoot `
    -Environment @{
      WEBVIEW2_USER_DATA_FOLDER = $profileRoot
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$devtoolsPort"
    } `
    -PassThru
  & $nodeBinary (Join-Path $repositoryRoot 'tools/windows-desktop-e2e.mjs') --executable $installedExecutable --port $devtoolsPort --pid $desktopProcess.Id
  if ($LASTEXITCODE -ne 0) { throw "installed desktop launch smoke failed with exit code $LASTEXITCODE" }
  if ($env:HANK_UPDATER_E2E -eq '1') {
    $updaterData = Join-Path $env:RUNNER_TEMP ("hank-release-updater-data-" + [guid]::NewGuid().ToString('N'))
    $updaterArtifacts = Join-Path $env:RUNNER_TEMP ("hank-release-updater-artifacts-" + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Force -Path $updaterData, $updaterArtifacts | Out-Null
    $env:HANK_DESKTOP_BIN = $installedExecutable
    $env:HANK_E2E_APP_DATA_DIR = $updaterData
    $env:HANK_DESKTOP_E2E_ARTIFACTS = $updaterArtifacts
    $env:HANK_E2E_ALLOW_RELEASE_DATA_DIR = '1'
    $env:HANK_E2E_MOCK_PROVIDER = '1'
    $env:HANK_EXPECTED_COMMIT_SHA = $ExpectedCommit
    $env:HANK_EXPECTED_TREE_SHA = $ExpectedTree
    $env:HANK_REQUIRE_ARTIFACT_PROVENANCE = '1'
    $windowsUpdaterBundle = Join-Path $releaseRoot "hank-$ReleaseTag-windows-updater.json"
    if (Test-Path -LiteralPath $windowsUpdaterBundle -PathType Leaf) {
      $env:HANK_UPDATER_RELEASE_BUNDLE = $windowsUpdaterBundle
      $env:HANK_UPDATER_RELEASE_ARTIFACT = $installerPath
      $env:HANK_UPDATER_RELEASE_SIGNING_METADATA = $signingMetadataPath
    } elseif ($env:HANK_UPDATER_REQUIRE_RELEASE_BUNDLE -eq '1') {
      throw "protected updater bundle is required but missing: $windowsUpdaterBundle"
    }
    & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $repositoryRoot 'desktop-e2e\run-windows.ps1')
    if ($LASTEXITCODE -ne 0) { throw "installed updater E2E failed with exit code $LASTEXITCODE" }
    $updaterReport = Join-Path $updaterArtifacts 'updater-rollback-report.json'
    if (-not (Test-Path -LiteralPath $updaterReport -PathType Leaf)) { throw "updater rollback report is missing: $updaterReport" }
    $updaterResult = Get-Content -Raw -LiteralPath $updaterReport | ConvertFrom-Json
    if ($updaterResult.status -ne 'PASS') { throw "updater rollback report failed: $($updaterResult.status) / $($updaterResult.evidenceScope)" }
    if ($updaterResult.evidenceScope -eq 'protected-release-signed-artifact') {
      $report.upgradeRollback = 'PASS'
    } elseif ($updaterResult.evidenceScope -eq 'native-synthetic-signed-fixture') {
      $report.upgradeRollback = 'PASS_LIMITED'
    } else {
      throw "updater rollback report has unknown evidence scope: $($updaterResult.evidenceScope)"
    }
  }
  $report.status = 'passed'
  $report.installedExecutable = 'hank-desktop.exe'
  Write-Output "release install smoke launch: PASS ($ReleaseTag)"
} catch {
  $report.error = $_.Exception.Message
  throw
} finally {
  $cleanupError = $null
  try {
    if ($desktopProcess) {
      try {
        $desktopProcess.Refresh()
        if (-not $desktopProcess.HasExited) { $desktopProcess | Stop-Process -Force -ErrorAction SilentlyContinue }
      } catch { }
    }
    if (-not (Test-Path -LiteralPath $uninstaller -PathType Leaf)) { throw "uninstaller not found: $uninstaller" }
    $uninstallProcess = Start-Process -FilePath $uninstaller -ArgumentList @('/S') -Wait -PassThru
    if ($uninstallProcess.ExitCode -ne 0) { throw "NSIS uninstall failed with exit code $($uninstallProcess.ExitCode)" }
    if (Test-Path -LiteralPath $installedExecutable -PathType Leaf) { throw 'installed executable remained after uninstall' }
    $report.uninstall = 'passed'
    if (-not (Test-Path -LiteralPath $profileMarker -PathType Leaf)) { throw 'profile marker was removed during uninstall' }
    if ([IO.File]::ReadAllText($profileMarker) -ne 'preserve-profile') { throw 'profile marker content changed during uninstall' }
    $report.profilePreserved = 'passed'
  } catch {
    $cleanupError = $_.Exception.Message
    $report.status = 'failed'
    $report.error = if ($report.error) { "$($report.error); cleanup: $cleanupError" } else { $cleanupError }
  } finally {
    Remove-Item -LiteralPath $profileRoot -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $installRoot -Recurse -Force -ErrorAction SilentlyContinue
    if ($updaterData) { Remove-Item -LiteralPath $updaterData -Recurse -Force -ErrorAction SilentlyContinue }
    if ($updaterArtifacts) { Remove-Item -LiteralPath $updaterArtifacts -Recurse -Force -ErrorAction SilentlyContinue }
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $reportPath) | Out-Null
    $report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $reportPath -Encoding utf8
  }
  if ($cleanupError) { throw $cleanupError }
}
