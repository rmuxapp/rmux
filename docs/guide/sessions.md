# Layouts, sessions and recovery

[← User guide](../user-guide.md)

## On this page

- [Save and open sessions](#save-and-open-sessions)
- [Named layout and session presets](#named-layout-and-session-presets)
- [Startup restoration and crash recovery](#startup-restoration-and-crash-recovery)
- [Launch a saved layout](#launch-a-saved-layout)
- [Save a layout](#save-a-layout)

## Save and open sessions

File → Layouts and Sessions → Save Session… (also in the command palette) captures all normal rmux
windows into one file. Capture refuses while a window is restoring, editing or
waiting for a close decision. The selected path is written atomically on a
background executor. This saves the state when the action was invoked.
File → Layouts and Sessions → Open Session in New Windows… imports that file into the running app,
leaving existing windows and sessions intact. It prepares fresh shells off the UI
thread, then opens the saved windows and activates the saved selection. No running
process or command history is resumed. All saved working directories must exist;
empty session files are rejected by this command. Closing the initiating window
cancels pending preparation. If native window creation fails partway through,
windows opened by this import are removed; existing windows remain. The command
is also available in the palette and as the `open_session` custom shortcut action.

Layout and session imports show the terminal preparation count. Cancel using the
status button, Cancel Layout / Session Restore in the palette, or a `cancel_restore`
custom shortcut. Cancellation stops at the next preparation boundary and cleans
up shells already prepared; it cannot interrupt a shell-spawn system call.


The repository includes [development and services layout examples](../../examples/layouts/README.md)
with customization and preset setup instructions.

## Named layout and session presets

For repeatable workspace setups, save a layout or session once and add named
presets to your settings. Replace these example paths with your saved files:

```json
{
  "layout_presets": [
    {"name":"Development", "path":"/absolute/path/development.json", "cwd":"/absolute/project"},
    {"name":"All Projects", "path":"/absolute/path/projects.json", "session":true}
  ],
  "keybindings": [{"key":"ctrl-k d", "preset":"Development"}]
}
```

After Reload Settings, choose **Open Preset: Development** in the palette or use
its shortcut. You can also launch `rmux --preset "Development"` from a shell.
File → Layouts and Sessions → Open Layout Preset… opens a dedicated searchable picker. It shows the
layout/session type, optional project directory and source file, and includes
configured shortcut hints. Search matches those details as well as the name.
The `layout_preset_picker` shortcut action opens the same picker. If none are
configured, it explains how to add presets and reload settings.

The name must match configuration exactly. `--rpc-stdio-live` may be combined with
`--preset`; explicit files, `-d`, commands and config inspection cannot. Set the
preset's `cwd` in settings instead. Like other explicit restores, a preset launch
neither loads nor overwrites the normal automatic recovery file. Each invocation opens fresh windows/shells in the directories saved
in the file; it does not replay prior terminal commands or resume running jobs.
The file is read when invoked, so edits apply to the next use. Layout presets
(default `session:false`) contain one window; session presets contain multiple.
An optional absolute `cwd` supplies the directory for tabs whose saved `directory`
is null. Explicit saved tab directories take precedence. This lets several named
presets reuse one template for different projects without editing the template.

The resulting directories are checked before any shell starts; the source file
is never modified by applying a preset.
Up to 32 presets are allowed, with unique names and absolute paths. Invalid files
or missing working directories report an import error. Configuration loading does
not read preset files. A binding selects exactly one `action`, `command`, `text`,
or `preset`; preset shortcuts resolve the current configuration at invocation.

## Startup restoration and crash recovery

Use `rmux --session PATH` to restore a version-1 multi-window recovery file with
fresh shells. It preserves window order and the selected window; an empty session
opens one default window. `--check-session PATH` validates structure without
launching shells or checking directories. Session files allow at most 16 windows,
256 total tabs and 8 MiB. `--rpc-stdio-live` works with session restore.
Process/output history is not resumed.
Saved layouts and sessions also include window position, size and display mode.
Pane zoom is saved per workspace; restoring it retains the underlying split tree
and proportions, and focuses the visible pane. Older files default to unzoomed.

Older files use default placement. If displays change, restore clamps the window
to an available display. Moving/resizing a window schedules a checkpoint when
automatic recovery is enabled. Fullscreen/maximized and multi-monitor visual
acceptance remain pending.

Normal launches automatically save and restore your workspaces. Quit rmux and
reopen it to recover window layouts, workspace names and groups, splits, tabs,
selection, and working directories with fresh shells. Closing the final window
also retains that window for the next launch. Explicitly closing a workspace or
terminal still removes it; running processes and scrollback are not resumed.

The default recovery file is `config.session.json` beside your settings file
(`~/.config/rmux/config.session.json` by default on macOS/Linux, or
`%APPDATA%\rmux\config.session.json` on Windows). An `RMUX_CONFIG` override uses
its own sibling `<config-stem>.session.json`, so separate profiles stay isolated.
Set `"restore_session": false` to disable automatic saving and restoration, or
set `"session_file": "/absolute/path/to/rmux-session.json"` to choose a custom
location. Disabling restoration leaves the existing saved file untouched. Missing parent directories are created automatically (private to
your user on macOS/Linux); existing files and directory permissions are preserved.
Recovery requires an
exclusive OS lock on a sibling `.lock` file. Another
process using that recovery file opens a fresh window with recovery disabled,
so it cannot overwrite the owner's checkpoint. The lock file remains on disk;
ownership is released by the OS when the process exits, including forced stops.
Normal launches restore
that file if present; normal Quit captures the current windows and awaits an
atomic background save within the shared three-second cleanup deadline. Closing the final window captures it before teardown. Closing the last workspace
or terminal explicitly records an empty session. Corrupt/unrestorable files log an error and
open a fresh window with a recovery notice. Automatic saving is disabled for that
launch so the original file remains available for repair; Save Session can write
the new session separately. Lock failures also produce a recovery notice. Explicit
`--layout`, `--session`, directory/command launches and benchmarks neither restore
nor overwrite the configured recovery file. This setting is frozen at startup;
reload takes effect on the next launch. Saves can be skipped during an active
editing/restore/close interaction. Background save failures appear in the window
and are logged; identical repeated failures do not keep reopening the notice.
The next workspace change retries saving. OS-originated quit has
GPUI's shorter best-effort deadline. Window/workspace/pane changes also schedule
a checkpoint after 750 ms, coalescing notifications in a one-slot queue. There is
no idle polling or terminal-frame observer. Writes are serialized, unchanged
serialized state is skipped, and older jobs cannot replace newer checkpoints.

Deduplication retains one encoded session (at most 8 MiB) while enabled.
Directory snapshots use delivered shell metadata without waiting for the terminal
parser; an explicit startup directory is retained until a directory report arrives.
Recent changes may be lost before a checkpoint completes. The native smoke verifies
forced-stop/relaunch from a completed checkpoint, with fresh shells and preserved
layout/directory metadata. Power-loss durability, interruption during file
replacement and high-output capture performance remain unverified.

## Launch a saved layout

Use `rmux --layout PATH` to start with a saved layout and fresh shells. Layout
validation and shell preparation finish before the first window opens; failures
return exit code 2. This cannot be combined with `--working-directory` or benchmark
mode. It restores layout metadata, not running processes or scrollback.

## Save a layout

File → Layouts and Sessions → Save Layout… exports the current window's workspaces, splits, tabs, focus
and reported directories to a JSON file. File → Layouts and Sessions → Open Layout in New Window… reads
that file and starts fresh shells in the saved directories, restoring names,
tabs, split proportions, focus and sidebar width. Preparation happens in the
background with a Cancel control; errors leave existing windows intact. Closing
the originating window also cancels preparation. Automatic restoration is not
implemented. Layout files do not save running processes or terminal scrollback.
