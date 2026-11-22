$ErrorActionPreference = 'Stop'
$env:TERM_PROGRAM = 'rmux'
$env:RMUX_SHELL_INTEGRATION = '1'
$script:promptCalls = 0
function global:prompt { $script:promptSawSuccess = $?; $script:promptCalls++; return 'fixture> ' }
$script:readCalls = 0
$script:nextLine = 'echo fixture'
function global:PSConsoleHostReadLine { $script:readCalls++; return $script:nextLine }
$integration = Join-Path $PSScriptRoot '../shell-integration/powershell/rmux.ps1'
$root = Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid().ToString())
$directory = Join-Path $root 'project 世界 % space'
$original = Get-Location
try {
    New-Item -ItemType Directory -Path $directory | Out-Null
    Set-Location -LiteralPath $directory
    $env:RMUX_SHELL_INTEGRATION = '0'
    . $integration
    if (Test-Path variable:global:_rmux_original_prompt) { throw 'Opt-out was ignored' }
    $env:RMUX_SHELL_INTEGRATION = '1'
    . $integration
    . $integration
    $global:LASTEXITCODE = 17
    Get-Item -LiteralPath (Join-Path $root 'missing') -ErrorAction SilentlyContinue
    $result = prompt
    $escape = [char]27
    $bell = [char]7
    $expectedUri = [Uri]::new((Get-Location).ProviderPath).AbsoluteUri
    $expected = "$escape]7;$expectedUri$bell$escape]133;A${bell}fixture> $escape]133;B$bell"
    if ($result -cne $expected) { throw 'Directory or prompt boundary output differs' }
    if ($script:promptSawSuccess) { throw 'Original prompt lost the failed-command status' }
    if ($script:promptCalls -ne 1) { throw 'Repeated sourcing duplicated prompt execution' }
    if ($global:LASTEXITCODE -ne 17) { throw 'Native command exit code was changed' }
    if (!$expectedUri.Contains('%25') -or !$expectedUri.Contains('%20')) { throw 'Directory URI was not escaped' }
    Set-Location Env:
    $result = prompt
    if ($result.Contains("]7;")) { throw 'Non-filesystem provider emitted a file URI' }
    $consoleOut = [Console]::Out
    $capture = [IO.StringWriter]::new()
    try {
        [Console]::SetOut($capture)
        $line = PSConsoleHostReadLine
        if ($line -cne $script:nextLine -or $script:readCalls -ne 1) { throw 'Original reader was not preserved' }
        if ($capture.ToString() -cne "$escape]133;C$bell") { throw 'Command start was not emitted exactly once' }
        Get-Item -LiteralPath (Join-Path $root 'missing') -ErrorAction SilentlyContinue
        $result = prompt
        if ($script:promptSawSuccess) { throw 'Completion marker changed prompt status' }
        if (!$result.StartsWith("$escape]133;D;1$bell")) { throw 'Command completion missing' }
        if ((prompt).Contains("]133;D")) { throw 'Completion was emitted twice' }
        $null = PSConsoleHostReadLine
        $global:LASTEXITCODE = 23
        $result = prompt
        if (!$script:promptSawSuccess -or !$result.StartsWith("$escape]133;D;0$bell")) { throw 'Successful command inherited a stale native failure' }
        if ($global:LASTEXITCODE -ne 23) { throw 'Completion changed native exit status' }
        $capture.GetStringBuilder().Clear() | Out-Null
        $script:nextLine = '   '
        $null = PSConsoleHostReadLine
        if ($capture.ToString() -ne '' -or (prompt).Contains("]133;D")) { throw 'Empty input started a command' }
        $global:_rmux_original_readline = { throw 'fixture canceled read' }
        try { $null = PSConsoleHostReadLine } catch { }
        if ($capture.ToString() -ne '' -or (prompt).Contains("]133;D")) { throw 'Canceled read started a command' }
    } finally {
        [Console]::SetOut($consoleOut)
        $capture.Dispose()
    }
    'PowerShell prompt and command-boundary integration checks passed'
} finally {
    Set-Location -LiteralPath $original.ProviderPath
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
}
