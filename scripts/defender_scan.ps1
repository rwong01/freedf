# Scan an artifact without executing it. A clean result is evidence, not certification.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Path,
    [Parameter(Mandatory)][string]$ReportDirectory,
    [string]$ExpectedSha256
)
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
New-Item -ItemType Directory -Force -Path $ReportDirectory | Out-Null
$report = [ordered]@{
    startedUtc = [DateTime]::UtcNow.ToString('o')
    path = $Path
    expectedSha256 = $ExpectedSha256
    outcome = 'inconclusive'
    error = $null
}
$failed = $false
try {
    $status = Get-MpComputerStatus
    $report.defender = $status | Select-Object AMEngineVersion, AMProductVersion,
        AntivirusSignatureVersion, AntivirusSignatureLastUpdated, AMServiceEnabled,
        AntivirusEnabled, RealTimeProtectionEnabled, AMRunningMode
    if (-not $status.AMServiceEnabled -or -not $status.AntivirusEnabled -or
        -not $status.RealTimeProtectionEnabled) {
        throw 'Defender is not fully active; this runner cannot produce a passing scan.'
    }
    $report.preferences = Get-MpPreference | Select-Object MAPSReporting,
        SubmitSamplesConsent, DisableBlockAtFirstSeen, ExclusionPath,
        ExclusionProcess, ExclusionExtension
    # Missing/quarantined artifacts must never be interpreted as a clean scan.
    $file = Get-Item -LiteralPath $Path
    if ($file.PSIsContainer) { throw 'Scan one explicit file at a time.' }
    $report.sha256 = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($ExpectedSha256 -and $report.sha256 -ne $ExpectedSha256.ToLowerInvariant()) {
        throw 'Artifact SHA-256 does not match the expected build.'
    }
    $report.authenticode = Get-AuthenticodeSignature -LiteralPath $file.FullName |
        Select-Object Status, StatusMessage
    $candidates = @(Get-ChildItem "$env:ProgramData\Microsoft\Windows Defender\Platform\*\MpCmdRun.exe" -ErrorAction SilentlyContinue |
        Sort-Object { [version]$_.VersionInfo.FileVersion } -Descending)
    $scanner = if ($candidates.Count) { $candidates[0].FullName } else {
        "$env:ProgramFiles\Windows Defender\MpCmdRun.exe"
    }
    if (-not (Test-Path -LiteralPath $scanner)) { throw 'Defender scanner is unavailable.' }
    $report.scanner = $scanner
    # Microsoft's custom-scan diagnostic mode ignores file exclusions and prints
    # detections to stdout. It does NOT disable Defender's real-time protection.
    # No sample is executed, restored from quarantine, or added to an allowlist.
    & $scanner -Scan -ScanType 3 -File $file.FullName -DisableRemediation 2>&1 |
        Tee-Object -FilePath (Join-Path $ReportDirectory 'scanner.txt')
    $report.exitCode = $LASTEXITCODE
    if ($report.exitCode -ne 0) { throw "Defender detected a threat or could not scan (exit $($report.exitCode)); inspect scanner.txt." }
    $report.sha256After = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($report.sha256After -ne $report.sha256) { throw 'Artifact changed during scanning.' }
    $report.outcome = 'no-detection-in-this-scan'
} catch {
    $failed = $true
    $report.error = $_.Exception.Message
    Write-Warning $report.error
} finally {
    try {
        @(Get-MpThreatDetection) | ConvertTo-Json -Depth 8 |
            Set-Content (Join-Path $ReportDirectory 'threat-history.json')
    } catch { $report.historyError = $_.Exception.Message }
    $report.finishedUtc = [DateTime]::UtcNow.ToString('o')
    $report | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $ReportDirectory 'result.json')
}
if ($failed) { throw 'Artifact scan failed or was inconclusive. See the saved report.' }
