param([switch]$Release)
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$profile = if ($Release) { 'release' } else { 'debug' }
$executable = Join-Path $workspace "experiments\window-poc\src-tauri\target\$profile\desktop-dashboard.exe"
if (-not (Test-Path -LiteralPath $executable)) { throw 'Build the official app first with scripts\Build-DesktopDashboard.ps1.' }
$existing = Get-Process -Name 'desktop-dashboard' -ErrorAction SilentlyContinue
if ($existing) { Write-Host 'Desktop Dashboard is already running. Open Settings from its tray icon.'; return }
$process = Start-Process -FilePath $executable -WorkingDirectory $workspace -WindowStyle Hidden -PassThru
Write-Host "Started official Desktop Dashboard. PID: $($process.Id). Use its tray icon for Settings/Show/Hide/Quit."
