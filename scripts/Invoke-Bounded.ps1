param(
    [Parameter(Mandatory=$true)][string]$Executable,
    [string[]]$Arguments = @(),
    [string]$WorkingDirectory = (Get-Location).Path,
    [Parameter(Mandatory=$true)][string]$Name,
    [ValidateRange(1,60)][int]$TimeoutSeconds = 30
)
$ErrorActionPreference = 'Stop'
$reportRoot = Join-Path $PSScriptRoot '..\docs\development\evidence'
New-Item -ItemType Directory -Path $reportRoot -Force | Out-Null
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss-fff'
$prefix = Join-Path $reportRoot ($Name + '-' + $stamp)
$quotedArguments = @($Arguments | ForEach-Object {
    if ($_ -match '"') { throw 'Embedded quotes are not supported by this bounded runner.' }
    if ($_ -match '\s') { '"' + $_ + '"' } else { $_ }
})
$startArgs = @{ FilePath=$Executable; WorkingDirectory=$WorkingDirectory; PassThru=$true; WindowStyle='Hidden'; RedirectStandardOutput=($prefix+'.stdout.log'); RedirectStandardError=($prefix+'.stderr.log') }
if ($quotedArguments.Count) { $startArgs.ArgumentList=$quotedArguments }
$process = Start-Process @startArgs
$retainedHandle = $process.Handle
$completed = $process.WaitForExit($TimeoutSeconds * 1000)
if (-not $completed) {
    # Only the process tree created by this invocation is stopped.
    if ($PSVersionTable.PSVersion.Major -ge 7) { $process.Kill($true) }
    else { & taskkill.exe /PID $process.Id /T /F | Out-Null }
}
$process.WaitForExit()
$record = [ordered]@{ name=$Name; timestamp=(Get-Date).ToString('o'); executable=$Executable; arguments=$Arguments; workingDirectory=$WorkingDirectory; timeoutSeconds=$TimeoutSeconds; timedOut=(-not $completed); exitCode=$process.ExitCode; stdout=($prefix+'.stdout.log'); stderr=($prefix+'.stderr.log') }
$record | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath ($prefix+'.json') -Encoding utf8
Get-Content -LiteralPath ($prefix+'.stdout.log'),($prefix+'.stderr.log')
if (-not $completed) { throw "Timed out. No automatic retry; user must run this step. Evidence: $prefix.json" }
if ($process.ExitCode -ne 0) { throw "Command failed with exit $($process.ExitCode). Evidence: $prefix.json" }
Write-Output "PASS: $Name. Evidence: $prefix.json"
