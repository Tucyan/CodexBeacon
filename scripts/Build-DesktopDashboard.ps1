param([switch]$Online, [switch]$Release)
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Push-Location $workspace
try {
    & npm.cmd run build
    if ($LASTEXITCODE -ne 0) { throw 'Frontend build failed.' }
    # Reuse already compiled dependencies; official source stays in src-tauri.
    $env:CARGO_TARGET_DIR = Join-Path $workspace 'experiments\window-poc\src-tauri\target'
    $arguments = @('build','--manifest-path','src-tauri/Cargo.toml')
    if (-not $Online) { $arguments += '--offline' }
    if ($Release) { $arguments += '--release' }
    & (Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe') @arguments
    if ($LASTEXITCODE -ne 0) { throw 'Rust build failed. Keep the output; do not repeatedly retry installation.' }
    $profile = if ($Release) { 'release' } else { 'debug' }
    Write-Host "Built: $env:CARGO_TARGET_DIR\$profile\desktop-dashboard.exe"
} finally { Pop-Location }
