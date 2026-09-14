param([switch]$Release)
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$profile = if ($Release) { 'release' } else { 'debug' }
$executable = Join-Path $workspace "experiments\window-poc\src-tauri\target\$profile\desktop-dashboard.exe"
if (-not (Test-Path -LiteralPath $executable)) { throw "Build the official $profile app first." }
$dataDirectory = Join-Path $workspace ('docs\development\evidence\smoke-' + (Get-Date -Format 'yyyyMMdd-HHmmss-fff'))
New-Item -ItemType Directory -Path $dataDirectory -Force | Out-Null
$previousData = $env:DASHBOARD_DATA_DIR
$previousSmoke = $env:DASHBOARD_SMOKE_TEST
try {
    $env:DASHBOARD_DATA_DIR = $dataDirectory
    $env:DASHBOARD_SMOKE_TEST = '1'
    & (Join-Path $PSScriptRoot 'Invoke-Bounded.ps1') -Executable $executable -WorkingDirectory $workspace -Name "official-native-smoke-$profile" -TimeoutSeconds 25
    $log = Get-Content -LiteralPath (Join-Path $dataDirectory 'runtime.log') -Raw
    if ($log -notmatch 'smoke_two_independent_renderers_passed' -or $log -notmatch 'application_exited_cleanly') { throw "Smoke assertions failed. Inspect $dataDirectory" }
    if ($log -notmatch 'smoke_event_isolation_passed' -or $log -match 'smoke_renderer_identity_mismatch|smoke_event_isolation_failed') { throw "Window event isolation failed. Inspect $dataDirectory" }
    if (-not (Test-Path -LiteralPath (Join-Path $dataDirectory 'dashboard.db'))) { throw 'SQLite database was not created.' }
    [ordered]@{ passed=$true; testedAt=(Get-Date).ToString('o'); binary=$executable; sha256=(Get-FileHash -LiteralPath $executable -Algorithm SHA256).Hash; dataDirectory=$dataDirectory; limitations='No visual approval; providers disabled; checks include repeated native move/resize, scoped snapshot and rendered widget identity, final revision, visibility and clean flush exit.' } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $dataDirectory 'result.json') -Encoding utf8
    Write-Host "Native smoke passed. Evidence: $dataDirectory"
} finally {
    $env:DASHBOARD_DATA_DIR = $previousData
    $env:DASHBOARD_SMOKE_TEST = $previousSmoke
}
