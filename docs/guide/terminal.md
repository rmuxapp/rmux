# Terminal, search and clipboard

[← User guide](../user-guide.md)

## On this page

- [Paste file paths](#paste-file-paths)
- [Paste images](#paste-images)
- [Drop files](#drop-files)
- [Hidden-input indicator](#hidden-input-indicator)
- [Copy mode and named buffers](#copy-mode-and-named-buffers)
- [Find history](#find-history)
- [Terminal context menu](#terminal-context-menu)
- [Links and clipboard input](#links-and-clipboard-input)
- [Paste limits](#paste-limits)
- [Prompt navigation and search](#prompt-navigation-and-search)
- [Scrollback and selection](#scrollback-and-selection)
- [Working-directory actions](#working-directory-actions)
- [Scrollback navigation](#scrollback-navigation)
- [Mouse selection and links](#mouse-selection-and-links)
- [Find text in a terminal](#find-text-in-a-terminal)
- [Search across terminals](#search-across-terminals)
- [Mouse reporting](#mouse-reporting)
- [Editing text fields](#editing-text-fields)
- [Exited terminals and restarting](#exited-terminals-and-restarting)

## Paste file paths

On macOS, **Copy** files in Finder and **Paste** into a focused terminal to insert
quoted absolute paths. Finder's file-URL metadata takes precedence over filename
text or image representations. This reads paths only, without opening files or
uploading them, and uses the same 256-file/64 KiB limits as file drops. Ordinary
text and image paste remain available when file metadata is absent. Linux
copied-file clipboard formats and native Finder paste acceptance remain pending.

## Paste images

On macOS/Linux, **Paste** also accepts clipboard images when no text representation
is present. rmux writes the original image bytes to a private temporary file in
the background, then pastes its quoted local path without Enter. Files remain
available until their terminal closes, including when the terminal moves between
windows. Limits are 16 MiB per image and 32 files / 64 MiB across the app. Closing
the owning terminal removes its files and frees the quota. If the terminal is no longer focused when
preparation finishes, delivery is canceled; a newer text paste or file drop cancels a
pending image paste. The **Preparing image paste…** badge exposes a cancel button;
**Cancel Queued Paste** also cancels preparation, and typing cancels it before
sending the new input. Automation mutations report busy during preparation.

Canceling delivery does not interrupt an in-flight disk write; its unused file
is removed when that bounded write finishes. This does not upload to remote hosts. Image files live under `~/Library/Caches/rmux/pasted-images` on macOS, or
`$XDG_CACHE_HOME/rmux/pasted-images` (default `~/.cache`) on Linux. A later image
paste scans at most 512 entries and removes unlocked rmux image files left after
a crash; active images stay locked. No idle cleanup timer runs. Older images in
the system temporary directory are not swept. Native clipboard acceptance and
forced-termination recovery testing remain pending.

## Drop files

On macOS/Linux, drop files into a **focused terminal** to insert their absolute
paths using POSIX shell quoting. A space follows each path; no Enter is sent.
Close terminal overlays first. Drops support at most 256 paths and 64 KiB of
quoted text; invalid UTF-8 or control characters are rejected. This inserts local
path text only—it does not upload files, read images or detect a remote shell.
Windows file-drop quoting is not implemented yet.

## Hidden-input indicator

A focused local macOS/Linux terminal shows **Input hidden** when canonical line
input has echo disabled. Raw-mode TUIs do not trigger this badge. It samples on
output, focus and input events, so silent mode changes may remain stale until
the next event. This is a local echo-state indication, not a guarantee that every
password prompt is recognized; remote/raw-mode prompts and Windows ConPTY
detection remain unsupported. No input content is inspected for this indicator.

## Copy mode and named buffers

**Enter Copy Mode** (palette/menu, custom action `copy_mode`) opens a stable
history snapshot while the terminal keeps running. Use ↑/↓ or j/k to move,
Home/End to reach either end, and Page Up/Down to move approximately one visible page. Hold Shift while
moving, or press Space/v to anchor a range. Click selects a line; Shift-click
extends. Enter/y or the platform Copy shortcut copies selected logical lines;
Select All selects the snapshot. A copy status reports the byte count; empty
selections leave the clipboard unchanged. Large selections are prepared in the
background; a newer copy elsewhere takes precedence over a pending copy. Closing
copy mode cancels pending clipboard delivery. The header shows the current line.

Escape closes it. Paste is disabled in this
view. The snapshot includes at most the first 8 MiB and 65,536 lines; truncation
is labeled. Long displayed lines are clipped at 4,096 bytes, but copying includes
their retained full text. Press / or Command-F (Control-F on Linux/Windows) to
search the snapshot. Enter submits a case-sensitive literal query; Shift-Enter
searches backward. n and Shift-N jump between matching lines with wraparound.
Search works on the full retained text, including clipped portions of long lines.
Escape in the search field returns to navigation. Press c to toggle character
selection; ←/→ or h/l also enter character mode. Shift-movement or Space/v anchors
a character range; Enter copies it. Navigation and copy preserve complete Unicode
graphemes, including combined emoji and accents. Vertical movement retains the
desired terminal-cell column across wide characters and short lines. Press c to return to line mode;
clicking a row or Select All also returns to line mode. Long-line display follows
the character cursor without shaping the entire line. Press r for rectangle mode,
then anchor with Space or Shift-movement and move across rows/columns. Rectangle
copy uses terminal-cell columns, includes whole graphemes touched by either edge,
and joins rows with newlines without padding short lines. r returns to character
mode; c returns to line mode. Named buffers can be managed through the CLI/API:

```sh
rmuxctl set-buffer --name notes 'text to retain'
printf 'first line\nsecond line\n' | rmuxctl set-buffer --name notes -
rmuxctl list-buffers
rmuxctl paste-buffer --surface SURFACE_ID --name notes --bracketed
rmuxctl buffer.get '{"name":"notes"}'
rmuxctl buffer.delete '{"name":"notes"}'
```

Omitting a name uses `default`. Omitted text or `-` reads UTF-8 stdin exactly,
including trailing newlines; an explicit empty string stores empty text. Stdin
reads are bounded, and JSON serialization must fit the request limit. Buffers are shared by windows in one rmux process,
independent of the clipboard, and disappear when the app exits. Limits are 32
buffers, 1 MiB per buffer and 8 MiB total text; over-limit writes fail without
eviction. The existing 64 KiB JSON request limit further constrains CLI writes
(including JSON escaping). Lists expose names and byte counts, not contents.
`buffer.get` and `buffer.delete` are rmux RPC extensions. `paste-buffer` requires an explicit surface ID and retains the buffer after
queueing. It preserves terminal focus and scroll position. Raw delivery is capped
at 64 KiB; `--bracketed` uses the terminal's paste encoding (including bracketed
markers when requested by the application), up to 1 MiB. Busy/exited terminals
and full queues report errors without automatic retry. RPC equivalent:
`buffer.paste` with `surface_id`, optional `name`, and optional `bracketed` boolean.

In copy mode, click **Save Buffer…** or press b. Enter a name and press Enter to
save or replace it; Escape cancels the name field. Line, character and rectangle
selections are supported, up to the 1 MiB buffer limit. Formatting happens in the
background without touching the clipboard. If any buffer changes during the
save, rmux reports a conflict and asks you to retry; it does not overwrite newer
state. Closing copy mode cancels pending save delivery.

After returning to a terminal, use **Paste Named Buffer…** in the command palette
or native menu (custom action `paste_buffer`). Search by name or preview, select with arrows,
and press Enter to queue terminal-aware paste into the terminal that opened the
picker. Rows show names, byte counts and up to 96 characters of content, with
whitespace/control characters displayed as spaces. Longer previews end in an
ellipsis; search covers this preview, not the full saved content. A buffer
change while the picker is open requires reopening it; a closed or moved target
is never replaced with an unrelated terminal. Paste preserves the viewport and
does not consume the saved buffer. **Delete Named Buffer…** in the palette/menu
(custom action `delete_buffer`) searches the same names and previews; Enter deletes
the chosen buffer and releases its storage. If buffers changed while the picker
was open, reopen it before deleting.


**Copy Named Buffer…** in the palette/menu (custom action `copy_buffer`) uses the
same searchable picker to copy the complete saved text to the OS clipboard.
It retains the buffer and sends no terminal input. If buffers change while the
picker is open, copying is refused and the clipboard stays unchanged.

Use **Rename Named Buffer…** in the palette/menu (custom action `rename_buffer`)
to choose a buffer, edit its preselected name, and press Enter or click Save.
Escape or Cancel closes without changing the name. Invalid names and collisions leave the editor open for correction;
concurrent buffer changes require reopening the picker.
You can also rename with `rmuxctl rename-buffer --name OLD NEW_NAME` (source defaults
to `default`), or RPC `buffer.rename` with `name` and required `new_name`.
Renaming moves the retained text without cloning it and preserves its byte count.
An existing destination is rejected; renaming to the same name is a no-op.

Successful renames invalidate open buffer pickers, which must be reopened.

## Find history

In terminal Find, **Alt+Up** recalls older searches and **Alt+Down** moves toward
newer searches, then restores your draft. Enter (or Shift+Enter), the match-arrow
buttons and Find navigation menu/shortcut actions remember valid queries while
navigating matches; closing Find also remembers its current query.
History is local to each terminal and kept only in memory: up to eight distinct
queries and 16 KiB of retained query text, with a 4 KiB limit per query. Editing a
recalled query starts a new draft. History navigation leaves active IME composition
alone, and does not replace an oversized draft that cannot be searched.

## Terminal context menu

Right-click terminal content for Copy, Paste, Find, selection, directory actions
and Clear Scrollback. Use Shift-right-click when a terminal app handles mouse
input. The palette’s **Terminal Actions** command opens the same menu; custom
shortcuts can bind `terminal_menu`. Arrow keys navigate, Enter activates, and
Escape returns focus to the terminal. Terminal, tab, workspace and group context menus show the current shortcut
beside each bound action, including custom key sequences. In short windows,
menus scroll and keyboard navigation brings the selected action into view.

## Links and clipboard input

Command-click (Control-click on Linux/Windows) opens terminal HTTP(S) links in
your browser and local `file://` links in your configured external editor.
Hold that modifier while hovering to see the link target in a tooltip and a pointer
cursor for supported links. Unsupported OSC 8 targets are labeled and do
not gain the pointer cursor. Hover lookup is cached per cell/revision and skips a
busy parser; move the pointer again to retry. Output changes clear old targets.
Link previews fit the window width and truncate long addresses with an ellipsis.
The terminal context menu also offers Copy Link Address and Open Link for the
link under the menu's opening position. It captures that target when opened, so
later terminal output cannot change the chosen address. Open Link is enabled
only for supported targets; copying preserves the original address.

Local file links work as explicit OSC 8 targets or plain-text `file:///…` URLs.
Configure `file_editor` as described in the [file guide](files.md). Paths are
percent-decoded and passed as a literal editor argument. The editor launcher
requires an existing regular file; directories and symlinks report an error.
Remote URL hosts, recognized remote terminals, network-share forms, queries and
fragments are not opened as local files. Path-shaped text and `path:line:column` references are also detected, including
quoted paths with spaces and local `~/` shorthand. See the [file guide](files.md)
for details. The browser demo cannot launch a local editor.

If a terminal program owns right-click, use Shift-right-click for the local menu.
If output parsing is busy, rmux reports a retry message instead of waiting; click
again after output pressure eases.

If Paste reports that the terminal is busy, no paste bytes were queued; retry
the action after output pressure eases. A successful retry clears the message.
Clipboard pastes retain the 1 MiB input limit and ordered bracketed delivery.
Accepted clipboard paste text is encoded on the PTY writer thread, keeping
subsequent keystrokes behind it. Asynchronous encoding/write failures appear as
terminal errors; accepted input is not a guarantee that the child consumed it.
A compact Paste pending overlay stays visible until the writer releases the
paste slot; its close control requests cancellation.
Cancel Queued Paste in the Edit menu/palette (custom action `cancel_queued_paste`)
discards paste payloads when cancellation is observed before writing starts.

It does not recall bytes already being written or interrupt their closing
bracketed-paste delimiter. The pending slot is released when the writer reaches
and discards the canceled message; ordinary queued keystrokes remain intact.

## Paste limits

Clipboard text pastes accept up to 1 MiB of UTF-8 per operation and preserve
bracketed-paste boundaries. Each terminal accepts one pending paste at a time;
another attempt reports an error until delivery finishes. Ordinary input queued
after a paste follows its closing marker. A full or closed input queue reports
an error instead of accepting a partial paste.

## Prompt navigation and search

View → Previous Prompt / Next Prompt navigate shell-reported prompt boundaries.
Use Cmd+Up/Down on macOS or Ctrl+Shift+Up/Down on Linux/Windows, or the command
palette. This requires semantic shell markers such as OSC 133; unmarked output
does not move the viewport. New zsh sessions supply these markers automatically.

Find Older/Newer Match reopen a closed search using its remembered query and
select a result on the first invocation. Explicit Find still prefers selected
text when prefilling a new search. Find Older/Newer Match are available in Edit
and the palette. Use Cmd+G /
Cmd+Shift+G on macOS, or F3 / Shift+F3 on Linux/Windows, with the terminal or find
field focused. If Find is closed, these commands open it. The find bar also has
28-pixel hover controls with labels for navigation and closing.

## Scrollback and selection

Unfocused terminals compress eligible scrollback after a quiet delay, using
bounded background steps. History restores transparently when accessed. Focusing
a terminal or opening Find cancels scheduling; completed passes stop until later
activity. Savings depend on content and platform support.

Edit → Clear Scrollback (also in the palette) removes retained history while
preserving the current screen and running shell. It clears selection and closes
Find; new output continues to accumulate within the normal history limit.

Edit → Select All selects the active terminal screen and its scrollback. Use
Cmd+A on macOS or Ctrl+Shift+A on Linux/Windows, then Copy. Alternate-screen
applications select only their own screen. In text fields, Select All selects
the field text instead.

## Working-directory actions

Copy Working Directory in Edit or the palette copies the focused terminal's
reported local directory as plain text, without shell quoting. It requires shell
directory reporting; remote or unavailable directory metadata leaves the clipboard
unchanged and shows a message.
Reveal Working Directory uses the same local-only metadata to reveal the path
in the system file manager (Finder on macOS). Neither action runs a shell command.
Linux/Windows use Ctrl+Shift+T and Ctrl+PageUp/PageDown. Click tab labels to
select them, × to close, or + to create. Ctrl+Shift+PageUp/PageDown moves the
active tab left/right. Tabs retain their processes and scrollback while hidden.

Drag a tab onto another tab to move it to that position within the pane. Drag a
workspace row onto another row to reorder within its window. Drop a tab onto a
tab in another pane of the same workspace to move it there. The existing session
is retained and becomes active; an empty source pane collapses. Moving between
workspaces or native windows is not implemented yet.
Shell-provided OSC 0/2 titles replace the default tab labels when available.
New tabs and splits inherit the focused shell's directory when it reports a local
OSC 7 file URI. Without a usable report they use the normal shell startup directory.

Remote-host directory reports are not interpreted as local paths. No shell startup
files are modified; automatic reporting depends on your shell configuration.
Workspace rows show the focused pane's reported local directory beneath its name.

## Scrollback navigation

Use the wheel/trackpad or Shift+PageUp/PageDown to browse scrollback; Shift+Home/End goes to the oldest/live output. Typing or pasting returns to the live viewport.

Last Prompt in the menu or command palette jumps to the newest retained shell
prompt, including one above a long command's output. It uses shell semantic
markers; without them it returns to live output.

## Mouse selection and links

Drag to select terminal text, double-click a word, or triple-click a line.
Alt-drag selects a rectangular block. Cmd+C copies the selection on macOS;
Ctrl+Shift+C copies on Linux/Windows. Hold Shift to select locally when a terminal
application has enabled mouse reporting. Copy uses Ghostty's text extraction,
including Unicode, joining soft-wrapped lines. Choose **Copy with Line Breaks**
from the Edit menu or command palette to retain those visual line boundaries.
It can be bound using `copy_with_line_breaks` in the `terminal` context; no default
shortcut is assigned. Both modes preserve explicit newlines and reject selections
above 16 MiB instead of copying a partial result. Drag above/below the terminal to extend
selection through scrollback; scrolling stops on release or at the history edge.


Cmd-click an OSC 8 hyperlink or plain-text HTTP(S) URL to open it in the default
browser (Ctrl-click on Linux/Windows). The lookup follows the displayed scrollback
viewport and runs only on modifier-click or modifier-hover; ordinary rendering does not copy link
URLs. Plain URLs can span soft-wrapped rows, but must be fully visible and fit
within an 8 KiB logical line. Surrounding quotes and trailing prose punctuation
are excluded. Explicit OSC 8 targets take precedence. Hold the link modifier while hovering to preview the target. Native hover
acceptance and local/remote file-path routing are still pending.

## Find text in a terminal

Cmd+F opens terminal search (Ctrl+Shift+F on Linux/Windows). A single-line selection
of up to 4 KiB prefills a newly opened search, selected for easy replacement;
otherwise Find restores that terminal's last query (empty for a new terminal).
The query survives closing Find, clearing scrollback and moving the terminal to
another window; it is not saved in layout files. Type a query, then
Enter to visit older matches or Shift+Enter for newer ones; the arrow buttons do
the same. Escape or × closes the find bar. Search includes scrollback, updates
with output, and runs in a background worker. Matching ignores ASCII letter case;
regex and case-sensitive modes are not yet available.

Invoke Find again while it is open to focus and select the query for replacement.
Active IME composition is preserved rather than selecting over marked text.

## Search across terminals

**Edit → Search Terminal Output…** or the command palette searches across terminals
in all rmux windows. The default shortcut is **Cmd+Shift+F** on macOS and
**Ctrl+Alt+Shift+F** on Linux/Windows; terminal Find retains **Ctrl+Shift+F** there.
Customize it with `search_terminal_output` in key bindings. Enter a literal
query and submit; typing does not scan terminals. Reopening the editor selects
the last submitted query from that window, ready to replace or rerun with Enter.
This query stays in memory only and is forgotten when the window closes.
Matching follows Find's ASCII
case-insensitive rules. A 64 KiB prefix supplies previews; a capped prefix without
a match falls back to incremental search of the terminal's retained history.

Results outside the preview identify the terminal without copying its full history.
Preview excerpts start up to 32 characters before the literal match and show at
most 1,024 characters, with ellipses marking omitted text. Literal matches in
these excerpts are highlighted; deep-history status messages are not excerpts.
Select a preview result and click **Copy excerpt** to copy its captured text,
including any clipping ellipses, while keeping the results open. This does not
copy the full original line or reread live terminal output.
You can also press **Cmd+Shift+C** on macOS or **Ctrl+Shift+C** on Linux/Windows
while the result palette is focused. Normal Copy still copies selected query text.

All selected terminals' previews are checked before deeper fallback scans begin;
the completed batch retains terminal order rather than completion order.
One worker scans at most 256 terminals, with the originating window first.
Each fallback has a 250 ms work budget within a five-second batch budget, checked
between incremental operations. Engine cleanup still serializes parser access,
so these are work budgets rather than hard wall-clock guarantees.
The result summary reports incomplete scans, unavailable terminals and terminals
beyond the limit, so no matches
does not necessarily mean the query is absent from all output.
Select a result to activate its window, focus that terminal and open Find with the same query, then
use Find navigation to move between live matches. Results contain one excerpt
per matching terminal, not every occurrence. Output may have changed since the
snapshot. Search again refreshes the result set; typing in results only filters
that snapshot. The loading notice shows preview/match counts and remaining deep
scans, and offers cancellation. This currently excludes
external agent-session archives; full global-search parity remains in progress.

## Mouse reporting

When a terminal application enables mouse reporting, clicks, movement and vertical wheel events are sent through Ghostty's mouse encoder. Hold Shift while scrolling to browse local history instead.

## Editing text fields

In search, rename and settings text fields on macOS, Command-Backspace deletes
to the start and Command-Forward Delete deletes to the end. With text selected,
these keys delete the selection. Command-Z undoes the edit. Option-Backspace
continues to delete by word; on Linux and Windows, use Control-Backspace for
word deletion.

On macOS, Control-A/E moves to the start/end, Control-B/F moves one character,
Control-H/D deletes backward/forward, and Control-K deletes to the end (or the
selection when selected). Shift extends movement selections; Command-Z undoes
deletion. These shortcuts also work in the WASM demo's shared text fields on
macOS. They do not change terminal input bindings or run during IME composition.

## Exited terminals and restarting

When a terminal process exits, its output stays available and a status strip shows
the exit message. **New shell** opens a fresh tab through the normal new-tab path,
using its directory/font inheritance; it does not replay the exited command.
The adjacent close button closes the finished terminal through the normal close
policy. Input, rendering and other operational errors retain their separate error
messages.

Finished terminals carry a **Finished** label in the tab strip. Hover the title
for the exit message, or search terminals in the palette to see and search their
finished status across windows. Custom tab names remain intact.
Live task and progress badges disappear when the terminal finishes. Program
Status retains the last reports for inspection and copying, labeled **Last
reported status** so they are not mistaken for running tasks. Tab/Shift-Tab or
the arrow keys navigate those reports; C copies the selected report.
