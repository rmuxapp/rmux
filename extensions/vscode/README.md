# rmux for VS Code

Press **Cmd+Shift+C** on macOS, or **Ctrl+Shift+C** on Linux, to open your
project in rmux. Repeating it reuses the named workspace and its running shells.
The command talks directly to rmux without starting a task, terminal panel, CLI,
or shell process. There are no runtime dependencies or background polling.

## Setup

This extension is currently source-only and has not been published or installed.
To try it in an Extension Development Host, launch VS Code with
`--extensionDevelopmentPath=/absolute/path/to/rmux/extensions/vscode`.

Enable automation in rmux's Settings JSON and restart rmux:

```json
"socket_directory": "/Users/your-name/.rmux-automation"
```

Leave that directory for rmux to create. The default VS Code setting
`rmux.socketPath` is `~/.rmux-automation/rpc.sock`; change it in User Settings
if your endpoint differs. rmux must already be running.

Remove any earlier Cmd+Shift+C task binding from your user keyboard shortcuts;
user bindings take precedence over the extension's default. Alternatively,
replace the binding with:

```json
{
  "key": "cmd+shift+c",
  "command": "rmux.openProject",
  "when": "workspaceFolderCount > 0 && isWorkspaceTrusted && !remoteName"
}
```

The command also appears as **rmux: Open Project Workspace** in the palette.
It uses the active editor's project folder. With multiple folders and no active
project, it asks you to choose. Workspace names match VS Code's folder names;
rename duplicate folder labels to keep projects separate. Existing rmux
workspaces retain their working directory when reused.

Local macOS and Linux folders are supported. Remote, virtual, untrusted, and
Windows workspaces are rejected rather than opening a similarly named local
directory. Foreground activation requires the rmux fix after v0.1.6; older
releases may select the workspace without bringing the app forward.

## Development

Use `mise run test` in this directory. Tests use disposable local Unix sockets
and a simulated VS Code command host; native extension-host acceptance and
switching latency measurement remain pending. No tools are installed by npm.
