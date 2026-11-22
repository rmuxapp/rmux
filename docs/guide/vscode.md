# Use rmux as your VS Code terminal workspace

Keep your editor in VS Code and your shells, dev servers, and coding agents in
rmux. Open the project's terminal workspace with **Cmd+Shift+C**, then return to
the same running terminals whenever you need them.

Native integration is available on macOS starting with **rmux 0.1.7**. It uses
VS Code's built-in external-terminal command. No extension, task, extra flags,
or socket configuration is needed.

## Set it up once

Install rmux in Applications. In VS Code, run **Preferences: Open User Settings
(JSON)** and add this property to your existing settings:

```json
"terminal.external.osxExec": "/Applications/rmux.app"
```

Open a local project folder and press **Cmd+Shift+C**. rmux launches if needed,
opens a workspace named after the folder, and requests focus for its window.
New shells start in the project directory. With several folders open, VS Code
chooses the directory passed to its external-terminal command.

## Keep your terminals running

Split panes for a shell, dev server, tests, or an agent. Press Cmd+Shift+C again
from that project to reuse its workspace; rmux keeps the existing processes,
layout, and working directories. Switching to another project in VS Code opens
that project's workspace instead.

Matching uses the folder name. Two projects with the same folder name will reuse
the same workspace. For explicit names or window targeting, use
[open-workspace through the CLI](automation.md#open-or-reuse-a-named-workspace).
Renaming a workspace means its old folder name no longer matches it.

## Switch back to VS Code

Cmd+Tab works as usual. To add a dedicated shortcut in rmux, merge these entries
into your rmux Settings JSON, then choose **Reload Settings**:

```json
{
  "commands": [{
    "name": "Focus VS Code",
    "mode": "background",
    "argv": ["/usr/bin/open", "-a", "Visual Studio Code"]
  }],
  "keybindings": [{
    "key": "cmd-alt-v",
    "command": "Focus VS Code",
    "context": "window"
  }]
}
```

This focuses VS Code as an application, not a specific editor window. macOS
controls switching between Spaces; cross-Space behavior has not yet been
verified. See [background commands](settings.md#background-commands-and-app-shortcuts)
for other app-switching shortcuts.

## If you tried the task-based setup

Remove the custom Cmd+Shift+C task binding from VS Code's Keyboard Shortcuts
JSON to restore its default **Open New External Terminal** command. The old
rmux tasks can also be removed. Keep socket automation enabled only if you use
it for other scripts; this workflow does not need it.

If nothing opens, check the app path and your installed rmux version. This native
setup covers local macOS folders, not Remote SSH, containers, or the browser demo.
