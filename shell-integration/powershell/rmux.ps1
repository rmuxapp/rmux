# PowerShell integration, loaded after profiles; also supports manual dot-sourcing.
# No profile edits, external commands, key-binding or execution-policy changes.
if ($env:TERM_PROGRAM -ne 'rmux' -or $env:RMUX_SHELL_INTEGRATION -eq '0') { return }
if (Test-Path variable:global:_rmux_original_prompt) { return }

$global:_rmux_command_running = $false
$global:_rmux_original_prompt = $function:prompt
function global:prompt {
    # Evaluate both expressions before assignment resets $?. The user prompt
    # sees the same status we capture, even when its own commands succeed.
    $commandSucceeded, $promptText = $?, (& $global:_rmux_original_prompt)
    $escape = [char]27
    $bell = [char]7
    $finished = ''
    if ($global:_rmux_command_running) {
        # Report normalized shell success, not a potentially stale LASTEXITCODE.
        $commandStatus = if ($commandSucceeded) { 0 } else { 1 }
        $finished = "$escape]133;D;$commandStatus$bell"
        $global:_rmux_command_running = $false
    }
    $directory = ''
    $location = Get-Location
    if ($location.Provider.Name -eq 'FileSystem') {
        $uri = [Uri]::new($location.ProviderPath)
        if ($uri.IsFile) { $directory = "$escape]7;$($uri.AbsoluteUri)$bell" }
    }
    return "$finished$directory$escape]133;A$bell$promptText$escape]133;B$bell"
}

# Delegate to the existing reader so PSReadLine history, completion and custom
# bindings remain in charge. A thrown/canceled read never starts a command.
if ($function:PSConsoleHostReadLine) {
    $global:_rmux_original_readline = $function:PSConsoleHostReadLine
    function global:PSConsoleHostReadLine {
        $line = & $global:_rmux_original_readline
        if (![string]::IsNullOrWhiteSpace($line)) {
            [Console]::Write("$([char]27)]133;C$([char]7)")
            $global:_rmux_command_running = $true
        }
        return $line
    }
}

# Preserve user aliases/functions. The installed CLI path is passed as data.
if ($env:RMUX_CLI -and $env:RMUX_CLAUDE_INTEGRATION -ne '0' -and
    -not (Get-Command claude -CommandType Alias,Function -ErrorAction SilentlyContinue)) {
    function global:claude {
        & $env:RMUX_CLI claude @args
    }
}
