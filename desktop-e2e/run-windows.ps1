$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$nodeBinary = $env:HANK_NODE_BIN
if (-not $nodeBinary) {
  $nodeCommand = Get-Command node.exe -ErrorAction SilentlyContinue
  if ($nodeCommand) { $nodeBinary = $nodeCommand.Source }
}
if (-not $nodeBinary -or -not (Test-Path -LiteralPath $nodeBinary -PathType Leaf)) { throw 'Node.js executable is required for the desktop E2E runner' }
$desktopBinary = $env:HANK_DESKTOP_BIN
if (-not $desktopBinary) { throw 'HANK_DESKTOP_BIN is required' }
if (-not (Test-Path -LiteralPath $desktopBinary -PathType Leaf)) { throw "desktop executable not found: $desktopBinary" }
$desktopBinary = (Resolve-Path -LiteralPath $desktopBinary).Path
$dataDir = $env:HANK_E2E_APP_DATA_DIR
if (-not $dataDir) { throw 'HANK_E2E_APP_DATA_DIR is required' }
$artifacts = $env:HANK_DESKTOP_E2E_ARTIFACTS
if (-not $artifacts) { throw 'HANK_DESKTOP_E2E_ARTIFACTS is required' }
New-Item -ItemType Directory -Force -Path $artifacts, $dataDir | Out-Null

function Stop-ExactDesktopProcess {
  Get-CimInstance Win32_Process -ErrorAction SilentlyContinue |
    Where-Object { $_.ExecutablePath -eq $desktopBinary } |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
}

function Get-FreeTcpPort {
  $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
  try {
    $listener.Start()
    return ([System.Net.IPEndPoint]$listener.LocalEndpoint).Port
  } finally {
    $listener.Stop()
  }
}

$webdriverPort = 4444
if ($env:HANK_WEBDRIVER_PORT) {
  $parsedPort = 0
  if (-not [int]::TryParse($env:HANK_WEBDRIVER_PORT, [ref]$parsedPort) -or $parsedPort -lt 1 -or $parsedPort -gt 65535) {
    throw "HANK_WEBDRIVER_PORT must be a valid TCP port: $($env:HANK_WEBDRIVER_PORT)"
  }
  $webdriverPort = $parsedPort
}
$nativeWebdriverPort = Get-FreeTcpPort
if ($nativeWebdriverPort -eq $webdriverPort) { $nativeWebdriverPort = Get-FreeTcpPort }

function Get-WebView2Executable {
  $explicit = $env:HANK_WEBVIEW2_BIN
  if ($explicit) {
    if (-not (Test-Path -LiteralPath $explicit -PathType Leaf)) { throw "WebView2 executable not found: $explicit" }
    return (Resolve-Path -LiteralPath $explicit).Path
  }

  $roots = @(
    "${env:ProgramFiles(x86)}\Microsoft\EdgeWebView\Application",
    "$env:ProgramFiles\Microsoft\EdgeWebView\Application",
    "$env:LOCALAPPDATA\Microsoft\EdgeWebView\Application"
  ) | Where-Object { Test-Path -LiteralPath $_ -PathType Container }
  $versions = foreach ($root in $roots) {
    Get-ChildItem -LiteralPath $root -Directory -ErrorAction SilentlyContinue |
      ForEach-Object {
        $executable = Join-Path $_.FullName 'msedgewebview2.exe'
        if (Test-Path -LiteralPath $executable -PathType Leaf) { Get-Item -LiteralPath $executable }
      }
  }
  $selected = $versions | Sort-Object { try { [version]$_.VersionInfo.ProductVersion } catch { [version]'0.0.0.0' } } -Descending | Select-Object -First 1
  if (-not $selected) { throw 'WebView2 Runtime executable not found; cannot select a compatible WebDriver' }
  return $selected.FullName
}

function Get-MatchingWebView2Driver {
  param([Parameter(Mandatory = $true)][string]$WebView2Executable)

  $runtimeVersion = (Get-Item -LiteralPath $WebView2Executable).VersionInfo.ProductVersion
  $runtimeParts = $runtimeVersion.Split('.')
  if ($runtimeParts.Count -lt 3) { throw "Could not read the first three WebView2 Runtime version components: $runtimeVersion" }
  $runtimeBuild = $runtimeParts[0..2] -join '.'

  $candidate = $env:HANK_MSEDGEDRIVER
  if (-not $candidate) {
    $candidateCommand = Get-Command msedgedriver.exe -ErrorAction SilentlyContinue
    if ($candidateCommand) { $candidate = $candidateCommand.Source }
  }
  if ($candidate -and (Test-Path -LiteralPath $candidate -PathType Leaf)) {
    $msedgedriverVersion = (& $candidate --version 2>$null | Out-String).Trim()
    $driverVersionMatch = [regex]::Match($msedgedriverVersion, '\d+\.\d+\.\d+\.\d+')
    if ($driverVersionMatch.Success) {
      $driverParts = $driverVersionMatch.Value.Split('.')
      if (($driverParts[0..2] -join '.') -eq $runtimeBuild) {
        return (Resolve-Path -LiteralPath $candidate).Path
      }
      Write-Warning "Ignoring EdgeDriver $($driverVersionMatch.Value); WebView2 Runtime $runtimeVersion requires matching first three version components"
    }
  }

  $runnerTemp = $env:RUNNER_TEMP
  if (-not $runnerTemp) { $runnerTemp = [IO.Path]::GetTempPath().TrimEnd('\') }
  New-Item -ItemType Directory -Force -Path $runnerTemp | Out-Null
  $versionsToTry = @()
  $lookupErrors = @()
  foreach ($lookup in @("LATEST_RELEASE_$runtimeBuild")) {
    try {
      $response = Invoke-WebRequest "https://msedgedriver.microsoft.com/$lookup" -UseBasicParsing -TimeoutSec 30
      if ($response.Content -is [byte[]]) {
        $bytes = [byte[]]$response.Content
        $encoding = if ($bytes.Length -ge 2 -and $bytes[0] -eq 0xFF -and $bytes[1] -eq 0xFE) {
          [Text.Encoding]::Unicode
        } else {
          [Text.Encoding]::UTF8
        }
        $content = $encoding.GetString($bytes)
      } else {
        $content = [string]$response.Content
      }
      $candidateVersion = [regex]::Match($content.Trim(), '\d+\.\d+\.\d+\.\d+')
      if ($candidateVersion.Success) {
        $versionsToTry += $candidateVersion.Value
      }
      $lookupErrors += "$lookup returned an invalid version payload"
    } catch {
      $lookupErrors += "${lookup}: $($_.Exception.Message)"
    }
  }
  # The runtime's exact version is a safe fallback when Microsoft's build lookup
  # is unavailable. The major-only endpoint can return an archived version that
  # no longer has a downloadable ZIP, so try it only after this exact fallback.
  $versionsToTry += $runtimeVersion
  try {
    $response = Invoke-WebRequest "https://msedgedriver.microsoft.com/LATEST_RELEASE_$($runtimeParts[0])" -UseBasicParsing -TimeoutSec 30
    if ($response.Content -is [byte[]]) {
      $bytes = [byte[]]$response.Content
      $encoding = if ($bytes.Length -ge 2 -and $bytes[0] -eq 0xFF -and $bytes[1] -eq 0xFE) {
        [Text.Encoding]::Unicode
      } else {
        [Text.Encoding]::UTF8
      }
      $content = $encoding.GetString($bytes)
    } else {
      $content = [string]$response.Content
    }
    $candidateVersion = [regex]::Match($content.Trim(), '\d+\.\d+\.\d+\.\d+')
    if ($candidateVersion.Success) {
      $versionsToTry += $candidateVersion.Value
    } else {
      $lookupErrors += "LATEST_RELEASE_$($runtimeParts[0]) returned an invalid version payload"
    }
  } catch {
    $lookupErrors += "LATEST_RELEASE_$($runtimeParts[0]): $($_.Exception.Message)"
  }

  $downloadErrors = @()
  foreach ($version in ($versionsToTry | Select-Object -Unique)) {
    $zip = Join-Path $runnerTemp "edgedriver-$version.zip"
    $dir = Join-Path $runnerTemp "edgedriver-$version"
    try {
      Invoke-WebRequest "https://msedgedriver.microsoft.com/$version/edgedriver_win64.zip" -OutFile $zip -UseBasicParsing -TimeoutSec 60
      Expand-Archive -Path $zip -DestinationPath $dir -Force
      $downloadedDriver = Join-Path $dir 'msedgedriver.exe'
      if (-not (Test-Path -LiteralPath $downloadedDriver -PathType Leaf)) { throw "Downloaded EdgeDriver is missing: $downloadedDriver" }
      $downloadedVersionText = (& $downloadedDriver --version 2>$null | Out-String).Trim()
      $downloadedVersion = [regex]::Match($downloadedVersionText, '\d+\.\d+\.\d+\.\d+')
      if (-not $downloadedVersion.Success -or (($downloadedVersion.Value.Split('.')[0..2] -join '.') -ne $runtimeBuild)) {
        throw "Downloaded EdgeDriver $downloadedVersionText does not match WebView2 Runtime build $runtimeBuild"
      }
      $script:downloadedDriverDirectory = $dir
      $script:downloadedDriverZip = $zip
      return $downloadedDriver
    } catch {
      $downloadErrors += "$version`: $($_.Exception.Message)"
      Remove-Item -LiteralPath $dir -Recurse -Force -ErrorAction SilentlyContinue
      Remove-Item -LiteralPath $zip -Force -ErrorAction SilentlyContinue
    }
  }
  throw "Could not resolve a compatible Microsoft EdgeDriver for WebView2 Runtime $runtimeVersion. Lookups: $($lookupErrors -join '; '). Downloads: $($downloadErrors -join '; ')"
}

$webView2 = $null
$driver = $null
try {
  $webView2 = Get-WebView2Executable
  $driver = Get-MatchingWebView2Driver -WebView2Executable $webView2
  if (-not (Test-Path -LiteralPath $driver -PathType Leaf)) { throw "msedgedriver not found: $driver" }
  Write-Output "Using WebView2 Runtime $((Get-Item -LiteralPath $webView2).VersionInfo.ProductVersion) with EdgeDriver $driver"
} catch {
  if ($script:downloadedDriverDirectory -and (Test-Path -LiteralPath $script:downloadedDriverDirectory)) {
    Remove-Item -LiteralPath $script:downloadedDriverDirectory -Recurse -Force -ErrorAction SilentlyContinue
  }
  if ($script:downloadedDriverZip -and (Test-Path -LiteralPath $script:downloadedDriverZip)) {
    Remove-Item -LiteralPath $script:downloadedDriverZip -Force -ErrorAction SilentlyContinue
  }
  throw
}

$frontendLog = Join-Path $env:HANK_DESKTOP_E2E_ARTIFACTS 'frontend-preview.stdout.log'
$frontendErrorLog = Join-Path $env:HANK_DESKTOP_E2E_ARTIFACTS 'frontend-preview.stderr.log'
$frontendProcess = $null
$driverProcess = $null
try {
  $frontendProcess = Start-Process -FilePath $nodeBinary -ArgumentList @('node_modules/vite/bin/vite.js', 'preview', '--host', '127.0.0.1', '--port', '1420') -WorkingDirectory (Join-Path $repositoryRoot 'frontend') -RedirectStandardOutput $frontendLog -RedirectStandardError $frontendErrorLog -PassThru
  $frontendReady = $false
  for ($attempt = 0; $attempt -lt 60; $attempt++) {
    if ($frontendProcess.HasExited) { throw "frontend preview exited with code $($frontendProcess.ExitCode)" }
    try {
      $response = Invoke-WebRequest 'http://127.0.0.1:1420/' -UseBasicParsing -TimeoutSec 2
      if ($response.StatusCode -eq 200) {
        if ($frontendProcess.HasExited) { throw "frontend preview exited with code $($frontendProcess.ExitCode)" }
        $frontendReady = $true
        break
      }
    } catch { }
    if ($frontendProcess.HasExited) { throw "frontend preview exited with code $($frontendProcess.ExitCode)" }
    Start-Sleep -Seconds 1
  }
  if (-not $frontendReady) { throw 'frontend preview did not become ready' }

  $driverLog = Join-Path $env:HANK_DESKTOP_E2E_ARTIFACTS 'tauri-driver.stdout.log'
  $driverErrorLog = Join-Path $env:HANK_DESKTOP_E2E_ARTIFACTS 'tauri-driver.stderr.log'
  $tauriDriverBinary = $env:HANK_TAURI_DRIVER_BIN
  if (-not $tauriDriverBinary) {
    $tauriDriverCommand = Get-Command tauri-driver.exe -ErrorAction SilentlyContinue
    if ($tauriDriverCommand) { $tauriDriverBinary = $tauriDriverCommand.Source }
  }
  if (-not $tauriDriverBinary) {
    $cargoDriver = Join-Path $env:USERPROFILE '.cargo\bin\tauri-driver.exe'
    if (Test-Path -LiteralPath $cargoDriver -PathType Leaf) { $tauriDriverBinary = $cargoDriver }
  }
  if (-not $tauriDriverBinary -or -not (Test-Path -LiteralPath $tauriDriverBinary -PathType Leaf)) { throw 'tauri-driver executable is required for the desktop E2E runner' }
  $driverProcess = Start-Process -FilePath $tauriDriverBinary -ArgumentList @('--port', "$webdriverPort", '--native-port', "$nativeWebdriverPort", '--native-driver', $driver) -RedirectStandardOutput $driverLog -RedirectStandardError $driverErrorLog -PassThru
  try {
    $ready = $false
    for ($attempt = 0; $attempt -lt 60; $attempt++) {
      try {
        $response = Invoke-WebRequest "http://127.0.0.1:$webdriverPort/status" -UseBasicParsing -TimeoutSec 2
        if ($response.StatusCode -eq 200) { $ready = $true; break }
      } catch { }
      if ($driverProcess.HasExited) { throw "tauri-driver exited with code $($driverProcess.ExitCode)" }
      Start-Sleep -Seconds 1
    }
    if (-not $ready) { throw 'tauri-driver did not become ready' }
    & $nodeBinary (Join-Path $repositoryRoot 'desktop-e2e/specs/project-lifecycle.e2e.mjs')
    if ($LASTEXITCODE -ne 0) { throw "desktop lifecycle E2E failed with exit code $LASTEXITCODE" }
  } finally {
    if ($driverProcess -and -not $driverProcess.HasExited) { Stop-Process -Id $driverProcess.Id -Force -ErrorAction SilentlyContinue }
    Stop-ExactDesktopProcess
  }
} finally {
  if ($frontendProcess -and -not $frontendProcess.HasExited) { Stop-Process -Id $frontendProcess.Id -Force -ErrorAction SilentlyContinue }
  Stop-ExactDesktopProcess
  if ($script:downloadedDriverDirectory -and (Test-Path -LiteralPath $script:downloadedDriverDirectory)) {
    Remove-Item -LiteralPath $script:downloadedDriverDirectory -Recurse -Force -ErrorAction SilentlyContinue
  }
  if ($script:downloadedDriverZip -and (Test-Path -LiteralPath $script:downloadedDriverZip)) {
    Remove-Item -LiteralPath $script:downloadedDriverZip -Force -ErrorAction SilentlyContinue
  }
}
