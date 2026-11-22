# rmux

<img src="docs/brand/rmux.svg" width="72" height="72" alt="rmux: split panes and a terminal prompt">

**A native terminal workspace built for responsiveness.**

rmux combines Ghostty terminals with project workspaces, split panes, and
keyboard-first navigation. Built in Rust with a GPU-rendered interface, it puts
input latency, idle CPU, and memory use at the center of development.

Keep your shells and coding agents together without losing sight of what is
running. See the [performance measurements and remaining work](docs/performance.md)
for the current evidence; cross-platform performance acceptance is still in progress.

![rmux with project workspaces and split terminals](docs/images/rmux.png)

[Download rmux](https://github.com/rmuxapp/rmux/releases) ·
[Getting started](docs/trying-rmux.md) · [User guide](docs/user-guide.md)

## A home for your terminal work

- **Organize by project.** Keep named workspaces in the sidebar, with groups,
  pinned entries, colors, descriptions, working directories, and Git branches.
- **See more at once.** Arrange terminal tabs in nested horizontal and vertical
  splits. Resize panes, zoom into one, or move work between windows.
- **Stay on the keyboard.** Jump between workspaces and tabs, search with the
  command palette, and configure shortcuts for common actions and commands.
- **Keep track of background work.** In-app notifications and unread indicators
  help you find terminals that need attention.
- **Make it yours.** Adjust text size, padding, terminal colors, cursor behavior,
  and copy-on-select from Settings or the configuration file.
- **Automate your workspace.** The optional local socket and bundled `rmuxctl`
  let scripts create workspaces, send input, read terminal output, and notify you.

## Install

Download a preview from [Releases](https://github.com/rmuxapp/rmux/releases):

| Platform | Download | Install |
| --- | --- | --- |
| macOS, Apple Silicon | `.dmg` | Open it and drag **rmux.app** into **Applications**. |
| Linux, x86-64 | `.tar.gz` | Extract it and run `./rmux` in the extracted directory. |
| Windows, x86-64 | `.zip` | Extract the whole ZIP and run `rmux.exe`, keeping its DLL beside it. |

The macOS app and DMG are Developer ID signed and notarized. **rmux → Check for
Updates…** uses Sparkle; the macOS ZIP is also available for manual installation
and updates. Linux builds target Ubuntu 24.04-compatible systems with X11 or
Wayland and working graphics drivers. Windows builds are currently unsigned.
Linux and Windows are early platform previews with native UI acceptance still
in progress; they do not yet have automatic updates. Every archive includes
`rmuxctl` and dependency notices. Release checksums are in `SHA256SUMS`.

## Start here

Use VS Code? [Set up rmux as your external terminal workspace](docs/guide/vscode.md)
with **Cmd+Shift+C in both directions** on macOS (v0.1.7+).

In VS Code User Settings JSON:

```json
"terminal.external.osxExec": "/Applications/rmux.app"
```

In rmux Settings JSON, merge these entries into your existing arrays, then choose
**Reload Settings**:

```json
{
  "commands": [
    {
      "name": "Focus VS Code",
      "mode": "background",
      "argv": [
        "/usr/bin/open",
        "-a",
        "Visual Studio Code"
      ]
    }
  ],
  "keybindings": [
    {
      "key": "cmd-shift-c",
      "command": "Focus VS Code",
      "context": "window"
    }
  ]
}
```

From VS Code, the shortcut opens or reuses the project's running terminals.
From rmux, it focuses VS Code. The return command focuses the app, not a specific
project window; macOS controls switching between Spaces.

Open a workspace for each project, split terminals for related tasks, and use the
command palette to find actions without memorizing every shortcut. **⌘,** opens
Settings. See [Getting started](docs/trying-rmux.md) for navigation, session
recovery, and configuration.

## Early, and moving quickly

rmux is an early preview inspired by [cmux](https://cmux.com/). It is usable, but
full feature and interaction parity is still in progress. The development branch
includes opt-in [desktop notifications](docs/guide/notifications.md), with native
delivery and click behavior still awaiting platform acceptance. The experimental
browser is separate from workspace panes; complete browser surfaces and remote
workflows remain in progress. Features on main may be newer than your installed
release; check its [release notes](https://github.com/rmuxapp/rmux/releases).
Linux and Windows packages are experimental; cross-platform behavior is still being validated.
Performance is a design priority, not a promise of zero lag or a fixed memory
footprint on every workload.

## Building and contributing

Development tools are managed with **mise**. See [Building](docs/building.md),
[Progress](docs/progress.md), and [Performance measurements](docs/performance.md).
Please report bugs and feature requests in
[Issues](https://github.com/rmuxapp/rmux/issues).

Powered by [Ghostty](https://ghostty.org/), [GPUI](https://www.gpui.rs/), and
[Sparkle](https://sparkle-project.org/). Third-party notices are included in the
macOS app bundle.

## License

rmux is licensed under the [MIT License](LICENSE). Dependencies retain their
own licenses; release packages include third-party notices.
