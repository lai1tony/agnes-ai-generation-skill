param(
  [ValidateSet("start", "stop", "restart", "status", "help")]
  [string]$Action = "start"
)

$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$PidFile = Join-Path $Root ".agnes-dev.pid"
$LogFile = Join-Path $Root ".agnes-dev.log"

function Get-DevProcess {
  if (-not (Test-Path -LiteralPath $PidFile)) {
    return $null
  }
  $raw = (Get-Content -LiteralPath $PidFile -ErrorAction SilentlyContinue | Select-Object -First 1)
  $pidValue = 0
  if (-not [int]::TryParse($raw, [ref]$pidValue)) {
    return $null
  }
  Get-Process -Id $pidValue -ErrorAction SilentlyContinue
}

function Stop-DevServer {
  $process = Get-DevProcess
  if ($process) {
    Write-Host "Stopping process tree $($process.Id)..."
    taskkill /PID $process.Id /T /F | Out-Null
  } else {
    Write-Host "Agnes AI Studio dev server is not running from this script."
  }

  $ports = Get-NetTCPConnection -LocalPort 1420 -State Listen -ErrorAction SilentlyContinue
  foreach ($port in $ports) {
    Stop-Process -Id $port.OwningProcess -Force -ErrorAction SilentlyContinue
  }

  Remove-Item -LiteralPath $PidFile -Force -ErrorAction SilentlyContinue
}

function Start-DevServer {
  Set-Location -LiteralPath $Root

  if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
    throw "npm was not found. Please install Node.js first."
  }
  if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "cargo was not found. Please install Rust first."
  }
  if (-not (Test-Path -LiteralPath (Join-Path $Root "node_modules"))) {
    Write-Host "node_modules not found. Running npm install..."
    npm install
    if ($LASTEXITCODE -ne 0) {
      exit $LASTEXITCODE
    }
  }

  $process = Get-DevProcess
  if ($process) {
    Write-Host "Agnes AI Studio dev server is already running."
    Show-Status
    return
  }

  $command = @"
Set-Location -LiteralPath '$($Root.Path.Replace("'", "''"))'
npm run tauri:dev 2>&1 | Tee-Object -FilePath '$($LogFile.Replace("'", "''"))'
"@
  $encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($command))
  $process = Start-Process `
    -FilePath "powershell.exe" `
    -ArgumentList @("-NoProfile", "-ExecutionPolicy", "Bypass", "-EncodedCommand", $encoded) `
    -WorkingDirectory $Root `
    -WindowStyle Hidden `
    -PassThru

  Set-Content -LiteralPath $PidFile -Value $process.Id -Encoding ascii
  Set-Content -LiteralPath $LogFile -Value "Started powershell.exe PID $($process.Id) at $(Get-Date -Format s)" -Encoding utf8
  Write-Host "Started. Console PID: $($process.Id)"
  Write-Host "Frontend URL: http://localhost:1420"
  Write-Host "The Tauri desktop window should open automatically after compilation."
}

function Show-Status {
  $process = Get-DevProcess
  if (-not $process) {
    Write-Host "Agnes AI Studio dev server is not running from this script."
    exit 1
  }
  Write-Host "Running. Console PID: $($process.Id)"
  Write-Host "Frontend URL: http://localhost:1420"
  Write-Host "The Tauri desktop window should be open if compilation finished."
}

switch ($Action) {
  "start" { Start-DevServer }
  "stop" { Stop-DevServer }
  "restart" {
    Stop-DevServer
    Start-DevServer
  }
  "status" { Show-Status }
  default {
    Write-Host "Usage: .\agnes-dev.bat [start|stop|restart|status]"
  }
}
