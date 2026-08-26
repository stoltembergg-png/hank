$ErrorActionPreference = 'Stop'
$driver = (Get-Command msedgedriver -ErrorAction SilentlyContinue).Source
if (-not $driver) {
  $edge = @(
    "$env:ProgramFiles(x86)\Microsoft\Edge\Application\msedge.exe",
    "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe"
  ) | Where-Object { Test-Path $_ } | Select-Object -First 1
  if (-not $edge) { throw 'Microsoft Edge executable not found; cannot select a compatible WebView2 driver' }
  $major = (Get-Item $edge).VersionInfo.ProductVersion.Split('.')[0]
  $version = (Invoke-RestMethod "https://msedgedriver.microsoft.com/LATEST_RELEASE_$major").Trim()
  $zip = Join-Path $env:RUNNER_TEMP "edgedriver-$version.zip"
  $dir = Join-Path $env:RUNNER_TEMP "edgedriver-$version"
  Invoke-WebRequest "https://msedgedriver.microsoft.com/$version/edgedriver_win64.zip" -OutFile $zip
  Expand-Archive -Path $zip -DestinationPath $dir -Force
  $driver = Join-Path $dir 'msedgedriver.exe'
}
if (-not (Test-Path -LiteralPath $driver -PathType Leaf)) { throw "msedgedriver not found: $driver" }

$driverLog = Join-Path $env:HANK_DESKTOP_E2E_ARTIFACTS 'tauri-driver.stdout.log'
$driverErrorLog = Join-Path $env:HANK_DESKTOP_E2E_ARTIFACTS 'tauri-driver.stderr.log'
$driverProcess = Start-Process -FilePath 'tauri-driver' -ArgumentList @('--port', '4444', '--native-driver', $driver) -RedirectStandardOutput $driverLog -RedirectStandardError $driverErrorLog -PassThru
try {
  $ready = $false
  for ($attempt = 0; $attempt -lt 60; $attempt++) {
    try {
      $response = Invoke-WebRequest 'http://127.0.0.1:4444/status' -UseBasicParsing -TimeoutSec 2
      if ($response.StatusCode -eq 200) { $ready = $true; break }
    } catch { }
    if ($driverProcess.HasExited) { throw "tauri-driver exited with code $($driverProcess.ExitCode)" }
    Start-Sleep -Seconds 1
  }
  if (-not $ready) { throw 'tauri-driver did not become ready' }
  npm --prefix desktop-e2e test
} finally {
  if (-not $driverProcess.HasExited) { Stop-Process -Id $driverProcess.Id -Force -ErrorAction SilentlyContinue }
}
