#Requires -Version 7.0
<#
.SYNOPSIS
  Tear down the Meridian dev stack started by dev-stack.ps1.

.DESCRIPTION
  Kills the native backend + WebUI process trees (taskkill /T, so cargo->gaussmeridian and cmd->node
  children go too), then stops the data containers. The SurrealDB data volume is KEPT by default so
  your users/projects survive a restart.

.PARAMETER Down
  `docker compose down` (remove the containers) but keep the data volume.
.PARAMETER Purge
  `docker compose down -v` - also drop the SurrealDB data volume for a clean slate.
#>
[CmdletBinding()]
param([switch]$Down, [switch]$Purge)

$ErrorActionPreference = 'Continue'

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$Compose  = Join-Path $RepoRoot 'docker-compose.yml'
$PidDir   = Join-Path $RepoRoot '.runtime\pids'

function Stop-Tree($pidFile, $name) {
  if (Test-Path $pidFile) {
    $procId = Get-Content $pidFile -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($procId) {
      Write-Host ">> stopping $name (PID $procId, tree)"
      taskkill /F /T /PID $procId *> $null
    }
    Remove-Item $pidFile -ErrorAction SilentlyContinue
  } else {
    Write-Host ">> no $name PID recorded (already stopped?)"
  }
}

Stop-Tree (Join-Path $PidDir 'webui.pid')   'WebUI'
Stop-Tree (Join-Path $PidDir 'backend.pid') 'backend'

if (-not (Test-Path $Compose)) {
  Write-Host "XX docker-compose.yml not found at $Compose - skipping container teardown" -ForegroundColor Yellow
} elseif ($Purge) {
  Write-Host '>> docker compose down -v (SurrealDB data volume WIPED)'
  docker compose -f $Compose down -v
} elseif ($Down) {
  Write-Host '>> docker compose down (data volume kept)'
  docker compose -f $Compose down
} else {
  Write-Host '>> docker compose stop surrealdb redis (containers stopped, data kept)'
  docker compose -f $Compose stop surrealdb redis
}

Write-Host 'Meridian dev stack stopped.' -ForegroundColor Green
