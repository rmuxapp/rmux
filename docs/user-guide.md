# rmux user guide

rmux keeps project workspaces, terminal tabs, and split panes together in one
native desktop app. Start with the everyday workflows below; detailed settings
and scripting instructions have their own pages.

> **Preview status:** Features and platform support are still evolving. Saving a
> session restores its layout with fresh shells—it does not preserve running
> processes. See [current progress](progress.md) for acceptance status.

## In this guide

- [Try the browser demo](#try-the-browser-demo)
- [Install and open rmux](#install-and-open-rmux)
- [Your first workspace](#your-first-workspace)
- [Everyday shortcuts](#everyday-shortcuts)
- [Arrange workspaces, tabs, and panes](#arrange-workspaces-tabs-and-panes)
- [Search, copy, and paste](#search-copy-and-paste)
- [Browse files](#browse-files)
- [Connect over SSH](#connect-over-ssh)
- [Save and restore your layout](#save-and-restore-your-layout)
- [Customize rmux](#customize-rmux)
- [Notifications and diagnostics](#notifications-and-diagnostics)
- [More detailed guides](#more-detailed-guides)

## Try the browser demo

Open [rmux.app](https://rmux.app) to try workspaces, tabs, split panes, and terminal
search without installing the app. Sessions use bundled sample commands:
`help`, `ls`, `logs`, `clear`, and `echo <text>`. They cannot access your files or
run programs on your computer. Refreshing the page starts fresh.

Right-click a tab to rename, move, or close tabs. **Shift+F10** opens the active
tab's menu. Right-click terminal output for copy, paste, selection, find, and
scrollback actions. These menus are also available through **Show Tab Actions**
and **Terminal Actions** in the command palette.

**Copy with Line Breaks** keeps the terminal's visible row breaks; normal copy
joins lines that wrapped at the pane edge. **Clear Scrollback** removes older
history while keeping the current screen. Multiline sample paste runs complete
lines and leaves the final unfinished line ready to edit. Sample input is limited
to 4 KiB. If the browser denies menu clipboard access, use your keyboard paste
shortcut instead.

Rename a workspace with **Cmd+Shift+R** (Ctrl+Shift+R elsewhere) or double-click
its sidebar name. Use the palette to pin workspaces or create, rename, and close
workspace groups. Closing a group removes its sample sessions.

Use **Edit Workspace Description**, **Edit Workspace Color**, or **Edit Workspace
Icon** in the palette to personalize the sidebar. Groups have their own color
and icon commands. Choose a preset or type a value, then press **Enter** to save
or **Esc** to cancel. Empty input clears the value. Editing a workspace inside a
collapsed group reveals it automatically. These changes last until you refresh
the demo.

**Cmd+=**, **Cmd+-**, and **Cmd+0** increase, decrease, and reset terminal text
size (Ctrl elsewhere). New tabs and splits inherit the active terminal's size;
resetting one terminal leaves the others unchanged.

The demo shares rmux UI code and the Ghostty terminal engine. Desktop-only
features and some browser interactions are still being brought over.

## Install and open rmux

Download a package from [GitHub Releases](https://github.com/rmuxapp/rmux/releases).
Use the assets available for that release:

| Platform | Package | What to do |
| --- | --- | --- |
| macOS, Apple Silicon | DMG or ZIP | Move **rmux.app** into **Applications**, then open it. |
| Linux, x86-64 | tar.gz | Extract it and run `./rmux` in the extracted folder. |
| Windows, x86-64 | ZIP | Extract all files together, then open `rmux.exe`. Keep its DLL alongside it. |

Linux packages target Ubuntu 24.04-compatible systems with a graphical session
and working graphics drivers. Linux and Windows remain experimental; Windows
packages are unsigned. On macOS, use **rmux → Check for Updates…** for Sparkle
updates. Linux and Windows currently require manual updates.

Building from source? Use [Building rmux](building.md).
Using mise? See [Install with mise](install-with-mise.md), including the prerelease setting.

## Your first workspace

1. Open rmux and create a workspace for your project.
2. Rename it so it is easy to recognize in the sidebar.
3. Split the terminal to keep a shell, development server, or coding agent visible.
4. Open the command palette when you need an action you don’t remember.

A **workspace** is a project-sized collection of panes. A **pane** occupies part
of the window and can contain multiple terminal **tabs**. Switching between them
keeps the terminal processes running.

## Everyday shortcuts

These are defaults. Custom bindings appear in menus and the command palette.

| Task | macOS | Linux / Windows |
| --- | --- | --- |
| Split right | Cmd+D | Ctrl+Shift+D |
| Split below | Cmd+Shift+D | Ctrl+Shift+E |
| Focus a neighboring pane | Cmd+Ctrl+Arrow | Alt+Shift+Arrow |
| Find in the current terminal | Cmd+F | Ctrl+Shift+F |
| Paste | Cmd+V | Ctrl+Shift+V |
| Close the current tab | Cmd+W | Ctrl+Shift+W |
| Rename the workspace | Cmd+Shift+R | Ctrl+Shift+R |
| Open Settings | Cmd+, | Ctrl+, |

On macOS, **Cmd+P** opens the command palette, **Cmd+T** opens a tab, and
**Cmd+B** toggles the sidebar. Menus and the palette also expose these actions
without requiring a shortcut.

## Arrange workspaces, tabs, and panes

Use the sidebar to switch workspaces. Pin frequently used ones, add a color or
description, and organize related workspaces into groups.

- **Resize a split:** drag its divider. Double-click to balance adjacent panes.
- **Focus one pane:** use **Zoom Pane**, then toggle it again to return to the layout.
- **Find a tab:** open **Choose Terminal Tab** and search its title or directory.
- **Move work:** use the palette’s move, detach, join, or merge actions.
- **Resize the sidebar:** drag its right edge; double-click to restore its width.

Moving a live tab or workspace preserves its processes. Restoring a saved layout
starts new ones. Close confirmations depend on your configured close policy.

[Workspace, tab, and pane details →](guide/workspaces.md)

## Search, copy, and paste

Use **Find** to search the focused terminal. Use **Search Terminal Output…** in
the Edit menu or command palette to search across terminals.

Drag to select text, double-click a word, or triple-click a line. **Enter Copy
Mode** opens a stable history snapshot with keyboard navigation and selection,
while the terminal continues running. Named buffers let you save and reuse text.

On macOS, copying files in Finder and pasting into a terminal inserts quoted file
paths. File drops do the same on macOS/Linux. Clipboard images on macOS/Linux are
saved locally and pasted as paths; they are not uploaded to remote hosts.

[Search, clipboard, and copy-mode details →](guide/terminal.md)

## Browse files

Choose **Browse Terminal Directory…** to inspect the local directory reported by
the focused shell. You can filter entries, navigate folders, preview text or hex,
and open Markdown in a reading view.

Use **External Editor** to open a file with your configured editor. These tools
browse local files; an SSH terminal’s remote path does not grant remote browsing.

[File browsing and preview details →](guide/files.md)

## Connect over SSH

Choose **File → Connect via SSH…**, enter an SSH alias, hostname, or `user@host`,
and select **Connect**. To specify a port, use `ssh://user@host:2222`.

**File → SSH Hosts…** lists discovered aliases and saved destinations. rmux uses
your existing SSH client and configuration; authentication prompts appear inside
the terminal. A saved SSH target is not automatically reconnected during restore.

[SSH connection details and limits →](guide/remote.md)

## Save and restore your layout

- **Save Layout…** saves the current window’s workspace arrangement.
- **Save Session…** saves the arrangement across normal rmux windows.
- **Open Layout… / Open Session…** restore those arrangements with fresh shells.
- Named presets make frequently used arrangements available from the palette.

Session recovery is opt-in. Neither a layout nor a recovery checkpoint resurrects
running commands or terminal scrollback.

[Sessions, presets, and recovery →](guide/sessions.md)

## Customize rmux

Open **Settings** to adjust font size, padding, terminal themes, cursor behavior,
background opacity, and other preferences. Use **Save these settings** to retain
your changes.

Advanced options live in `config.json`, normally under `~/.config/rmux/` or the
configured XDG directory. **Open Settings File** opens it; **Reload Settings**
applies edits. No configuration file is created automatically.

[Settings and custom shortcuts →](guide/settings.md) ·
[Shell selection and integration →](guide/shells.md)

## Notifications and diagnostics

Unread markers help identify background terminals that need attention. Open the
notification panel to read, copy, acknowledge, or dismiss messages. Desktop
notification banners can be enabled in Settings.

**Show Listening Ports** lists local ports and can open a selected service in your
external browser. **Terminal Memory** shows diagnostic snapshots. Error banners
can be dismissed; they do not imply that a failed action succeeded.

[Notifications →](guide/notifications.md) · [Ports and diagnostics →](guide/diagnostics.md)

## More detailed guides

| Topic | What you’ll find |
| --- | --- |
| [Workspaces and panes](guide/workspaces.md) | Groups, transfers, tab pickers, resizing, and close behavior |
| [Terminal and clipboard](guide/terminal.md) | Search, copy mode, buffers, selection, and input limits |
| [Files and Markdown](guide/files.md) | Directory browsing, previews, and external editors |
| [Settings](guide/settings.md) | Themes, configuration keys, custom commands, and shortcuts |
| [Shell integration](guide/shells.md) | Shell selection, startup behavior, and prompt navigation |
| [SSH](guide/remote.md) | Hosts, authentication, and saved destinations |
| [Sessions and recovery](guide/sessions.md) | Layout files, presets, and crash checkpoints |
| [Notifications](guide/notifications.md) | Unread activity, messages, and shell notifications |
| [Diagnostics](guide/diagnostics.md) | Ports, memory, and error messages |
| [CLI and automation](guide/automation.md) | `rmuxctl`, socket setup, commands, and RPC examples |
| [Experimental browser](guide/browser.md) | Source-build-only Find, Inspector, and navigation controls |

For development status, see the [roadmap](roadmap.md) and [progress log](progress.md).
Report problems in [GitHub Issues](https://github.com/rmuxapp/rmux/issues).
