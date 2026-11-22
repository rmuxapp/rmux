# Shell setup and integration

[← User guide](../user-guide.md)

## On this page

- [Choose the shell](#choose-the-shell)
- [Starting directory](#starting-directory)
- [Automatic shell integration](#automatic-shell-integration)
- [Bash integration](#bash-integration)
- [Login shells and fish integration](#login-shells-and-fish-integration)

## Choose the shell

Set `RMUX_SHELL` when launching rmux to choose the executable for new shell
terminals, including fresh shells restored from layouts. It takes precedence
over `SHELL` on Unix and `COMSPEC` on Windows. Supply one executable path/name,
not a command string; paths containing spaces remain literal. An empty override
is an error. Unix shells retain interactive login arguments (`-i -l`), and zsh
retains its existing integration. Use the startup `-- PROGRAM ARG…` form when
you need custom arguments. This override does not install or change your shell.

## Starting directory

Launch with `rmux --working-directory PATH` (or `-d PATH`) to choose the default
directory for new shells. A focused terminal's reported directory takes precedence
for inherited tabs/splits. `--help` and `--version` exit without opening the UI;
invalid arguments or directories return exit code 2.
`rmux --rpc-stdio` provides standalone discovery without opening a window or
starting a shell. Send one JSON-RPC 2.0 object per line, for example
`{"jsonrpc":"2.0","id":1,"method":"system.capabilities"}`. Supported methods
are `system.ping` and `system.capabilities`; parameters must be absent or empty
(`{}` or `[]`). Responses use stdout, diagnostics stderr, and notifications
produce no response. Each request is limited to 64 KiB including its newline;
exceeding that limit closes the stream with exit code 2. Batch requests are not
supported. Capabilities explicitly report `live_workspace_access: false`: this
entry point does not connect to existing windows or claim cmux wire compatibility.

## Automatic shell integration

New zsh sessions automatically load prompt and directory integration through a
private temporary startup wrapper. Existing user startup files run in order;
their `ZDOTDIR` is restored before `.zshrc`, and no shell configuration files are
modified. Set `RMUX_SHELL_INTEGRATION=0` before launching rmux to opt out.
The hooks emit OSC 133 boundaries and percent-encoded OSC 7 paths using built-ins.
For an already-running shell, source `shell-integration/zsh/rmux.zsh` manually;
the macOS bundle also carries it under `Contents/Resources/shell-integration/zsh/`.
Fish automatically loads its hooks after normal shell configuration for both
login and non-login sessions. PowerShell loads its hooks automatically after normal
profiles through a private temporary script, without editing profiles or changing
execution policy. Unix login startup places `-Login` first; `-NoExit -Command`
loads the hook and keeps the shell interactive. `RMUX_SHELL_INTEGRATION=0` opts out.

For existing shells, dot-source `shell-integration/powershell/rmux.ps1` after defining
your prompt. The macOS bundle includes it under
`Contents/Resources/shell-integration/powershell/`.
The wrapper preserves prompt output and emits OSC 7 plus OSC 133 A/B. When an
existing console line reader is available, it delegates to that reader and emits
OSC 133 C for nonempty commands, then D at the next prompt. Completion reports
normalized shell success (`0`) or failure (`1`) from `$?`, captured before the
user prompt runs while preserving the status that prompt sees. It does not report
exact native exit codes: `LASTEXITCODE` can be stale after PowerShell commands.
Empty or thrown/canceled
reads emit no command markers. Existing PSReadLine bindings remain in charge. A real macOS PowerShell PTY fixture
covers profile loading, Unicode directory changes and prompt navigation. Windows,
complex prompt themes and native PSReadLine interaction remain unvalidated.

Run `mise run test-powershell-integration` for prompt semantics and
`mise run test-powershell-startup` for the macOS PTY fixture. Run
`mise run test-zsh-integration` for the focused hook check.

## Bash integration

Bash reports completed-command status through OSC 133 D before the next prompt.
A leading hook captures the command's status and returns it unchanged for the
first user prompt hook; the trailing integration hook emits the captured value.
User hooks therefore cannot accidentally turn a failed command into a successful
status report. The first prompt does not emit a synthetic completion. This does
not install a DEBUG trap. Bash 4.4+ prepends an OSC 133 command-start marker to
`PS0`, preserving its existing contents; Bash 3.2 retains prompt/completion markers
without command-start reporting. Prompt themes that later replace PS0 can remove
the start marker.


Bash integration can currently be sourced manually from
`shell-integration/bash/rmux.bash` (also included in newly built macOS bundles).
It reports encoded working directories and prompt boundaries, preserves existing
`PROMPT_COMMAND` hooks, and supports `RMUX_SHELL_INTEGRATION=0`. Run
`mise run test-bash-integration` for its focused check. Automatic Bash login startup
is still pending. The fixture checks command-start emission on Bash 4.4+ and
unchanged PS0 on older versions. It passed locally on macOS Bash 3.2 and in
[GitHub CI on Linux Bash 5.2.21](https://github.com/rmuxapp/rmux/actions/runs/44850857084).
Plain and exported prompt-hook arrays also passed order/status/idempotence checks
on [Linux Bash 5.2.21](https://github.com/rmuxapp/rmux/actions/runs/44851031041).
Native prompt-theme acceptance remains pending.

## Login shells and fish integration

Unix shells launch as interactive login shells by default. Set `RMUX_SHELL_LOGIN=0`
before launching rmux to use interactive non-login shells instead. In this mode,
Bash automatically loads integration through a private `--rcfile` wrapper that
sources the user's `.bashrc` first. No user startup files are modified. Bash login
shells retain their normal startup path and require manual sourcing; use
`RMUX_SHELL_INTEGRATION=0` to disable automatic hooks in either supported shell.
The login-mode setting affects default-shell tabs, splits and restored layouts,
not explicit commands passed after `--`, and has no effect on Windows.
The macOS PTY fixture verifies both zsh and non-login Bash directory reporting,
Unicode path decoding, prompt navigation through scrollback and shell exit.


Fish integration loads automatically after user configuration, in both login modes.
Modern fish supplies OSC 7 directory reports and OSC 133 prompt/command markers;
rmux leaves those native markers in charge instead of registering duplicate hooks,
including when the user disables fish's `mark-prompt` feature. Fish versions before
3.7 use the bundled event-hook fallback, which does not emit input-start markers.
The script remains available at `shell-integration/fish/rmux.fish` for manual use.
`RMUX_SHELL_INTEGRATION=0` disables rmux's script; it does not disable fish's own
native terminal integration.

`mise run test-fish-integration` uses pinned, mise-managed fish 4.9.3. Its startup
checks pass on macOS, covering user configuration, login modes and avoiding duplicate
hooks. `mise run test-fish-startup` passes a real macOS PTY fixture with Unicode cwd
and prompt navigation. The startup checks also pass on Ubuntu CI. Older fish fallback and Linux native
PTY coverage remain pending.
Event behavior follows
the [fish event documentation](https://fishshell.com/docs/current/language.html#event-handlers)
and [built-in URL escaping](https://fishshell.com/docs/current/cmds/string-escape.html).
