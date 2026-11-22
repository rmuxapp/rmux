# CLI and automation reference

[← User guide](../user-guide.md)

## On this page

- [Merge workspaces from scripts](#merge-workspaces-from-scripts)
- [Capture terminal output](#capture-terminal-output)
- [Pipe captured output](#pipe-captured-output)
- [Live stdio automation](#live-stdio-automation)
- [Unix socket automation](#unix-socket-automation)
- [Command-line client](#command-line-client)
- [Shell completion](#shell-completion)
- [Run rmuxctl](#run-rmuxctl)
- [Create splits](#create-splits)
- [Move and join panes](#move-and-join-panes)
- [Send keys and pasted text](#send-keys-and-pasted-text)
- [Send and list notifications](#send-and-list-notifications)
- [Replay events](#replay-events)
- [Create and close workspaces](#create-and-close-workspaces)
- [List windows, workspaces and groups](#list-windows-workspaces-and-groups)
- [Select workspaces and focus terminals](#select-workspaces-and-focus-terminals)
- [Send and read terminal text](#send-and-read-terminal-text)
- [Memory and output APIs](#memory-and-output-apis)
- [Move terminals and zoom panes](#move-terminals-and-zoom-panes)
- [Window and workspace metadata](#window-and-workspace-metadata)
- [Quit through RPC](#quit-through-rpc)
- [Mark a notification read](#mark-a-notification-read)
- [Resize panes through RPC](#resize-panes-through-rpc)
- [Equalize panes from scripts](#equalize-panes-from-scripts)
- [Swap panes through RPC](#swap-panes-through-rpc)

## Merge workspaces from scripts

CLI: `rmuxctl merge-workspace --workspace SOURCE_ID --target-workspace DEST_ID`
uses the same guarded transfer path. It selects the destination inside the
app; add `--activate` to activate the OS window. Raw RPC is `workspace.merge`
with `workspace_id`, `target_workspace_id` and optional `activate`. Both IDs are
required; self-targets, unknown fields and stale targets fail without transfer.
Cross-window merges also reject destinations that would exceed 256 terminals. This is an rmux extension; no corresponding merge command appears in
the pinned cmux CLI/RPC inventory.

## Capture terminal output

`rmuxctl capture-pane --surface SURFACE_ID` returns visible text in a JSON
response. Add `--scrollback` for retained history, or `--lines 20` for the last
20 logical screen lines (which implies scrollback). Counts must be 1–65536.
Soft-wrapped rows are joined and unused trailing screen rows are omitted.
Interior blank lines and line breaks produced by explicit spaces are preserved. Output is capped at 1 MiB: check
`result.truncated` before treating it as complete. For an oversized tail, returned
text is the prefix of that requested tail range. Capture preserves selection,
focus and scroll position; a busy parser returns a retryable error. Explicit
surface IDs are required; implicit targets and workspace/window aliases remain
pending. Raw `surface.read_text` also accepts `lines` with `scope: "history"`.

## Pipe captured output

`rmuxctl pipe-pane --surface SURFACE_ID --lines 20 --command 'wc -l'` captures
the last 20 logical history lines and sends them once to a local `/bin/sh`
command as stdin. Omit `--lines` to capture all retained history; supported counts
are 1–65536. Quote
the command as one argument; a trailing quoted command is also accepted. The
command runs in the CLI's working directory and inherits its stdout/stderr.
Unlike other rmuxctl commands, this returns command output and its exit status,
not a JSON envelope. Capture is capped at 1 MiB; truncation or RPC failure prevents
execution. Use `--selection` instead of `--lines` to send only the terminal
selection; add `--preserve-wraps` to retain its displayed soft-wrap boundaries.

These options use the same selection export as `read-screen`; they do not change
selection or focus. `--selection` and `--lines` cannot be combined.
The CLI waits for the command, with ordinary shell interruption for
long-running consumers. There is no future-output stream or app-side subprocess.
Windows shell execution, implicit workspace/window selectors and JSON-wrapped
command output are not implemented.

## Live stdio automation

`rmux --rpc-stdio-live` launches the normal UI with automation on its
inherited stdin/stdout. It adds `window.list` and `workspace.list`, both with no
parameters, for all windows in that process. Window results contain IDs, workspace
counts and selected workspace IDs; workspace results include owning window ID,
zero-based sidebar index, metadata, selection, pin state, and pane/terminal counts.
IDs are opaque strings valid for that process lifetime, not saved-layout IDs.
Selected means the selected sidebar workspace, not OS keyboard focus. Queries
read cached UI state without terminal locks, scrollback extraction or focus changes.
Pipe IO runs on one worker; the UI request queue holds at most eight requests and
each reply waits at most five seconds. Error `-32000` indicates an unavailable/full
queue; `-32001` means the request expired or was canceled before execution.

If execution started but its reply is unavailable, `-32002` reports an unknown
outcome; inspect current state before retrying. Closing stdin
or breaking the stream detaches automation and leaves the windows running.
This launch mode can combine with `--layout PATH`, or with directory and startup
command options. Inspection flags remain incompatible. Stdio alone opens no socket
and does not attach to an existing process. Additional targeting, subscriptions
and cmux compatibility remain pending.
The native GUI-to-pipe path passed a macOS smoke check covering restored layouts,
zoom and target selection across two windows. Visual focus, OS activation and
other-platform acceptance remain pending. Reproduce with
`mise run smoke-live-rpc` on Unix with a usable GUI session.

`mise run smoke-live-sessions` additionally checks running shell state through tab,
workspace and workspace-merge transfers, including closure of emptied windows.

## Unix socket automation

For opt-in Unix socket automation, launch with
`RMUX_SOCKET_DIR=/absolute/new-directory mise run run`. The parent must exist;
the endpoint directory must not. Connect to `rpc.sock` inside it using the same
newline-delimited JSON-RPC protocol and live methods. This also works alongside
`--rpc-stdio-live`; clients share the bounded UI dispatcher. The directory is
mode 0700 and socket 0600. Same-user clients can control the app. At most eight
connections are served, with ten-second read and five-second write timeouts;
idle clients should reconnect. No idle polling or network listener is added.
Normal quit removes the endpoint if its identity still matches. After a crash,
choose a new directory or manually inspect/remove the stale one. Startup failures
are logged and leave the GUI running without the socket. Windows named pipes and
caller-scoped permissions remain pending. `mise run smoke-socket` checks two live
clients, shared mutations, reconnection and normal-quit cleanup on macOS/Unix.

## Command-line client

Build the lightweight external client with `mise exec -- cargo build -p rmux-cli`.
`rmuxctl help COMMAND` or `rmuxctl COMMAND --help` shows command-specific usage
without a socket or a running app. A leading `--socket PATH` is also accepted for
help without opening that path. `rmuxctl --version` prints the client build version.
Help flags are recognized in these dedicated forms; text after `--` remains literal
input for commands such as `send`.

## Shell completion

`rmuxctl completions bash|zsh|fish` prints shell setup without requiring a running
app or `RMUX_SOCKET_DIR`. With `rmuxctl` on your PATH, enable it for the current shell:

```sh
# Bash
source <(rmuxctl completions bash)
# Zsh (after compinit in your shell setup)
source <(rmuxctl completions zsh)
# Fish
rmuxctl completions fish | source
```

The scripts complete supported friendly command names, command-specific flags,
and `--focus`/`--zoomed` boolean values. They do not contact rmux, enumerate live IDs, or
complete raw RPC JSON. Save the generated script in your shell's startup setup
if you want it enabled in later sessions. Bash and Zsh wrapper handling has been
checked locally; Fish runtime verification remains pending.

## Run rmuxctl

The macOS bundle also includes it at `Contents/MacOS/rmuxctl`, without a global
install or PATH change. It has no desktop renderer or terminal-engine dependency:

```sh
target/debug/rmuxctl --socket /absolute/new-directory/rpc.sock workspace.list
target/debug/rmuxctl --socket /absolute/new-directory/rpc.sock workspace.rename '{"workspace_id":"ID_FROM_LIST","name":"Build"}'
```

Alternatively, export `RMUX_SOCKET_DIR` and omit `--socket`. The client prints the
complete JSON-RPC reply; exit status is 0 for success, 1 for an RPC error and 2
for usage/transport failure. It never retries mutations automatically because a
lost reply can have an unknown outcome. Requests are capped at 64 KiB and replies
at 8 MiB. Alongside raw method calls, the client supports `list-windows`, `list-workspaces`,
`list-panes`, `list-panels`, `select-workspace --workspace ID`,
`rename-workspace --workspace ID NAME`, `focus-panel --panel ID`,
`send --surface ID [--paste] TEXT`, and `read-screen --surface ID` (optionally `--scrollback`
or `--selection`). Select/focus accept `--activate` to request OS activation;
otherwise they change the target selection without requesting OS activation.

Use `--` before positional text starting with a dash. All output remains JSON.
Lists cover the whole process by default. Narrow workspace/pane/panel lists with
`--window ID`; pane/panel lists also accept `--workspace ID`. Combined filters
intersect, and a well-formed missing or mismatched target produces an empty list.
Malformed IDs and unsupported filters are errors. Filtering does not focus or
create anything. Ref/index resolution, implicit caller targets and the rest of
the cmux CLI remain pending. Unsupported flags
are rejected rather than ignored.

## Create splits

Use `split-pane` to create a new terminal beside an explicit pane:

```sh
rmuxctl split-pane --pane PANE_ID --direction right --focus false
# Equivalent raw RPC:
rmuxctl pane.split '{"pane_id":"PANE_ID","direction":"right","focus":false}'
```

Add `--ssh DESTINATION` (`ssh_destination` in RPC) to create a separate SSH session
in the new pane. For example, `rmuxctl split-pane --pane PANE_ID --direction right --ssh production`.
Both local and SSH RPC splits run PTY startup and first-frame/color preparation in the background; completion
rechecks the target and limits. Color changes during startup are applied before insertion.
An unavailable executable leaves the source process and layout intact. A successful
reply means the SSH client was started, not that remote authentication succeeded.
Without this option, RPC splitting creates a local shell even beside an SSH tab.
The optional cwd is always a local launch directory, never a remote path.


Use `left` or `right` for a side-by-side split, and `up` or `down` for a stacked
split. The new pane appears on the requested side of the target. Optional `--cwd` (`cwd` in RPC)
must be absolute; otherwise the new terminal inherits the target pane's cached
working directory. Its font settings follow that pane too. The reply contains
`pane_id`, `surface_id`, and `workspace_id` for subsequent layout operations.
Focus stays unchanged by default; `focus:true` selects the new terminal and
`activate:true` additionally activates its window. Successful splitting exits
pane zoom. A failed shell start preserves the existing split tree and zoom.

RPC creation is limited to 64 panes per workspace and respects busy-window guards.
Interactive and RPC split creation, including Break Tab, stop at 32 nested splits, matching saved-layout
validation; shallower sibling panes remain splittable.

## Move and join panes

Live layout commands use explicit IDs from the list commands:

```sh
rmuxctl move-panel --panel SURFACE_ID --workspace DESTINATION_WORKSPACE_ID
rmuxctl move-workspace --workspace WORKSPACE_ID --window DESTINATION_WINDOW_ID
rmuxctl set-pane-zoom --pane PANE_ID --zoomed true
```

Moves reuse the existing transfer operations and preserve running terminal
processes. They select the destination; pass `--activate` to additionally request
OS window activation. Zoom accepts an explicit `true` or `false`; enabling it
selects that pane's terminal within its window without requesting OS activation.
Busy, missing and invalid destinations retain the RPC error behavior. These are
rmux command forms, not a claim of complete cmux CLI compatibility.

## Focus history (v0.1.4+)

Use `rmuxctl focus-back --window ID` or `rmuxctl focus-forward --window ID`.
Both support `--activate` and offline shell completion. Obtain the window ID
from `rmuxctl list-windows`; neither command assumes a currently focused window.

`window.focus_history` takes an explicit `window_id`, a `direction` of `back`
or `forward`, and optional `activate: true`. It navigates the same per-window
terminal history as the menu actions, skipping closed terminals. Example:

```sh
rmuxctl window.focus_history '{"window_id":"4294967297","direction":"back"}'
```

The result contains `window_id`, `moved`, and the destination `surface_id`.
At the history boundary, `moved` is false and `surface_id` is null; no focus or
OS activation changes. Busy windows return `-32003`, missing windows `-32004`,
and invalid IDs/directions/fields `-32602`. Activation is opt-in and only occurs
when navigation moves. This changes focus, not the terminal's shell history.

## Send keys and pasted text

`send-key --surface ID KEY` sends one symbolic press/release tap, for example
`Enter`, `Escape`, `Up`, `F5`, `Ctrl-c`, `Alt-Left` or `Shift-Tab`. The CLI also
accepts C-/M-/S- modifier prefixes, Return/Esc/BSpace aliases, and single Unicode
characters. It uses the terminal's negotiated keyboard encoding, not hard-coded
escape sequences, and preserves focus/viewport. Raw `surface.send_key` takes
`surface_id`, a lowercase named `key` (or a literal character), and optional
`control`, `alt`, `shift` booleans. A busy parser returns `-32007` before queueing;
other input failures return `-32006`. Multi-key sequences, held keys and --force
semantics remain pending.

`send --paste` sets `paste:true` on `surface.send_text`. It uses Ghostty's current
bracketed-paste mode, preserves viewport/focus, and queues the complete encoded
payload before later input. Both raw and paste automation text are capped at
16 KiB UTF-8. A busy parser returns `-32007` without queueing input; an exited
terminal, pending paste or unavailable input queue returns `-32006`. No implicit
newline is appended. Without `--paste`, input retains the existing raw behavior.

## Send and list notifications

`notify --surface ID --title TITLE --body BODY` posts into rmux's existing
notification panel and unread indicators without changing focus. Each surface
retains up to 32 notifications, newest first within each terminal.
`dismiss-notification --surface ID --revision NUMBER` removes one message using
the exact identifiers returned by `list-notifications`. It returns
`dismissed:false` for an unknown, evicted, or already dismissed revision; it does
not clear newer messages or switch focus. Revisions are unsigned 64-bit integers.

`list-notifications` lists non-dismissed messages
and accepts optional `--window`/`--workspace` filters. These are in-app messages;
native OS notification delivery is not implemented yet. The raw methods are
`notification.send`, `notification.list`, and `notification.dismiss`; dismissal
requires `surface_id` and the returned `revision`, and returns `dismissed:false`
if the message was dismissed, evicted by the history limit, or is unknown. Shell OSC notifications and RPC messages
share UI revisions while tracking shell updates independently. Titles/bodies are
bounded and control characters removed. The full cmux notify/reply/clear contract
is still pending. `surface.list` also includes its latest non-dismissed message
as `notification` (or null).

## Replay events

Read one page with the CLI:

```sh
rmuxctl read-events --limit 128
rmuxctl read-events --after 'CURSOR_FROM_PREVIOUS_RESULT' --wait-ms 3000
rmuxctl read-events --surface SURFACE_ID --kind surface.exited --wait-ms 3000
```

Commands print the JSON-RPC response, with the page under `result`. Keep the
returned `result.cursor` as a string; use it for subsequent calls. This command
does not run a permanent subscription or retry failed requests. Offline help
and Bash/Zsh/Fish completions include its options.

Live stdio and Unix socket clients can call `events.read`:

```json
{"jsonrpc":"2.0","id":1,"method":"events.read","params":{"limit":128}}
```

The result contains `events`, `cursor`, `has_more` and `gap`. Pass the returned
cursor as `params.after` on your next request, including after reconnecting to
the same running app. Treat cursors as opaque strings. A request without `after`
starts at the oldest retained record. Limits are 1–128 (default 128); when
`has_more` is true, read the next page before waiting for new events.

Filter with optional `surface_id` and `kinds` (an array of up to 16 distinct
supported event names). Both filters must match; an empty kinds array accepts
all kinds. The CLI uses `--surface ID` and repeatable `--kind KIND`.
Unknown kinds and malformed IDs are rejected. IDs may refer to closed surfaces,
since their retained events remain readable. Cursors advance past skipped
records, so keep filters consistent while following a stream; changing filters
does not replay previously skipped events. `has_more` means more retained records
remain to scan, which may not match the filter. Waiting readers wake only for
matching events; a timeout can advance the cursor past unrelated activity.

Set `wait_ms` to an integer from 0 to 3000 (default 0) to wait for a change when
caught up. Available records and gap responses return immediately. A timeout
returns an empty page with the same cursor; otherwise the new page contains the
events that arrived. At most 16 reads may wait concurrently; excess waits return
RPC error `-32000`. Each waiter holds one coalesced wake-up, not a separate copy
of every event. Expiry or completion releases its slot. Disconnects may retain
a slot until the bounded wait expires. Waiting does not block other clients;
use a separate socket connection for commands while waiting, since requests on
one connection are processed sequentially.

The app retains the latest 512 metadata events. `gap` is `null` normally,
`"retention"` if older events were evicted, or `"restart"` when the cursor belongs
to another app process. On a gap, refresh your resource lists; the returned page
starts at the oldest available event. Cursors ahead of the current process's
sequence are rejected. Events are not persisted across app launches.

Each event has a string `cursor`, `kind` and `surface_id`. Notification receive,
individual read and dismissal events also include `notification_id` as a decimal
string corresponding to the notification API's numeric `revision`.
Supported kinds are `notification.received`, `notification.read`,
`notification.all_read`, `notification.dismissed`, `notification.cleared` and
`surface.created`, `surface.closed` and `surface.exited`. Creation and closure
follow terminal ownership: moving a tab retains its surface ID and emits neither.
Closure is reported when ownership ends, even if rendering retains the old view;
process exit is a separate event and does not mean the tab has closed.
These are invalidation signals: use `notification.list` or
`surface.list` to fetch current state. Retained events can refer to surfaces or
notifications that have since closed or been removed. Titles, bodies and terminal
output are not copied into the journal.

Terminal metadata also emits `surface.title_changed`, `surface.directory_changed`,
`surface.status_changed` and `surface.progress_changed`. Shell title and directory
reports only emit when their values change; manual title edits use the same title
event. Local/remote directory changes in one delivered update produce one event.
Program status reports signal a new status revision; progress signals a changed
value. These events describe metadata delivered to the UI, not every intermediate
escape sequence in a burst of terminal output.

`surface.list` includes `program_status` (null or the current summary with `id`,
`count`, `state`, `kind`, `progress`, `app`, `title`, and `message`) and `progress`
(null or `state` and nullable `percent`). Program status states are `idle`,
`working`, `done`, `blocked`, and `error`; kinds are `none`, `permission`,
`question`, and `auth`. Progress states are `running`, `error`, `indeterminate`,
and `paused`. The status summary covers the currently selected record, not the
full program-status history. These reports come from terminal programs and are
not authenticated agent identities.

This is a replay API with bounded long polling. Push subscriptions, resource
workspace/pane lifecycle and movement events and the full cmux event contract are not implemented.
Readers use the existing bounded RPC queue; publishing never waits for clients.

## Create and close workspaces

`close-workspace --workspace ID` calls `workspace.close`. It removes the targeted
workspace and shuts down its owned terminals through the usual lifecycle; closing
the last workspace also closes its window. Closing a background workspace preserves
the current selection/focus. If the configured close policy requires confirmation,
the API returns `-32003` and leaves the workspace untouched; complete the close in
the UI. No force override is exposed. The result reports `closed` and
`window_closed`; process reaping finishes asynchronously.

`new-window [--activate]` creates a window. `new-workspace --window ID` creates
one terminal workspace in an existing window, optionally with `--name NAME`,
`--description TEXT`, `--cwd ABSOLUTE_PATH`, `--focus true|false`, and `--activate`.
Workspace creation defaults to keeping the current workspace selected;
`--activate` implies selection and requests OS activation. New workspaces are
ungrouped; group creation has its own RPC methods. Command/env/layout overrides
for creation remain pending. Raw `workspace.create` accepts the same fields
using `window_id`, `name`, `description`, `cwd`, `focus`, and `activate`.
Invalid cwd/shell startup leaves no empty workspace behind. Windows are limited
to 64 workspaces through this API; busy windows reject creation for retry later.

## List windows, workspaces and groups

`workspace.list` accepts an optional `window_id`. `pane.list` and `surface.list`
accept optional `window_id` and `workspace_id` filters; with no parameters they
list all panes and terminal tabs across that process. Both include parent window/workspace IDs;
surfaces also include pane ID, tab index, effective title, selected/attention flags
and a cached directory label (not a canonical path). Pane results include selected
surface ID, surface count and zoom state. Pane indices describe internal ordering,
not spatial position. `workspace.list` includes `layout`, a nested tree of
`pane` nodes with pane IDs and `split` nodes with `rows`/`columns`, child order and
relative weights, plus `zoomed_pane_id`. The tree retains hidden panes during zoom.

Use this tree for spatial layout and IDs for identity; tab order and indices may
change. Cached labels can be stale while terminal metadata updates are pending.
Surface results also contain `process_id` (the original PTY child PID, nullable)
and `process_exited` (whether reaping has completed). These avoid terminal locks;
a retained PID is not a liveness guarantee and may be reused after exit.

`window.list` includes `groups` with window-scoped handles such as
`workspace_group:17`, `window_id`, name, icon/color, pin/collapse flags,
`generated_anchor`, `anchor_workspace_id` and ordered `workspace_ids`.
Use the group handle together with its window ID; handles can repeat in another
window. `workspace.list` adds nullable `group_id` and boolean `group_anchor`.
Its `index` follows logical sidebar order, including collapsed members.
`workspace.group.update` requires `window_id`, `group_id` and at least one of
`name`, `collapsed`, `pinned`, absolute `cwd` or `clear_cwd: true`; it returns the
updated group record. `cwd` and `clear_cwd: true` cannot be combined. Clearing the
override restores inheritance from the anchor. Group records expose nullable `cwd`.
`workspace.group.ungroup` accepts only those two IDs and removes the container,
returning its `workspace_ids` without closing terminals. Both respect busy-state
guards and support notifications. `workspace.group.add`, `workspace.group.remove`
and `workspace.group.set_anchor` require `window_id`, `group_id`, `workspace_id`.

The workspace must belong to that window; remove/set-anchor also require current
membership. Add appends a member and is idempotent within the same group. The
response includes `group` (null if removing the final member dissolved it).
These methods preserve terminals and selection.
`workspace.group.create` requires `window_id` and `name`. Optional `workspace_ids`
must be a nonempty, unique list from that window; the first becomes the anchor.
Omitting it starts and selects a fresh anchor using the active workspace's cached
directory. Optional `external_id` is a persistent, window-scoped retry key:
repeating it returns the existing group without changing it or starting a shell.

Creating from existing workspaces preserves selection. Optional `cwd` supplies
an absolute directory for a fresh anchor and stores a group override for future
members. It can also be set when grouping existing workspaces; their shells are
unchanged. The override survives layout/session save and restore.
`workspace.group.ungroup` optionally accepts `remove_generated_anchor: true`.
This closes only a sole anchor explicitly created by rmux; it rejects groups with
other members and user-selected/promoted anchors. Close-policy confirmation
requirements return `-32003` so the user can close the workspace in the UI.
Successful cleanup returns `closed_workspace_id`; ordinary ungroup closes nothing.
`workspace.group.new_workspace` accepts `window_id`, `group_id` and optional
`placement` (`after_current`, `top`, `end`). It creates/selects a fresh workspace
using the anchor's cached directory and returns its `workspace_id`. The anchor
is the placement reference, so `after_current` inserts directly below it. Without
an override, the configured `group_new_workspace_placement` is used.

## Select workspaces and focus terminals

Live mode also supports `workspace.select` with an ID from `workspace.list`:
`{"jsonrpc":"2.0","id":2,"method":"workspace.select","params":{"workspace_id":"ID","activate":false}}`.
It uses sidebar selection behavior, including remembering the previous workspace
and restoring pane focus inside that window. `activate` defaults to false; true
also requests OS window activation. No index fallback is used for stale IDs.
Unknown workspaces return `-32004`; windows with active editors, palettes, layout
restoration or close prompts return `-32003` instead of interrupting that work.
Queued requests canceled at the deadline cannot execute later. Selection
notifications execute without a response; use a request ID to receive confirmation.

`surface.focus` accepts `{"surface_id":"ID","activate":false}` using an ID from
`surface.list`. It selects that terminal's workspace, pane and tab through the
existing terminal-navigation path. If the workspace is zoomed, zoom follows the
target pane. OS activation remains opt-in. The same deadline and busy-window
errors apply, including active tab renames and close prompts. A removed surface
returns `-32004` without selecting another tab. Success returns `surface_id`,
`workspace_id` and `selected: true`; visual focus/zoom acceptance is still pending.

## Send and read terminal text

`surface.send_text` takes `{"surface_id":"ID","text":"text"}` and queues the exact
UTF-8 bytes to that PTY. Text must be nonempty and at most 16 KiB after UTF-8
encoding. No newline or bracketed-paste envelope is added; include line endings
explicitly when wanted. Workspace/tab selection and the scrollback viewport are
preserved. `queued_bytes` confirms queue acceptance, not application execution or
completed delivery. An exited process or unavailable writer queue returns
`-32006`; invalid parameters return `-32602`. Existing target, busy and deadline
rules apply. The native live-shell test verifies Unicode delivery and retained
shell variable values after workspace transfer.

`surface.read_text` takes `{"surface_id":"ID","max_bytes":65536}`. It reads the
current viewport, including a scrolled-back viewport, from a fresh terminal
snapshot without changing selection or scroll position. Visual rows are separated
by newlines (including soft wraps), and row-end whitespace is trimmed. The optional
byte limit defaults to 64 KiB and ranges from 1 byte to 1 MiB; output ends at a valid
UTF-8 boundary with an explicit `truncated` flag. Responses include columns, rows,
scrollback offset and `scope: "viewport"` (the default).
If parsing owns the terminal lock, error `-32007` asks the caller to retry instead
of blocking the UI waiting for that lock. Snapshot extraction itself still has a
cost; sustained-read performance has not been benchmarked.


Use `scope: "selection"` with `surface.read_text` to read only selected text.
It uses the same bounded background export and UTF-8-safe truncation as history;
no selection returns empty text with `truncated: false`. Add `preserve_wraps: true`
to selection-scope requests, or use `read-screen --surface ID --selection
--preserve-wraps`, to retain terminal soft-wrap line breaks. The default joins
soft-wrapped lines. `preserve_wraps: true` is rejected for other scopes. It does
not change the clipboard, selection or viewport.

## Memory and output APIs

`surface.memory_usage` accepts `{"surface_id":"ID"}` and samples Ghostty screen
storage on demand in the background. It reports primary/alternate resident,
virtual, compressed and image bytes plus compression support. Its scope is
`ghostty_screen_storage`: these values exclude Rust snapshots, fonts, GPU assets
and child-process memory, and are not total application RSS. A busy parser returns
`-32007` for retry. No periodic sampler runs when the endpoint is unused.
The View menu and command palette also offer Show / Refresh Terminal Memory.
Opening or refreshing focuses the panel. Use ↑/↓, Home/End to select a row,
or Page Up/Page Down to move by the visible list height (using measured row sizes).

Copying a report preserves a manually scrolled position.
Use Enter to focus its terminal, R to refresh, S to choose sorting, C to copy the JSON
report, and Esc to close. Selection follows the terminal through sorting and
refreshes while it remains in the sample; Esc restores prior focus when the panel
still owns focus. Rows are virtualized for larger terminal lists.
Its panel samples up to 256 terminals in the current window and provides manual
Refresh/Close controls, including explicit busy or unavailable rows.
Rows initially sort by resident storage; click the sort control or press S to
search and choose screen storage, image storage, shell, foreground or shell-tree RSS/CPU. The
current mode is marked; Enter applies and Escape cancels. The command palette's
Cycle Terminal Memory Sort remains available. All modes sort descending with missing
samples last. Sorting reuses existing samples without another process scan.

The sort row shows when background sampling finished, in UTC. Copied reports
include `sample_completed_at_unix_ms` (milliseconds since the Unix epoch).
This timestamp describes the captured batch, not the time you copied it; the
previous snapshot and timestamp remain visible while a refresh runs.
Image storage sorts Ghostty's sampled image bytes; it does not represent all GPU
texture allocations or total application memory.
Click a row to focus its terminal. Copy exports the last sample as JSON in the
displayed order, with raw byte counts and explicit unavailable values.
The panel also samples each terminal's root shell RSS and CPU on the background
worker. CPU requires two sufficiently spaced refreshes with the same PID set;
the first sample is unavailable rather than zero. Exited processes are omitted.

On Unix, a distinct foreground process-group leader is also sampled and included
under `foreground_group_leader` in copied JSON. Results are discarded if that
terminal's foreground group changes before sampling completes. Windows does not
yet expose this information. This measures the leader alone; a pipeline or compiler
can have additional workers whose memory is not included.
The separate **Shell tree** row includes the shell and its descendants, including
background workers. JSON exports it under `shell_tree`, with sampled/expected
process counts and a `complete` flag. Sampling retains at most 4,096 process
metrics across the window. Oversized trees are unavailable; capped, exited or
changed members produce a partial total. Children arriving during the sampling
window also make the total partial: expected counts include membership observed
in either snapshot, and aggregate CPU is withheld until a stable refresh.

Parent/start-time snapshots before and
after sampling exclude changed ownership chains. Process discovery still reads
the host process table. Reparented helpers outside the shell tree are excluded.
Tree CPU is unavailable until all members have valid baselines. Summed RSS can
count shared pages more than once; it is not unique physical memory. Shell and
foreground figures overlap the tree: do not add them together. All process
figures are separate from Ghostty screen storage. Sampling is manual, with no
idle polling timer.
The panel's rmux summary reports application-process RSS and CPU across all
windows, also included under `application` in copied JSON. RSS is the operating
system's resident process measure, not a GPU allocation total. Screen storage is
already part of the application's memory: do not add it to application RSS.


Use `{"surface_id":"ID","scope":"history","max_bytes":65536}` to export
the active screen's retained history oldest-first, unwrapping soft-wrapped lines.
The same byte limits apply; responses contain surface_id, scope, text and
truncated. Export runs on a background executor, without changing selection,
focus or scroll position. It returns `-32007` when the engine is busy or the
terminal closes before export. Formatting still holds the engine lock while
reading, so large exports can contend with rendering and parsing; sustained
history-read latency is not yet benchmarked. Alternate-screen exports contain
only that screen, not the main screen's history.

## Move terminals and zoom panes

`surface.move_to_workspace` takes `{"surface_id":"ID","workspace_id":"ID","activate":false}`.
It moves the existing terminal into the destination workspace's remembered pane,
including across windows, and selects the transferred tab. Activation is optional
and defaults to false. Moving to the current workspace is a no-op. Stale IDs return
`-32004`; busy editing/restore/close states return `-32003`. Successful responses
contain the surface and destination workspace IDs. This shares the palette's
cross-window transfer path and preserves the PTY and custom tab label.

The rmux-specific `pane.zoom.set` method takes `{"pane_id":"ID","zoomed":true}`.
Setting true selects that pane's workspace and active terminal, then zooms it
without OS window activation. Setting false clears zoom only if that pane is
currently zoomed; it does not clear another pane's zoom or change selection.
Repeated requests are idempotent. Layout weights and terminal identities survive
zoom/unzoom. Missing IDs return `-32004`, malformed parameters `-32602`, and busy
windows `-32003`. Native state-transition checks pass; pixel-level zoom appearance
and latency remain unmeasured. This method is not a cmux wire-compatibility claim.

## Window and workspace metadata

`window.create` accepts an optional `{"activate":false}` object and returns the
new window's ID, selected workspace ID and initial workspace metadata. It uses
the normal default-shell window creation path. Activation defaults to false;
true requests window focus and app activation. A close prompt or shutdown returns
`-32003`; native window creation failure returns `-32005`. Each successful call
creates a new window, so inspect `window.list` after an unknown-outcome error
before retrying. Creation does not guarantee that the configured shell started
successfully; the window retains the normal terminal error display if it fails.

`workspace.rename` takes `{"workspace_id":"ID","name":"New name"}` and returns
the ID and effective name. It shares sidebar editor normalization: trim whitespace,
remove control characters, keep at most 128 Unicode characters, then trim again.
An empty result returns `-32602`. Renaming does not select the workspace or request
OS activation; an active workspace also updates its window title. Existing
busy-window, stale-ID and deadline rules apply. The native smoke verifies Unicode
normalization and renaming an inactive workspace without changing selection.

`workspace.move_to_window` takes `{"workspace_id":"ID","window_id":"DESTINATION","activate":false}`.
It moves the existing workspace and terminals, selects it in the destination,
and removes an empty source window. Names, metadata, split layout and terminal
IDs are retained. The destination ID must come from `window.list`. Activation
defaults off; repeated requests targeting the current owner are no-ops. Missing
targets return `-32004`; busy source/destination interactions return `-32003`.
The native smoke verifies layout/ID retention and empty-source cleanup.
`mise run smoke-live-sessions` additionally verifies five live fixture-shell PIDs
survive both transfers and exit during harness cleanup. Arbitrary running-shell
state, OS focus and performance still need dedicated acceptance.

## Quit through RPC

Live RPC `system.quit` requests the normal quit flow, including the configured
close confirmation. `{"requested":true}` confirms the request, not exit or save
completion; a notification can be used when no reply is needed.

## Mark a notification read

Use `rmuxctl read-notification --surface ID --revision NUMBER` with the identifiers
returned by `list-notifications`. It calls `notification.mark_read`, retains the
message in history and does not focus the source terminal. The response includes
`marked_read: true` when an unread message changed; missing or already-read
messages return `false`. An unknown surface remains an RPC error.

## Resize panes through RPC

Scripts can invoke the same directional resize operation with
`rmuxctl resize-pane --pane PANE_ID --direction left|right|up|down`, or JSON-RPC
`pane.resize` with `pane_id` and `direction`. The response contains `pane_id`,
`workspace_id` and `changed`; `changed: false` covers a single pane, missing axis
or a clamped divider. It does not change workspace selection or OS focus. Zoomed
layouts, active divider drags and pending split launches are rejected as busy.
Use `workspace.list` to read the updated split weights.

## Equalize panes from scripts

To balance every split in a workspace from a script, run
`rmuxctl equalize-panes --workspace WORKSPACE_ID` or call JSON-RPC
`workspace.equalize` with `workspace_id`. The response includes `workspace_id`
and `changed`. It normalizes each split's weights to one without moving terminals
or changing focus/selection; repeated calls and single-pane layouts return
`changed: false`. Zoomed layouts, active divider drags and pending split launches
are rejected as busy. Unzoom first to equalize through automation.

## Swap panes through RPC

`rmuxctl swap-panes --pane PANE_ID --other-pane OTHER_ID` exchanges two panes in
the same workspace. JSON-RPC uses `pane.swap` with `pane_id` and `other_pane_id`.
Tabs, terminal processes and active tabs stay attached to their pane; split axes
and weights stay attached to their positions. Workspace selection and focus are
unchanged. The result includes both pane IDs, `workspace_id` and `changed`; swapping
a pane with itself returns false. Different-workspace targets, zoomed layouts,
active divider drags and pending split launches are rejected. Explicit IDs also
allow swaps in hidden workspaces without relying on old screen coordinates.
