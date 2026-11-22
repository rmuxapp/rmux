# Settings and keyboard shortcuts

[← User guide](../user-guide.md)

## On this page

- [Background opacity and blur](#background-opacity-and-blur)
- [Choose a terminal font](#choose-a-terminal-font)
- [Use the Settings panel](#use-the-settings-panel)
- [New workspace placement](#new-workspace-placement)
- [Custom commands](#custom-commands)
- [Custom keyboard shortcuts](#custom-keyboard-shortcuts)
- [Send literal text from shortcuts](#send-literal-text-from-shortcuts)
- [Shortcut display and action names](#shortcut-display-and-action-names)
- [Last-terminal shortcut](#last-terminal-shortcut)
- [Configuration file](#configuration-file)
- [Scrollback budget](#scrollback-budget)
- [Padding and copy on select](#padding-and-copy-on-select)
- [Option key and cursor behavior](#option-key-and-cursor-behavior)
- [Terminal colors and palette](#terminal-colors-and-palette)
- [Theme presets](#theme-presets)
- [Import terminal themes](#import-terminal-themes)
- [Cursor and selection colors](#cursor-and-selection-colors)
- [Font sizes and configuration checks](#font-sizes-and-configuration-checks)
- [Close confirmations](#close-confirmations)
- [Workspace group shortcuts](#workspace-group-shortcuts)

## Background opacity and blur

**Settings → Terminal background opacity** adjusts the default terminal background
from 0–100% in 5% steps. **Blur behind terminal** requests platform blur below
100% opacity; availability depends on your OS/compositor. Text, explicitly colored
cells, tabs and sidebar remain opaque. Changes apply live across windows; use
**Save these settings** to retain them. JSON keys are `background_opacity` (integer
percentage, default 100) and `background_blur` (boolean, default false).
Native visual and compositor-performance acceptance remains pending.

## Choose a terminal font

**Choose Terminal Font…** searches installed font families for the current
terminal. Type to filter, use arrows and Enter to choose, or Escape to cancel.
Monospaced families suit the terminal grid best. The picker preserves text size;
**Use Configured Font** restores the configured family and size. The custom
action is `choose_font`. To change the default, use **Default font family** in
Settings or **Choose Default Font…** in the palette/menu (`choose_default_font`
for custom shortcuts). It updates terminals using the previous default family
and preserves per-terminal overrides and text size. Click **Save these settings**
to keep the default across launches.

## Use the Settings panel

Open Settings with Cmd+, on macOS (Ctrl+, elsewhere), the app menu or palette.
In narrow windows, Settings and Notifications overlay the right side instead of
shrinking the terminal layout. Wider windows keep them alongside the terminals.

The fixed footer keeps **Save**, **Open settings file**, **Reload from file** and
**Keyboard Shortcuts…** available while scrolling. Reload replaces session
settings with the saved file; an invalid file leaves the current settings intact.
The footer also shows **Unsaved session changes** when appearance controls differ
from the last loaded or successfully saved values. This status is shared across
windows. Edits made while a save is running stay marked as unsaved; a failed save
does not clear the status. It does not monitor external edits to the JSON file.
**Revert unsaved changes** restores the panel controls to their last loaded or
successfully saved values across all windows. It keeps commands, shortcuts,
explicit color overrides and environment-controlled close policy, and does not
read or write the settings file. Imported theme colors are restored from the
loaded snapshot, even if the theme file has changed or disappeared. It does not
repair that file; restore the file or choose a bundled theme before the next
reload/restart. Revert is unavailable during save/reload operations. The menu
and palette call this **Revert Unsaved Settings**; custom shortcuts can bind
`revert_settings`. This does not undo a previous save on disk.

**Undo last save** restores the settings file from rmux's most recent Save in
this session. Each value is restored only if it still matches what rmux saved;
later edits to that value are kept. Invalid files and redirected configuration
symlinks are left alone. Undo keeps one record in memory, shared across windows;
it is unavailable after restarting rmux and is consumed after a successful undo.
If you have unsaved session edits, those stay active; use **Reload from file**
when you want to apply the restored file. Otherwise the restored settings apply
immediately. The menu/palette action is **Undo Last Settings Save** and its custom
shortcut action is `undo_settings_save`.

Save and undo check for file changes again just before atomic replacement. This
detects intervening edits but does not lock external editors; simultaneous writes
from another process can still race the final check and replacement.

Use Tab or the arrow keys to select controls, then Enter to activate them.
The side panel adjusts text-size defaults, terminal padding/theme, unfocused
cursor, copy-on-select and close confirmation. Choose Active tasks, Always or Never
for close confirmation. Active tasks also requests confirmation when terminal
state is busy, so closing does not wait for output parsing. An `RMUX_CONFIRM_CLOSE`
override remains authoritative
and is not saved into your config. Active tasks uses reported task status. Changes apply for the session; Save persists those
fields while preserving other configuration options. Font defaults update matching terminals live while retaining custom size/family
values. Reset Font Size returns the focused terminal to the size default. The editor link exposes the remaining settings. Tab/Shift-Tab or up/down navigate
the panel; left/right adjust values, Enter/Space activate, and Escape closes it.
Font/theme pickers and editor actions open only with Enter/Space, not arrow keys. Opening
the panel moves keyboard focus out of the shell; closing restores its prior focus
when the panel still owns focus.

Open Settings panels follow shared changes across windows. While a save, reload,
or settings-file open is running, Save shows “Working with settings…” and file
actions are temporarily unavailable. Controls remain editable; saving an older
snapshot reports when newer session changes still need saving.

## New workspace placement

Settings → **New workspace position** controls ungrouped workspace creation:
**First**, **After current**, or **Last** (the default). The JSON field is
`new_workspace_placement`, accepting `top`, `after_current`, or `end`. This applies
to the New Workspace action, configured commands, and RPC workspace creation.
Background RPC creation preserves the selected workspace even when indices move.
Pinned workspaces remain above unpinned ones; placement after a grouped origin is
after that group's sidebar block. If the origin is pinned, the new unpinned
workspace becomes the first row below the pinned section. Group members retain the separate New group
workspace setting. Generated empty-group anchors retain their existing append
behavior. Save these settings to persist your choice.

## Custom commands

Custom palette commands can be configured in `config.json`:

```json
{
  "commands": [
    {"name": "Tests", "argv": ["mise", "exec", "--", "cargo", "test"], "cwd": "/absolute/project/path", "env": {"RUST_BACKTRACE": "1"}}
  ]
}
```

From a shell or launcher, use `rmux --command "Tests"` to start the configured
command in a new rmux window. This uses the same literal argv, child environment
overrides and configured `cwd`; without `cwd`, it inherits the launcher's current
directory. Names are exact and case-sensitive. This explicit launch bypasses
automatic session recovery and does not overwrite its saved file. Do not combine
`--command` with `--preset`, layout/session options, `-d`, or a program after `--`;
set the directory and arguments in the command definition instead.

After Reload Settings, choose **Run Tests** in the palette. For a list limited to
your saved commands, use **File → Run Configured Command…**, or assign the
`custom_command_picker` shortcut action. Search matches names and the displayed
directory/argument preview; environment values are omitted. By default (`"mode": "terminal"`), each invocation
starts a dedicated workspace with its own terminal; it does not inject text into
an existing shell. Arguments are literal, including spaces and empty arguments;
use an explicit shell executable only when shell interpretation is intended.
Without `cwd`, the selected workspace's remembered terminal directory is used
when available. Limits are 32 unique command names, 64 argv entries and 16 KiB
of argument text per command, within the config's overall 64 KiB limit. Optional
`env` overrides inherited/default variables for that child only; it supports up
to 64 portable variable names and 16 KiB of names/values, including empty values.

Only one custom command may be starting per window; repeat activation while it
is pending is rejected with a notice. Startup
runs off the UI thread; failures appear in the notice area. These definitions
are retained when saving appearance settings. Environment packs and the complete upstream command configuration contract
remain pending; shortcut overrides are described below.

## Background commands and app shortcuts

Set `"mode": "background"` to run a command without opening a terminal or
workspace. Use the same named `command` binding as other configured commands.
For example, on macOS this shortcut opens or focuses VS Code:

```json
{
  "commands": [
    {
      "name": "Focus VS Code",
      "mode": "background",
      "argv": ["/usr/bin/open", "-a", "Visual Studio Code"]
    }
  ],
  "keybindings": [
    {"key": "cmd-alt-v", "command": "Focus VS Code", "context": "window"}
  ]
}
```

Merge these entries into your existing arrays, then **Reload Settings**.
The palette also offers **Run Focus VS Code**. This is a general command launcher;
other apps and platforms can use their own executable and arguments. It focuses
VS Code as an application, not a particular project window.

Background commands use literal `argv`, optional absolute `cwd`, and child-only
`env` overrides. Without `cwd`, a shortcut uses the focused terminal's reported
local directory when available, otherwise rmux's launch directory. Input and
output are disconnected; use terminal mode for interactive commands or output
you need to read. Launch errors and unsuccessful exits appear in rmux's notice
area. At most eight background commands run concurrently. Long-running commands
retain a slot until they exit; closing a workspace does not terminate them.
They are independent processes and are not restored with sessions.

`rmux --command "Focus VS Code"` also supports background mode: it starts no GUI,
waits for the launcher command to exit, and returns success or failure. It cannot
be combined with `--rpc-stdio-live`. Existing definitions without `mode` retain
their terminal behavior.

## Custom keyboard shortcuts

**View → Keyboard Shortcuts…** lists effective command bindings for the current
focus context, including custom commands, literal terminal-text bindings and overrides.
Text bindings show an escaped excerpt and byte count; control characters do not
render as invisible commands. Search by action name
or key names. Modifier/key queries such as `ctrl tab` match actual bindings,
including aliases; command names remain fuzzy-searchable. To bind this view
itself, use the action `keyboard_shortcuts`.


Shortcut overrides can be added alongside custom commands:

```json
{
  "keybindings": [
    {"key": "ctrl-alt-y", "action": "settings", "context": "window"},
    {"key": "ctrl-k ctrl-t", "command": "Tests"},
    {"key": "ctrl-shift-p", "action": "disabled"}
  ]
}
```

The `Tests` command must exist in `commands`. Bindings default to `terminal`
context, so they do not override text editors; `window` makes a binding global.
Use `find` to target the terminal Find field, for example
`{"key":"ctrl-alt-n","action":"find_newer","context":"find"}`.
This scope supports disabling/remapping Find shortcuts without changing their
terminal bindings. Literal `text` bindings remain restricted to `terminal`.
Use `settings` to target only the Settings panel. While that panel is focused,
Cmd+S on macOS or Ctrl+S elsewhere saves the settings through the same action as
**Save these settings**. This binding leaves terminal input unchanged. Customize
it with `{"key":"ctrl-alt-s","action":"save_appearance","context":"settings"}`;
use `action: "disabled"` in the same context to disable the default key.

Use `preview` for text-file preview shortcuts. Markdown links expose
`next_preview_link`, `previous_preview_link` and `open_preview_link`. Defaults
are Tab, Shift+Tab and Alt+Enter only in the Markdown reading view.
 Wrap excerpts defaults to
Ctrl+Alt+W and can be remapped with
`{"key":"ctrl-alt-r","action":"toggle_preview_wrap","context":"preview"}`.
Disable the original key separately with `action: "disabled"` in that context.
The button tooltip reflects the active binding. This scope excludes hex previews,
ordinary command palettes, terminal input and Settings.
Use `file_preview` for bindings shared by text and hex previews: `refresh_preview`
(default F5), `preview_directory` (Alt+Up), and `switch_preview_mode` (Ctrl+Alt+M).
These controls work while filtering; their visible hints follow the active keymap.
For example, `{"key":"ctrl-alt-r","action":"refresh_preview","context":"file_preview"}`
adds a refresh shortcut without consuming terminal keys. Disable a default with
`action: "disabled"` in the same context. IME composition suspends these actions.

Preview navigation can be rebound with `preview_back` and `preview_forward`
under `context: "file_preview"`. Defaults are Cmd+[ / Cmd+] on macOS and
Alt+Left / Alt+Right elsewhere.

Use `files` for the directory browser: `file_back` (Alt+Left), `file_forward`
(Alt+Right), `file_parent` (Alt+Up), `refresh_files` (F5), and `file_location`
(Cmd+L on macOS, Ctrl+L elsewhere). These bindings work while filtering; unavailable
history/root destinations do nothing. The location action opens the existing
literal path field. Bindings and displayed hints can be customized independently
of terminal and file-preview shortcuts; IME composition suspends navigation.
The first stroke must use Ctrl/Cmd/Alt or F1–F24; one- and two-stroke bindings are
supported. The second stroke may be unmodified, for example
`{"key":"ctrl-k t","action":"new_tab"}`. Bare `t` remains normal typing.
Incomplete chords use GPUI's one-second timeout and replay handling; a leader
can therefore delay its ordinary terminal behavior. A small overlay shows the
pending keys while rmux waits for the next stroke; it does not take keyboard
focus or resize the terminal. The limit is 128 bindings. Duplicate normalized keys in the same
context, unknown key names/actions/commands and definitions with more than one
target fail config validation. A single stroke and a two-stroke chord cannot
share a prefix in overlapping custom contexts (for example a window-wide
`ctrl-k` and terminal `ctrl-k ctrl-t`). Terminal/Find and text/file-preview
contexts also overlap. Independent panels can reuse prefixes; two different
chords may share their first stroke. Error messages identify both entries and
their contexts. Function keys use F1–F24, without zero padding. Reload Settings replaces prior overrides and restores
removed bindings to defaults. Later overrides win within the same context;
a more specific terminal context takes precedence over a global binding.

`disabled` removes the matching app binding, allowing normal event handling.

## Send literal text from shortcuts

A binding may specify `text` instead of `action` or `command` to send exact UTF-8
bytes to the focused terminal, for example:

```json
{"keybindings":[
  {"key":"ctrl-k g","text":"git status"},
  {"key":"ctrl-k u","text":"\u0015"}
]}
```

The first inserts `git status` without pressing Enter; the second sends Ctrl-U.
JSON `\r` explicitly sends Return, and `\u001b` sends Escape. These are raw input
bytes, not bracketed paste: no newline, escaping or shell expansion is added by
rmux. Text bindings require terminal context and 1–16,384 UTF-8 bytes. They do not
send while a search/copy/ports overlay or confirmation prompt is active. Existing
leader-key, override and reload rules apply. Ghostty's configuration syntax and
named key tables are not yet imported.

## Shortcut display and action names

The palette shows shortcuts resolved for its originating focus context. Display
labels use macOS modifier symbols (for example ⇧⌘P) or Ctrl/Alt/Shift names on
other platforms. Pending chords use the same notation. Configuration still uses
GPUI strings such as `cmd-shift-p`; displayed labels are not configuration syntax.

Clipboard and lifecycle action names are `copy`, `paste`, `select_all`,
`clear_scrollback`, `close_tab`, `close_workspace`, `close_window`, and `quit`.
These invoke the same handlers as the menus, including close confirmations and
clipboard behavior. A `close_tab` binding closes the active tab, removing an empty
pane/workspace as usual. For example:

```json
{"keybindings":[
  {"key":"ctrl-k c","action":"copy"},
  {"key":"ctrl-k v","action":"paste"},
  {"key":"ctrl-k w","action":"close_tab"}
]}
```

Session/configuration actions include `save_layout`, `open_layout`, `save_session`, `open_session`,
`open_settings_file`, and `save_appearance`. These retain the existing dialogs and
save behavior. Diagnostics actions include `terminal_memory`,
`close_terminal_memory`, `copy_terminal_memory`, `cycle_terminal_memory_sort`,
`refresh_git_context`, `toggle_program_status`, and `dismiss_all_notifications`.
Use `"context":"window"` for a binding that should also work while a window panel
has focus; bindings otherwise require terminal context. No new defaults are added.

Supported action names: `new_workspace`, `new_window`, `new_tab`, `new_local_tab`, `split_right`,
`split_down`, `split_left`, `split_up`, `focus_left`, `focus_right`, `focus_up`, `focus_down`,
`next_workspace`, `previous_workspace`, `last_workspace`, `last_terminal`,
`next_pane`, `previous_pane`, `next_tab`, `previous_tab`, `tab_picker`,
`move_tab_left`, `move_tab_right`, `move_workspace_up`, `move_workspace_down`,
`previous_prompt`, `next_prompt`, `last_prompt`, `scroll_to_top`, `scroll_to_bottom`,
`find_older`, `find_newer`, `increase_font_size`, `decrease_font_size`,
`reset_font_size`, `use_configured_font`,
`palette`, `toggle_zoom`, `toggle_sidebar`, `settings`, `notifications`,
`next_unread`, `mark_all_read`, `find`, `reload_settings`, and `disabled`. Graphical binding editing,
complete upstream shortcut coverage remain pending. Invalid custom bindings identify
their zero-based `keybindings` array index and context. Duplicate and chord-prefix
errors also identify the conflicting entry; bindings in different scopes remain
valid. The shortcut browser lists all effective aliases for each listed command,
with shadowed bindings omitted. Ordinary menu hints keep one preferred binding.
Numbered navigation is also configurable: `select_tab_1`…`select_tab_9`,
`select_pane_1`…`select_pane_9`, and `select_workspace_1`…`select_workspace_9`.
For example, `{"key":"ctrl-k 3","action":"select_tab_3"}` selects the third
tab in the focused pane. These dispatch the existing numbered commands and keep
their ordering and missing-target behavior; zero, leading zeros and numbers
above nine are rejected.

## Last-terminal shortcut

For example, bind `last_terminal` to return to the previous live terminal across
panes and workspaces, or `last_prompt` to jump to the newest retained shell prompt:

```json
{"keybindings":[
  {"key":"alt-backspace","action":"last_terminal"},
  {"key":"alt-home","action":"last_prompt"}
]}
```

These actions use the same handlers as the menus and palette. Terminal-specific
navigation needs terminal focus; keep the default `terminal` context for those
bindings. Reordering preserves the existing live sessions.

## Configuration file

Optional startup configuration is read from `$XDG_CONFIG_HOME/rmux/config.json`
(or `~/.config/rmux/config.json`) on macOS/Linux, and
`%APPDATA%/rmux/config.json` on Windows. `RMUX_CONFIG` selects an explicit file.
No configuration file is created automatically. Example:

```json
{"font_size": 16, "confirm_close": "active"}
```

Open Settings File in the menu or palette uses your configured `file_editor`.
Without one, macOS uses TextEdit; other platforms use the system's default
application for JSON. If missing, this explicit action creates a default
file without overwriting an existing one. After editing, use Reload Settings.

## Scrollback budget

`scrollback_mib` sets the retained-history budget per new terminal (0–256,
default 4). Zero disables scrollback. Reloading changes the default for new
terminals without trimming existing sessions. This is an engine retention budget,
not a cap on total process/GPU memory.
Settings exposes this as New terminal history, adjustable in 1 MiB steps.
Changes affect terminals started afterward; Save these settings persists the
budget for future launches.
In Settings, Page Up/Down moves through controls by the visible panel height;
Home/End jumps to the first control or editor link. The selected control scrolls
into view, including Save and Open all settings in shorter windows.

## Padding and copy on select

`terminal_padding` sets the inset around terminal content in logical pixels
(0–32, default 8). Reload Settings applies it to existing terminals and resizes
their grids. Find/status bars stay outside this inset. Set it to 0 for the prior
edge-to-edge layout; match padding when comparing benchmark window sizes.

`copy_on_select` (default `false`) copies a completed local mouse selection to
the system clipboard on release, including word/line and rectangular selections.
Reload Settings applies it to existing terminals. Application mouse reporting
keeps its normal behavior; hold Shift to select locally in those applications.

## Option key and cursor behavior

Option+Left/Right on macOS moves backward/forward by word in shells using
standard Emacs/readline or zle bindings. rmux sends Meta-b/Meta-f for these keys;
applications that negotiate the Kitty keyboard protocol keep the original
modified arrow events. Custom shell bindings can change the resulting movement.

Cmd+Left/Right sends Ctrl+A/Ctrl+E for shell line-start/line-end navigation.
Cmd+Backspace sends Ctrl+U to clear the shell input according to its bindings.
These terminal-only defaults can be remapped or disabled through `keybindings`.
They do not run while composing IME text or editing a palette/search field.
The same actions are available in the command palette as **Move to Start of
Shell Line**, **Move to End of Shell Line**, and **Delete to Start of Shell Line**.
Use `move_to_line_start`, `move_to_line_end`, or `delete_to_line_start` to bind
them to another shortcut on any platform, without specifying control codes:

```json
{"key":"alt-home","action":"move_to_line_start","context":"terminal"}
```

On macOS, Settings → **Option key behavior** switches between **Terminal Alt**
and **Native text** immediately; use Save these settings to keep your choice.
In JSON, `macos_option_as_alt` defaults to `true`. Set it to `false` to leave
Option-modified text and dead keys to native text composition, useful for
accented characters. Control+Option and non-text navigation keys retain terminal
encoding. Reload applies immediately. This setting has no effect on Linux or
Windows; native text commits follow the existing UTF-8/IME path.

`unfocused_cursor` is `"hollow"` by default, keeping the insertion position visible
in inactive panes and windows. Set it to `"hidden"` to suppress inactive cursors.
`focused_cursor` defaults to `"application"`, retaining application-selected
shapes. Choose `"block"`, `"bar"`, or `"underline"` to override the focused
cursor shape. `cursor_opacity` accepts an integer percentage from 0 to 100
(default 100). Settings includes a Cursor opacity control in 5% steps. Zero
hides the cursor; partial opacity also softens the block cursor's inverted text.
The theme preview reflects this setting. Settings exposes these cursor controls, and Save these settings
persists them. Applications that hide their cursor remain hidden, and IME
composition uses a bar caret. Reload Settings applies these preferences to
existing windows; neither setting adds a blink timer.

## Terminal colors and palette

`foreground_color`, `background_color` and `cursor_color` accept `"#RRGGBB"`
strings or `null` for the selected terminal theme's defaults. For example:

```json
{"foreground_color":"#dcdfe6","background_color":"#12141a","cursor_color":"#8cb4ff"}
```

Reload Settings updates existing terminals. Application OSC color overrides
take precedence until the application resets them. These settings affect the
terminal's default colors; sidebar styling and the ANSI palette are unchanged.

`ansi_palette` separately accepts an array of exactly 16 `"#RRGGBB"` strings:
black, red, green, yellow, blue, magenta, cyan, white, followed by their eight
bright variants. Reload Settings recolors existing ANSI-indexed output. Extended
indices 16–255 and true-color output retain their colors. Application OSC palette
overrides take precedence; setting `ansi_palette` to `null` restores the selected
terminal theme's default palette.

## Theme presets

`terminal_theme` accepts `"dark"` (the existing default), `"light"` (white
background with a darker ANSI palette), `"solarized_dark"`, `"solarized_light"`, `"nord"`,
`"catppuccin_mocha"`, or `"catppuccin_latte"`.
The Solarized presets use [Ethan Schoonover's published palette](https://ethanschoonover.com/solarized/).
Nord uses the [official Nord terminal palette](https://github.com/nordtheme/alacritty/blob/main/src/nord.yaml).
Catppuccin Mocha and Latte use the [official Ghostty port](https://github.com/catppuccin/ghostty/tree/92b40c1137b647bdc21f999c098d19d3f59a0227),
including its ANSI, cursor and selection colors. Explicit JSON colors and imported
theme values take precedence over preset defaults. The split divider color from
that port is not applied.
Settings shows all seven presets as direct choices, highlighting the selected one;
A compact sample below the choices shows configured text, background, block-cursor
text and selection colors, plus ANSI swatches when a preset or custom palette
supplies them. Hover the imported filename to see its full path. It includes
your explicit color overrides; running applications can still override these colors.
Left/Right still cycles when the theme control is selected. The palette offers
Terminal Theme commands for direct keyboard selection. Use Save these settings
to persist a session choice. It reloads live; explicit color and
palette values override the preset. This changes terminal content only, not the
sidebar or other application controls, and does not automatically follow OS
appearance changes.

## Import terminal themes

Choose **Import Terminal Theme…** in the menu/palette, or **Import terminal theme →
Choose file…** in Settings, to select a local color file. The Settings row supports
keyboard navigation; custom action `import_theme` can also be bound to a shortcut.
Imports load in the background and apply for this session; **Save these settings**
persists the selected path. Canceling or selecting an invalid file leaves current
colors unchanged. Newer settings edits take precedence over a pending import.

Alternatively set `"theme_file": "/absolute/path/to/colors.theme"` and Reload
Settings. This supports a strict subset of
[Ghostty color assignments](https://ghostty.org/docs/config/reference):

```text
foreground = d8dee9
background = #2e3440
cursor-color = d8dee9
cursor-text = 2e3440
selection-background = 4c566a
selection-foreground = eceff4
```

Optional `palette = N=COLOR` entries override indexed colors 0–255. Indices
accept decimal or `0x`, `0o`, and `0b` prefixes (for example `palette = 0xf=red`). Omitted slots inherit the selected built-in preset; Dark uses the pinned
Ghostty engine defaults. Slots 16–255 inherit engine defaults unless imported.
Explicit JSON `ansi_palette` overrides slots 0–15 while retaining imported extended
colors. Application OSC overrides remain active across reloads until reset. A new import never inherits entries from the previous
import. Theme colors use the pinned Ghostty parser: X11 names (for example `red`),
3/6-digit hex with optional `#`, 9/12-digit hex with `#`, and `rgb:`/`rgbi:`
specifications. Color values are capped at 256 bytes. Blank lines and whole-line
`#` comments are allowed. Repeated supported assignments use the last value.

Includes and other settings
are rejected with a line-specific error rather than silently ignored. Files must
be regular UTF-8 files, not symlinks, and at most 64 KiB. This is not a full Ghostty
configuration importer. No files are watched or downloaded.

## Cursor and selection colors

`cursor_text` in rmux JSON accepts `"#RRGGBB"`, `"cell-foreground"`, or
`"cell-background"` and overrides imported `cursor-text`. Theme files accept the
same two dynamic references alongside literal theme colors. References resolve
against the actual cursor cell each frame, following application color changes.
Without either setting, the glyph uses its cell background color. Bar, underline
and hollow cursor shapes are unaffected by cursor-text settings.

Cursor fill also accepts `cell-foreground`/`cell-background` via `cursor_color`
in JSON or `cursor-color` in themes. Application OSC cursor colors take precedence;
resetting them restores the configured dynamic reference. Selection backgrounds
also accept `cell-foreground` and `cell-background`, resolving independently for
each selected cell. Selection foregrounds accept both references too, so a theme
can invert selection colors with `selection-background = cell-foreground` and
`selection-foreground = cell-background`.

`selection_background` in rmux JSON also accepts `"#RRGGBB"` and overrides the
imported selection background. `selection_opacity` accepts an integer percentage
from 0 to 100, or `null` for the theme default. Settings offers 5% steps and a
**Use theme default** reset; Save Settings persists the choice. The default keeps
configured selection backgrounds opaque and the built-in blue highlight
translucent. Opacity changes affect the highlight only, not the selected text. Text keeps
its terminal foreground colors unless `selection_foreground` (JSON) or
`selection-foreground` (theme file) is set. Explicit JSON values win. Foreground
overrides lazily cache one alternate text rendering per selected visible row;
normal and selected text are clipped to their respective regions. Disabling the
override clears those alternate renderings. Background-only changes do not reshape
text.

Precedence is explicit rmux color/palette settings, imported colors, then the
selected built-in preset. Settings shows the imported filename and effective
colors. Choosing a built-in preset clears the import for this session; Save these
settings persists that choice. Invalid reloads retain the previous valid settings.

## Font sizes and configuration checks

Font sizes must be 8–40. Reset Font Size uses this configured default; new tabs
and splits inherit the source terminal's size and family, and saved layouts
retain their saved fonts.
Optional `font_family` selects an installed monospace font for terminals without
a source to inherit from; the default remains Menlo/Consolas/DejaVu Sans Mono by platform. Missing
fonts use GPUI's fallback stack. Existing terminals follow a changed family default only when their current family
matches the previous default; saved layouts preserve each terminal's family. Older layouts without a family
use the current configured default.
Use Reload Settings in the menu or command palette to apply file changes.

It keeps current settings if validation fails. Size and family values matching
the previous defaults update live; values differing from those defaults remain
unchanged. Matching is value-based, including fonts restored from layouts. `RMUX_CONFIRM_CLOSE` overrides the file's
`confirm_close` setting. Unknown keys and invalid values fail startup.
Use Configured Font in the View menu or palette applies both the configured
family and size to the focused terminal while preserving its running session.
Run `rmux --check-config` to validate settings without opening a window or
starting a process. It returns 0 on success or 2 with a diagnostic on failure.

`rmux --dump-config` prints the effective JSON preferences after defaults, the
configuration file and the close-policy environment override are applied, then
exits. Its output can be reused as a configuration file. It excludes internal
source paths and separate launch variables such as `RMUX_SHELL`.
`rmux --check-layout PATH` validates a saved layout's version, structure and
resource limits without loading preferences, opening a window or starting shells.
It returns 0 for a valid structure and 2 on error. Directory availability and
shell executability are checked when restoring, not by this structural check.

## Close confirmations

Close warnings use `RMUX_CONFIRM_CLOSE=active` by default (reported working/blocked
tasks, live directly launched commands, and Unix foreground jobs whose process
group differs from the shell). Set it to `always` to confirm closing any terminal-containing
tab/workspace/window, or `never` to disable these confirmations. The Quit rmux
action shares the policy. Set the variable before launching rmux; invalid values
fail startup. OS-originated quit routes, Windows foreground-job detection and
background jobs or commands that replace the shell remain pending.

## Workspace group shortcuts

Group actions can be assigned custom shortcuts using `group_current_workspace`,
`edit_group_color`, `edit_group_icon`, `toggle_group_pin`, `ungroup_workspaces`,
`close_group`, `remove_workspace_from_group` and `set_group_anchor`.
`close_workspace` targets the current workspace and keeps its usual confirmation
policy. These action names do not add default key bindings.
