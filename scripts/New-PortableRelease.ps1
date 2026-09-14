param(
    [switch]$Online,
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$package = Get-Content -LiteralPath (Join-Path $workspace 'package.json') -Raw | ConvertFrom-Json
$tauri = Get-Content -LiteralPath (Join-Path $workspace 'src-tauri\tauri.conf.json') -Raw | ConvertFrom-Json
$cargoText = Get-Content -LiteralPath (Join-Path $workspace 'src-tauri\Cargo.toml') -Raw
$cargoVersionMatch = [regex]::Match($cargoText, '(?ms)^\[package\].*?^version\s*=\s*"([^"]+)"')
if (-not $cargoVersionMatch.Success) { throw 'Could not read the Cargo package version.' }

$version = [string]$package.version
if ($version -ne [string]$tauri.version -or $version -ne $cargoVersionMatch.Groups[1].Value) {
    throw "Version mismatch: package.json=$($package.version), tauri.conf.json=$($tauri.version), Cargo.toml=$($cargoVersionMatch.Groups[1].Value)"
}

$target = 'x86_64-pc-windows-msvc'
$targetRoot = Join-Path $workspace 'experiments\window-poc\src-tauri\target'
$sourceExecutable = Join-Path $targetRoot 'release\desktop-dashboard.exe'
if (-not $SkipBuild) {
    if ($Online) {
        & (Join-Path $PSScriptRoot 'Build-DesktopDashboard.ps1') -Release -Online
    } else {
        & (Join-Path $PSScriptRoot 'Build-DesktopDashboard.ps1') -Release
    }
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed.' }
}
if (-not (Test-Path -LiteralPath $sourceExecutable -PathType Leaf)) {
    throw "Release executable is missing: $sourceExecutable"
}

$releaseRoot = [IO.Path]::GetFullPath((Join-Path $workspace 'release'))
$folderName = "Codex-Beacon-$version-windows-x64-portable"
$stage = [IO.Path]::GetFullPath((Join-Path $releaseRoot $folderName))
$zipPath = [IO.Path]::GetFullPath((Join-Path $releaseRoot ($folderName + '.zip')))
$checksumPath = $zipPath + '.sha256'
if (-not $stage.StartsWith($releaseRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Refusing to stage outside the release directory.'
}
New-Item -ItemType Directory -Path $releaseRoot -Force | Out-Null
if (Test-Path -LiteralPath $stage) { Remove-Item -LiteralPath $stage -Recurse -Force }
if (Test-Path -LiteralPath $zipPath) { Remove-Item -LiteralPath $zipPath -Force }
if (Test-Path -LiteralPath $checksumPath) { Remove-Item -LiteralPath $checksumPath -Force }
New-Item -ItemType Directory -Path $stage | Out-Null

$portableExecutable = Join-Path $stage 'Codex-Beacon.exe'
Copy-Item -LiteralPath $sourceExecutable -Destination $portableExecutable
$binaryHash = (Get-FileHash -LiteralPath $portableExecutable -Algorithm SHA256).Hash
$builtAt = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')

@"
Codex Beacon $version Windows x64 便携版

运行：双击 Codex-Beacon.exe。程序启动后通过系统托盘管理组件和设置。

开机启动：在“设置 → Behavior”勾选“开机自动运行”。该设置只作用于当前 Windows 用户，不需要管理员权限，也不会写入 JSON 备份。

系统要求：Windows 11 x64，并具备 Microsoft Edge WebView2 Runtime。Windows 11 通常已经包含该运行时。

数据位置：配置、组件内容和自动备份保存在当前 Windows 用户的应用数据目录
%APPDATA%\local.desktopdashboard.widgets

本 ZIP 不包含个人配置、Memo、Todo、Countdown、Codex 数据或备份。删除解压目录不会删除上述用户数据；如需迁移内容，请使用设置中的“导出 JSON”和“导入 JSON”。

移动解压目录：如果已经启用开机启动，移动后请从新位置手动运行一次。旧位置已不存在时程序会自动刷新启动路径；如果旧副本仍然存在，可在 Behavior 页面点击“改为当前程序”。

这是免安装启动包，不会创建开始菜单项或卸载入口。首次从网络下载并运行时，Windows 可能显示 SmartScreen 提示。
"@ | Set-Content -LiteralPath (Join-Path $stage '便携版说明.txt') -Encoding utf8

[ordered]@{
    product = [string]$tauri.productName
    version = $version
    target = $target
    builtAtUtc = $builtAt
    executable = 'Codex-Beacon.exe'
    executableSha256 = $binaryHash
    appIdentifier = [string]$tauri.identifier
} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $stage 'VERSION.json') -Encoding utf8

Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $zipPath -CompressionLevel Optimal
$zipHash = (Get-FileHash -LiteralPath $zipPath -Algorithm SHA256).Hash
"$zipHash  $([IO.Path]::GetFileName($zipPath))" | Set-Content -LiteralPath $checksumPath -Encoding ascii

Write-Host "Portable folder: $stage"
Write-Host "Portable ZIP: $zipPath"
Write-Host "ZIP SHA256: $zipHash"
