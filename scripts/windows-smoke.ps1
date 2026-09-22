$ErrorActionPreference = 'Stop'
if ($env:CI -ne 'true') { throw 'Run this only on an isolated CI runner' }
$resultPath = Join-Path $env:RUNNER_TEMP 'agent-pulse-enter-probe.json'
$probe = Start-Process python -ArgumentList @('scripts/window_probe.py', $resultPath) -WindowStyle Hidden -PassThru
try {
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    do {
        Start-Sleep -Milliseconds 100
        $probe.Refresh()
    } until ($probe.MainWindowHandle -ne 0 -or $probe.HasExited -or [DateTime]::UtcNow -ge $deadline)
    if ($probe.HasExited -or $probe.MainWindowHandle -eq 0) { throw 'Probe window unavailable' }
    $env:AGENT_PULSE_ISOLATED_DESKTOP_TEST = '1'
    cargo run --manifest-path src-tauri/Cargo.toml --example desktop_probe --locked
    if ($LASTEXITCODE -ne 0) { throw 'Native input probe failed' }
    if (!$probe.WaitForExit(15000)) { throw 'Probe timed out' }
    $result = Get-Content -LiteralPath $resultPath | ConvertFrom-Json
    if ($result.submissions.Count -ne 1 -or $result.submissions[0] -ne 'pulse probe 123') {
        throw "Unexpected input: $(Get-Content -LiteralPath $resultPath)"
    }
    Get-Content -LiteralPath $resultPath
} finally {
    if (!$probe.HasExited) { $probe.Kill() }
}
