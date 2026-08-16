#Requires -Version 7.0
<#
.SYNOPSIS
  Bring up the full Meridian dev stack with Docker off-by-default.

.DESCRIPTION
  Turns up ONLY the version-correct data containers (SurrealDB v2.0.0 + Redis) from the existing
  docker-compose.yml, then runs the backend and WebUI natively. Each step is health-gated, and the
  script refuses to clobber a port that is already in use. Non-destructive: it reads .env for the
  SurrealDB password and changes nothing on disk. The PATH `surreal` (3.0.5) is never invoked — the
  container image (v2.0.0) matches the backend crate.

.PARAMETER WebUiPath
  Path to the WebUI app. Defaults to the in-repo 'webui' directory; pass this only if you keep
  the WebUI checked out somewhere else.
.PARAMETER SkipWebUI
  Start only the data tier + backend; run the WebUI yourself.
.PARAMETER NoSeed
  Skip the provider-catalog seed (GAUSSMERIDIAN_DB_SEED).
#>
[CmdletBinding()]
param(
  [string]$WebUiPath = (Join-Path $PSScriptRoot '..\webui'),
  [switch]$SkipWebUI,
  [switch]$NoSeed
)

$ErrorActionPreference = 'Stop'

$RepoRoot   = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$BackendDir = Join-Path $RepoRoot 'gaussmeridian'
$Compose    = Join-Path $RepoRoot 'docker-compose.yml'
$RuntimeDir = Join-Path $RepoRoot '.runtime'
$PidDir     = Join-Path $RuntimeDir 'pids'
$LogDir     = Join-Path $RuntimeDir 'logs'
New-Item -ItemType Directory -Force -Path $PidDir, $LogDir | Out-Null

function Write-Step($msg) { Write-Host "`n>> $msg" -ForegroundColor Cyan }
function Fail($msg)       { Write-Host "XX $msg" -ForegroundColor Red; exit 1 }

function Test-PortBusy([int]$Port) {
  [bool](Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue)
}

# Poll a URL until it answers (any non-5xx counts as "up") or the budget runs out.
function Wait-Http([string]$Url, [int]$TimeoutSec) {
  $deadline = (Get-Date).AddSeconds($TimeoutSec)
  while ((Get-Date) -lt $deadline) {
    try {
      $r = Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 3
      if ($r.StatusCode -ge 200 -and $r.StatusCode -lt 500) { return $true }
    } catch { Start-Sleep -Milliseconds 1500 }
  }
  return $false
}

# ---------------------------------------------------------------- Preflight ---
Write-Step 'Preflight'
try { docker info *> $null } catch { Fail 'Docker is not running. Start Docker Desktop and retry.' }
if (Test-PortBusy 8000) { Fail 'Port 8000 (backend) is already in use. Stop it first (scripts\dev-stack-stop.ps1) - I will not clobber it.' }
if (-not $SkipWebUI -and (Test-PortBusy 3000)) { Fail 'Port 3000 (WebUI) is already in use. Stop it first - I will not clobber it.' }
if (-not (Test-Path $Compose))    { Fail "docker-compose.yml not found at $Compose" }
if (-not (Test-Path $BackendDir)) { Fail "Backend workspace not found at $BackendDir" }

# --------------------------------------------------------------- Data tier ---
Write-Step 'Starting data tier: SurrealDB v2.0.0 + Redis (containers)'
docker compose -f $Compose up -d surrealdb redis
if ($LASTEXITCODE -ne 0) { Fail 'docker compose failed to start surrealdb/redis.' }
Write-Host '   waiting for SurrealDB :8001 ...'
if (-not (Wait-Http 'http://127.0.0.1:8001/health' 45)) {
  Fail 'SurrealDB did not become ready on :8001. Inspect: docker compose logs surrealdb'
}
Write-Host '   SurrealDB ready.' -ForegroundColor Green

# SurrealDB root password: read from .env so the backend signs in with the same value the container
# was started with. Never printed. Falls back to the compose default.
$pw = 'changeme'
$envFile = Join-Path $RepoRoot '.env'
if (Test-Path $envFile) {
  $match = Select-String -Path $envFile -Pattern '^\s*SURREALDB_PASSWORD\s*=' -ErrorAction SilentlyContinue | Select-Object -First 1
  if ($match) { $pw = ($match.Line -replace '^\s*SURREALDB_PASSWORD\s*=\s*', '').Trim().Trim('"').Trim("'") }
}

# ----------------------------------------------------------------- Backend ---
Write-Step 'Starting backend (cargo run --bin gaussmeridian) - first build may take a few minutes'
# Set deterministically in this process env; dotenvy does NOT override already-set vars, so these win.
$env:GAUSSMERIDIAN_DB_URL       = 'ws://127.0.0.1:8001'
$env:GAUSSMERIDIAN_DB_USERNAME  = 'root'
$env:GAUSSMERIDIAN_DB_PASSWORD  = $pw
$env:GAUSSMERIDIAN_DB_NAMESPACE = 'gaussmeridian'
$env:GAUSSMERIDIAN_DB_DATABASE  = 'main'
if (-not $NoSeed) { $env:GAUSSMERIDIAN_DB_SEED = 'true' }

$beOut = Join-Path $LogDir 'backend.out.log'
$beErr = Join-Path $LogDir 'backend.err.log'
$be = Start-Process -FilePath 'cargo' -ArgumentList 'run', '--bin', 'gaussmeridian' `
  -WorkingDirectory $BackendDir -RedirectStandardOutput $beOut -RedirectStandardError $beErr -PassThru
"$($be.Id)" | Out-File -FilePath (Join-Path $PidDir 'backend.pid') -Encoding ascii
Write-Host "   backend PID $($be.Id); logs: $beErr"
Write-Host '   waiting for backend :8000/health (allowing time for first compile) ...'
$deadline = (Get-Date).AddSeconds(600); $beReady = $false
while ((Get-Date) -lt $deadline) {
  if ($be.HasExited) { Fail "Backend process exited (code $($be.ExitCode)). See: $beErr" }
  try {
    $r = Invoke-WebRequest 'http://127.0.0.1:8000/health' -UseBasicParsing -TimeoutSec 3
    if ($r.StatusCode -eq 200) { $beReady = $true; break }
  } catch {}
  Start-Sleep -Seconds 2
}
if (-not $beReady) { Fail "Backend not healthy on :8000 within 10 min. Tail: Get-Content -Wait '$beErr'" }
Write-Host '   backend ready.' -ForegroundColor Green

# ------------------------------------------------------------------- WebUI ---
if (-not $SkipWebUI) {
  if (-not (Test-Path $WebUiPath)) { Fail "WebUI path not found: $WebUiPath (pass -WebUiPath)" }
  $WebUiPath = (Resolve-Path $WebUiPath).Path
  Write-Step "Starting WebUI (pnpm dev) in $WebUiPath"
  $env:GAUSSMERIDIAN_API_URL = 'http://localhost:8000'
  $feOut = Join-Path $LogDir 'webui.out.log'
  $feErr = Join-Path $LogDir 'webui.err.log'
  # pnpm is a .cmd shim on Windows - launch via cmd so redirection is reliable. The stop script kills
  # the whole tree (taskkill /T), so the cmd -> node child is cleaned up.
  $fe = Start-Process -FilePath 'cmd.exe' -ArgumentList '/c', 'pnpm dev' `
    -WorkingDirectory $WebUiPath -RedirectStandardOutput $feOut -RedirectStandardError $feErr -PassThru
  "$($fe.Id)" | Out-File -FilePath (Join-Path $PidDir 'webui.pid') -Encoding ascii
  Write-Host "   WebUI PID $($fe.Id); logs: $feErr"
  Write-Host '   waiting for WebUI :3000 ...'
  if (Wait-Http 'http://localhost:3000' 120) {
    Write-Host '   WebUI ready.' -ForegroundColor Green
  } else {
    Write-Host '   WebUI not confirmed on :3000 yet - check the log; it may still be compiling.' -ForegroundColor Yellow
  }
}

# ------------------------------------------------------------------ Report ---
Write-Step 'Meridian dev stack is UP'
Write-Host '   WebUI     : http://localhost:3000'
Write-Host '   Backend   : http://localhost:8000  (/health)'
Write-Host '   SurrealDB : ws://127.0.0.1:8001    (container, surrealkv)'
Write-Host ''
Write-Host "   Logs : $LogDir"
Write-Host '   Stop : scripts\dev-stack-stop.ps1'
