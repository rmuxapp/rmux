Combined native build at c7a4898 passed. A separately signed compact fixture
launched with temporary configuration and synthetic terminals; installed rmux
and its sessions were untouched. UI automation could inspect the window/menu
but keyboard input and selecting Command Palette produced no visible change,
so native palette/settings/find interaction acceptance remains open. The test
process exited through the fixture's normal RPC cleanup after interruption.
The static native view exposed duplicated notification previews in an expanded
group and its child row. Group headers now aggregate member previews only when
collapsed; expanded headers retain their anchor's own preview while children
show theirs. This final presentation-only change awaits the next combined build.

Find feedback polish: completed searches with zero matches now use the shared
error-colored query border/status and disable match navigation in both hosts.
Native and WASM navigation handlers reject completed no-match requests; shared
pointer controls now honor their disabled state. Editing a query clears the
no-match state immediately, preserving queued navigation during pending scans.
Native compile checking and the WASM build passed. Browser review verified the
no-match state, recovery to a matching query, and Escape returning terminal input
where `echo find-focus-restored` ran immediately. Screenshot:
`target/find-no-matches.png`. Native pointer acceptance remains pending.

Palette search quality: native and WASM palettes now keep command text separate
from shortcut aliases. Titles retain fuzzy matching, while binding aliases match
whole tokens; fuzzy words cannot span or form inside repeated shortcut synonyms.
This fixes the observed `group` query incorrectly returning Move Tab Left.
Exact-token queries rescan candidates so typing `c` then `ctrl` can introduce a
binding result; title-only palettes retain incremental filtering. Eleven focused
core search tests, native compile checking and one WASM build passed. Browser
review verified `group` now returns only group commands and Keyboard Shortcuts
still resolves `ctrl tab` to Next/Previous Pane. Screenshot:
`target/palette-search-matching.png`. No release or deployment.

Contextual palette commands: both native and WASM hosts share group-action
availability rules. Group-only actions require a grouped origin workspace;
Group Selected Workspaces requires multiple sidebar selections, and move-to-group
results omit the current group. Native cancellation entries now also require an
active file-loading or restore job. These are palette-discovery rules; existing
action handlers still revalidate their targets. Native compilation and the WASM
build passed. Browser review created a group from the palette, verified group
actions appeared, then ungrouped and verified they disappeared on reopening.
Screenshot: `target/palette-group-context.png`. Native job-cancellation interaction
and multiselect acceptance remain unverified. Nothing published.

Shared palette search polish: native palettes, the tab picker and WASM command
palette now show an inline clear control for nonempty search text. Clearing uses
the text editor's undo path, restores input focus and updates results through
the normal change event. It is hidden while composing text. The local WASM build
passed; browser interaction verified clearing restores the full command list
without closing the palette and Cmd+Z restores the query and filtered results.
Screenshot: `target/palette-clear-search.png`. Native compile checking passed;
native pointer/IME acceptance remains pending. No release or deployment.

Native tab-picker lifecycle polish: asynchronous terminal creation, keyboard
reordering, tab transfers out, and single/batch closing now reconcile an open
picker immediately. Previously only metadata updates refreshed its list state,
so structural changes could leave stale row counts or selection until a later
title update. Existing selection/scroll reconciliation preserves a surviving
identity and chooses a nearby result when it disappears. The six focused tab
order/list tests passed; these cover the reconciliation helpers and close-scope
rules, not native interaction or asynchronous launch acceptance. No app bundle,
release, or deployment was produced.

Combined WASM review at 8511c46: one local demo build passed with the accumulated
shared UI changes. In-browser review verified Cmd+P opens the palette, keyboard
shortcut discovery and `ctrl tab` filtering, Escape restores terminal typing,
Cmd+1/2 switches isolated workspace output, Cmd+T adds a tab, Cmd+Shift+[ returns
to the original tab with its output, and Cmd+D splits. Clicking the palette Close
control returned input to the originating split, where an echo ran immediately.
No stale workspace text or long sidebar caret appeared in this sequence.
Screenshots: `target/wasm-polish-combined.png`, `target/wasm-focus-restored.png`.
This is local sample-session acceptance, not native OS behavior or full parity.
The temporary local server was stopped; nothing was deployed.

CLI failure-path polish: connection errors identify the endpoint, explain
missing/refused/permission-denied cases and explicitly state no request was sent.
Write/read failures and malformed/incomplete/wrong-ID replies consistently state
that the result may be unknown, recommend inspecting app state and confirm no
retry. JSON stdout and exit codes are unchanged. All 34 lightweight CLI tests
passed, including a real local socket that consumes a workspace.open request
then returns a mismatched reply ID, plus a missing-endpoint case. No desktop
bundle or release was built.

Imported-theme recovery acceptance: a focused test now exercises switching from
an imported theme to a bundled theme, breaking and then deleting the source file,
and reverting from the in-memory baseline. Background, palette and selection
colors restore without file I/O; a newer explicit foreground override survives.
The unsaved marker clears after restoration. The test passed; no production
behavior change, app bundle rebuild or release was needed. The guide distinguishes
session restoration from repairing an absent theme file for future reloads.

Socket notification acceptance follow-up: the prior unread assertion depended
on OS window focus after pane zoom/unzoom. Focused-terminal notifications are
intentionally stored as read. The fixture now focuses a different pane and
observes the target's input_focused=false before checking unread delivery.
One full config-socket smoke then passed without rebuilding: CLI discovery/error
exits, private endpoint, two clients, shared mutations, cross-window filtering,
nested splits/failure atomicity, notification history/acknowledgement/OSC,
workspace lifecycle, child cleanup, reconnection and normal endpoint cleanup.
This resolves the previously reported smoke failure as a fixture precondition;
no notification product semantics changed. OS activation/UI review is separate.

Editor automation startup: optional absolute `socket_directory` in Settings
JSON now enables the existing private Unix endpoint on normal app launch.
RMUX_SOCKET_DIR wins when present; omission keeps automation opt-in. Reload
reports that endpoint changes require restart, and attach failures are visible
in normal app windows as well as stderr. Configuration read/save never creates
an endpoint. The guide includes a process-based VS Code task for named workspace
reuse with literal project paths; CLI socket selection remains explicit.
One desktop/client build, the focused validation/persistence test, and an
isolated config-only endpoint smoke passed (CLI discovery, window lookup, normal
quit cleanup, app launched without RMUX_SOCKET_DIR). The broader socket smoke
reached notification checks but failed because retained notifications were read
rather than unread; that focus-dependent assertion is unresolved and the suite
is not reported as passing. Startup-error UI and VS Code task invocation remain
unverified. No user configuration was changed and nothing was released.

Tab-picker live-update polish: when the selected terminal disappears or no
longer matches after metadata changes, replacement selection stays at the
nearest surviving result row rather than jumping to the first. Enter no longer
falls back to an unrelated first result if its selected identity is stale.
The desktop test binary compiled and six existing tab-order/picker tests passed;
those cover ordering, scroll preservation and selection reveal, not native
race reproduction. No app bundle rebuild or release was performed.

Diagnostic panel interaction batch: Listening Ports keeps its previous snapshot
visible during refresh with an explicit label, preserves selection made during
the scan by address/PID, and avoids scrolling on copy/unrelated typing or an
unchanged refresh. Failed/expired-process scans clear old results. Ports and
Terminal Memory now support Tab/Shift-Tab and leave modified keys to shortcut
routing, aligning with Settings and Program Status. The compiled panel-focused
filter passed 19 tests (including listener identity and existing panel behavior).
Native pointer/keyboard acceptance remains pending; no extra app bundle or
release was built for this batch.

Responsive inspector layout: Settings and Notifications now share a 380-point
panel shell. When the viewport minus the visible sidebar cannot leave 400 points
for terminals beside the panel, the panel overlays the right edge with occlusion
and shadow instead of shrinking the terminal/PTY layout. Width is capped at the
viewport, allowing the full panel width on compact windows rather than a 50%
cap. One native bundle build passed. The isolated compact fixture stayed live,
but UI control reported no available windows after capturing its initial state;
visual acceptance is still pending. The fixture was stopped without touching the
installed app. No additional test suite or release run was performed.

Settings recovery batch: Revert Unsaved Settings restores panel-controlled
values from the shared last-loaded/successfully-saved baseline without file I/O.
It preserves commands, shortcuts, paths, explicit colors and environment policy,
including loaded imported-theme data. Footer, menu, palette and custom action
`revert_settings` use one guarded handler; busy operations cannot be reverted.
Five appearance persistence/restore tests passed, followed by one native bundle.
Native compact-window review verified opening Settings, 13→14 pt marking dirty,
End/Enter reverting to 13 pt and clearing dirty status, and footer activation
of Keyboard Shortcuts. Escape returned to Settings and another adjustment is
visible in `target/settings-revert-review.png`. Final terminal-focus review was
interrupted by the UI tool and is not accepted. The compact view exposes cramped
terminal/notice space beside Settings; responsive panel layout remains follow-up.
The disposable fixture was stopped; installed app sessions were untouched.
This is session reversion, not A09's full receipt-based conditional disk undo.

Finished-terminal status polish: tab strips now suppress live task/progress
badges once the terminal exits, avoiding contradictory Working/Finished labels.
The Program Status panel retains its records under Last reported status, and
both pointer and keyboard copying include an explicit finished-terminal prefix.
Task navigation now supports Tab/Shift-Tab and leaves modified keys to shortcut
routing. The focused status test passed, including selection identity and copied
historical context. Native visual acceptance of these changes remains pending.

Settings keyboard follow-up: Left/Right now adjust values without launching
the font picker, theme import dialog or external editor. Enter/Space activate
those action controls. Keyboard activation of fixed-footer actions also respects
the same busy state as pointer clicks. Both targeted Settings tests passed,
covering value adjustment, action activation, navigation wrapping and bounds.
No additional app bundle build or repeated native capture was performed.

Settings save-state polish: the fixed footer now reports unsaved session
appearance changes against a shared last-loaded/successfully-saved baseline.
Successful saves record the submitted snapshot, so edits made during a save
remain dirty. Failed saves preserve the baseline, and applied reloads replace
it. All Settings windows observe baseline updates. This is session state, not
external-file monitoring. The existing persisted-appearance comparison test
passed with the desktop test binary compiled; native visual acceptance remains
pending after the prior UI capture limitation. No release or deployment.

Editor handoff release follow-up: workspace.open now preserves the original
window search scope and rechecks it after asynchronous terminal preparation,
in the same foreground update as publication. A workspace added manually in
another eligible window can be reused; newly ambiguous names fail instead of
publishing a duplicate. The launch window's busy flag is cleared even when
reuse or ambiguity discards the prepared terminal. One desktop build and the
isolated named-workspace smoke passed (creation, retained IDs/PIDs/Unicode/cwd,
ambiguity and explicit-window lookup). The manual cross-window startup race
was reviewed in code, not reproduced through UI automation.

Native review at cf3598c used a uniquely identified disposable app bundle.
Cmd+Option+V executed the configured background marker command successfully;
the workspace/tab layout remained unchanged in the captured window. Settings
shortcut/menu actions did not produce an observable panel through the UI tool,
so settings interaction acceptance remains unresolved. The disposable fixture
was stopped; the installed app and user sessions were left running.

Shortcut search polish: modifier/key queries in the dedicated shortcut view
now require real binding tokens rather than fuzzy letters from command titles.
An all-key query must fit one alias, preventing cross-alias false combinations;
modifier-plus-action queries retain fuzzy action matching. Shared filtering is
used by native/WASM. Query-dependent ranking can reintroduce matches on appended
text and correctly returns to ordinary search. Five focused shortcut-label tests
and ten core search tests passed, plus one WASM build. Browser review verified
`ctrl tab` shows only Previous/Next Pane (not Close Tab or Move Tab), then appending
` move` reveals the tab-move commands. Screenshot:
`target/shortcut-search-polish.png`. Native rendering remains unverified.

Release-quality follow-up (93fce70): the existing native live-shell transfer
smoke passed on one diagnostic rerun, including UTF-8 delivery, seven retained
shell PIDs/variables across merges and transfers, stale-target rejection and
fixture child cleanup. The prior state timeout did not reproduce; its cause is
not established. Failure-only diagnostics now report actual fixture state and
bounded fixture screen contents instead of just a missing-state message.
Using the existing WASM build, shortcut discovery rendered alternate bindings
for Next Tab and Previous/Next Pane; searching `ctrl tab` matched the alternate
pane bindings, and Escape restored terminal input focus. Screenshot saved at
`target/shortcut-aliases.png`. No rebuild, release or deployment in this pass.

Named workspace handoff: `rmuxctl open-workspace --name NAME [--window ID]
[--cwd PATH] [--activate]` and `workspace.open` reuse exact normalized names
across normal windows or create in the requested/active/fallback window. Selection
defaults on; OS activation stays explicit. Reuse leaves metadata, layout, cwd and
PTYs unchanged; replies include `reused`. Duplicate names fail without mutation,
including duplicates inside an explicitly scoped window. Existing serialized
workspace preparation prevents concurrent open requests from independently
creating duplicates; publish rechecks same-window names after async startup.
CLI help/completions and editor handoff docs updated. CLI and RPC parser tests
passed, then one desktop/CLI build. `smoke-open-workspace.py` passed against an
isolated native app with real /bin/sh: creation, stable IDs/PIDs, retained Unicode
shell variable/cwd, ignored creation cwd on reuse, ambiguity and scoped lookup.
A broader smoke attempt failed in its pre-existing transfer shell-state check
before reaching this feature; no broad-suite success is claimed. OS activation
and Windows runtime validation remain pending. No installed app was modified.

Configured background commands: custom commands accept `mode: "background"`
(default remains terminal), reuse existing named command shortcuts/palette, and
spawn literal argv with child env/cwd without creating a workspace or PTY.
Launch and exit failures surface as notices; output/input are disconnected and
eight concurrent children are allowed. Spawning/reaping stay off the UI thread;
idle apps have no polling cost. Palette previews identify execution mode. SSH
continues to use terminal mode. Standalone `--command` supports background mode
without GUI, waits for completion, reports failure, and rejects live RPC mode.
The user guide includes a macOS VS Code focus example and process lifetime rules.
A real-process unit test passed for literal Unicode/metacharacter arguments,
env/cwd, exit failure and failed-launch slot release. One desktop build and an
isolated CLI smoke passed for output, nonzero exit and RPC incompatibility.
Actual shortcut-to-app activation remains unverified; no user config was edited.
Open-or-reuse named workspace remains separate pending work.

Shortcut discovery batch: native and WASM shortcut views now collect every
effective alias for each listed command from GPUI, highest precedence first,
and deduplicate equivalent display spellings. All aliases are searchable;
ordinary palette/menu hints keep a single preferred binding. Custom-binding
validation errors identify the zero-based binding index and scope, with the
other entry identified for duplicate/chord-prefix conflicts. Existing scope
precedence and disabled-binding behavior are unchanged. Twenty-one focused
shortcut/configuration tests and one WASM build passed. Visual alias layout and
native shortcut-list interaction remain pending.

Browser help acceptance (d863079): an isolated sandboxed iframe without
allow-popups reproduced blocked tab creation. The labelled fallback rendered
correctly, focused its guide link, and Escape removed the dialog and restored
terminal input. A subsequent `echo focus-restored` executed exactly once in the
sample shell. Screenshot: `target/help-fallback.png`. No rebuild was needed for
this browser review. A combined native bundle passed, but native automation
reported the debug/release bundle-ID collision and retained the old ID after an
ephemeral debug-only identity change. The experiment was reverted and both
synthetic preview processes quit cleanly; native interaction acceptance remains
pending. Installed user app and release configuration were untouched.

Browser help resilience: help commands now use an opener-isolated new tab and
show a native HTML link dialog when tab creation returns null or throws. The
fallback has a labelled heading, keyboard-accessible link/Close control, native
Escape dismissal and origin-focus restoration, with no uploaded app data.
Three focused Node tests cover successful navigation/opener removal and both
blocked paths with link attributes and focus restoration. WASM build passed.
The public user guide destination was observed in a separate browser tab; the
modal fallback has unit coverage but still needs rendered/embedded acceptance.

Help discovery batch: native Help menu now links to the user guide, searchable
keyboard shortcuts, issue reporting and rmux.app. Shared command identities and
fixed public URLs also expose guide/issue/website actions in native and WASM
palettes; native custom shortcuts support open_user_guide/report_issue/open_website.
No terminal contents or diagnostics are attached. User guide documents the flow.
Desktop offline cargo check and one WASM build passed. On fresh localhost:4322,
searching "user guide" shows the shared command and Enter dismisses the palette
with input focus restored. Screenshot: `target/help-palette.png`. The browser
harness did not expose a new destination tab, so external-link opening remains
unverified; native Help-menu interaction also awaits a working native UI harness.
The older localhost:4319 browser view retained stale assets despite reload.

Settings pointer/keyboard continuity: footer clicks now select the action they
activate, so a subsequent Enter no longer acts on an unrelated setting. Modified
keys are ignored except Shift+Tab navigation. Focused settings tests passed.
Combined native bundle through b08061f passed; the isolated preview rendered the
workspace layout, but UI automation did not apply menu/keyboard interactions, so
settings/footer/notification pointer acceptance remains unverified. Saved native
overview at `target/native-polish-overview.png`; preview quit cleanly and the
installed app was left running. No release or deployment.

Notification interaction polish: selected cards now have an accent outline;
empty filters explain how to return to earlier messages. Clearing history or
acknowledging all unread messages resets stale selection/expansion and scroll
state. Modified keys no longer trigger plain notification actions, while
Shift+Tab still moves backward. One offline desktop cargo check passed; native
visual and pointer acceptance remains for the combined UI review. No release
or deployment was triggered.

# Implementation progress

## Current implementation focus

Snapshot allocation checkpoint (505b6c5): the existing debug allocation benchmark
at 1/8/32 terminals exactly matches prior allocation counts/bytes. Identical ASCII
rewrites require zero Rust allocations; changed ASCII rows require four calls and
6,032 requested bytes per snapshot; long combining-grapheme style changes require
five calls and 6,113 bytes. Recorded in
`docs/benchmarks/snapshot-allocations-polish-2026-11-21.jsonl`. These are allocation
traffic measurements, not retained memory, total process RSS or latency budgets;
setup/parser writes and non-Rust allocations are excluded. Single-run timing is
not a speedup claim. No snapshot-cache change justified by this checkpoint.


Settings workflow completion: fixed-footer actions now include Reload from file
and Keyboard Shortcuts alongside Save/Open settings file. A shared footer-action
mapping drives pointer and keyboard activation; Tab, arrow and Home/End navigation
includes all four controls. The reload tooltip explains replacement of session
settings and preservation on invalid input, reusing the existing asynchronous
reload path. Focused settings navigation/bounds test passes. Native visual and
pointer acceptance remain for the next combined review; no app bundle rebuilt.


Accumulated-polish native reliability check (0241f26): one desktop build followed
by `smoke-live-rpc.py --live-shell` and `smoke-session-restore.py`, with isolated
config/layout/session files and fixture shells. Both passed. Live transfers and
merges preserved process/surface identities, seven shell PIDs, Unicode variables,
output and hidden-tab unread state while removing emptied source windows.
Restore covered two windows, metadata, weighted splits, selected tabs, fresh
shells, normal and forced-stop relaunch, corrupt/unsupported session fallback,
and preservation of original corrupt bytes on normal Quit. All fixture cleanup
passed. These checks do not establish visual focus, OS activation, UI pointer
behavior or latency budgets. User sessions and installed app were not replaced.


Sidebar editor polish: native and WASM workspace/group metadata now share a
bordered input, explicit Save/Cancel buttons, keyboard hint and accent-matched
icon/color presets. Button events use existing validation and focus return and
stop bubbling into workspace selection. Narrow-sidebar review caught a squeezed
hint beside the controls; it now occupies its own line above the button row.
Native compile and WASM build pass. Browser preset+Save applied a blue workspace
marker; preset+Cancel retained the original unset color and restored terminal
focus. Screenshot: `target/sidebar-editor-polish.png`. Native pointer acceptance
and each metadata field's validation paths remain outside this focused review.


Command coverage polish: Copy, Paste and Previous/Next Pane are now discoverable
in native and WASM palettes and effective-shortcut lists. Both hosts resolve them
through shared action adapters; existing clipboard, paste and pane-cycle handlers
remain responsible for behavior. Native palette terminal commands now use the
shared terminal adapter too. Native and WASM compile checks pass; new palette
entry interaction remains for the combined review. W10 implementation ledger
updated without claiming complete parity or runtime acceptance.


Shared tab rename polish: a bordered input, clear title/hint and visible Save /
Cancel buttons replace the keyboard-only editor strip in native and WASM.
Buttons use the same TextField submit/cancel methods as Enter/Escape and do not
commit unfinished IME composition. Native compile and WASM build pass. Browser
pointer acceptance renamed Terminal 1 to Build logs, then cancelled a second
rename; the title stayed Build logs and focus returned to the unchanged sample
terminal. Screenshot: `target/tab-rename-polish.png`. Native pointer and narrow
pane wrapping acceptance remain open. Existing exited-terminal output retention
and fresh-shell/SSH recovery were inspected, with no change needed in this pass.


Shortcut label correction from the browser review: shifted bracket and numeric
aliases now display the base key once when Shift is already present (for example
⇧⌘] rather than ⇧⌘}, and Ctrl+Shift+2 rather than Ctrl+Shift+@). Literal symbols
without Shift remain literal. The shared formatter serves native and WASM chrome.
Search terms also accept the long names page up, page down and escape. Two
focused formatter/search tests pass; no app or demo rebuild for this text-only
follow-up. No binding or dispatch semantics changed.


Shortcut discovery now also runs in the WASM demo. Native and browser use the
same command identity and symbolic/plain key search terms. The browser view
filters resolved command hints, sorts empty-query results by title, preserves
original command indices for dispatch, and resets to the ordinary palette on
reopen. Browser acceptance: searched Keyboard Shortcuts, then `cmd shift`, chose
Split Below from the filtered list and observed a second pane. Reopening Cmd+P
returned the normal heading and unbound Keyboard Shortcuts entry. Screenshot:
`target/shortcut-discovery.png`. Native check and WASM build passed; native list
visual acceptance is still open. Production demo has not been deployed.


Shortcut discovery: View and the command palette now expose Keyboard Shortcuts.
It reuses the resolved command catalog, keeps only bound commands, sorts by name,
and searches command titles and symbolic/plain key names. Enter runs through the
same palette dispatch path; Escape uses normal focus restoration. It shows the
preferred binding in the originating focus context, not all aliases or text-editor
keys. A custom `keyboard_shortcuts` action is supported. Native bundle passes;
the menu entry was observed. The UI tool lost access to the isolated window before
list interaction/visual review, so that acceptance remains open. The preview
process was still live and was stopped through its normal SIGINT cleanup.


Combined browser input review (ee65dca): rebuilt the WASM demo once and checked
it at localhost:4319. Cmd+P opened the palette; Down selected New Terminal Tab;
Shift+Down followed by Backspace on the empty query preserved that selection.
Escape dismissed it. Cmd+F opened Find; after typing `rmux`, opening/dismissing
the palette and typing ` find` produced `rmux find` in Find while the sample
shell prompt stayed unchanged. Clear followed by Cmd+Z restored that query.
Screenshot: `target/input-review.png`. Live IME composition, native modified-key
behavior and multi-message notification interactions remain unverified. No
production demo deployment or release was performed.


Find/input polish: identical replacements and boundary deletions now update
caret/selection without publishing a false text-change event, avoiding needless
search restarts and palette-selection resets. IME commits still publish the
committed value even when it equals the preedit buffer. Native and browser Find
navigation refuse unfinished composition, and shared match buttons dim while
composition is active. Native and WASM compile checks pass; live IME and no-op
editing acceptance remains for the combined input review. No app bundle rebuilt.


Palette/picker keyboard consistency: the native command palette, native tab
picker and WASM palette now share a plain-arrow/PageUp/PageDown classifier.
Modified keys remain available to editing/configured bindings rather than
silently changing the selected result. Plain list-navigation keys are consumed
even with no matches; IME composition retains its existing bypass. A focused
regression covers all four navigation keys with Shift/Control/Alt/Command and
excludes editing/submission keys. The test and WASM target check pass. Native
and browser interaction acceptance remains for the next combined UI review.


Palette redraw cost: result-area measurement now stops when the 320 px panel
is filled instead of scanning every matching command/file-preview line on each
redraw. The production renderer now uses the bounded helper previously present
only in tests. A 100,000-row regression verifies only nine ordinary rows are
measured, with mixed/wrapped-row sizes preserved. Unused preview link, wrapping
and excerpt-copy shortcut hints are no longer resolved in ordinary palettes.
All six focused palette navigation tests pass. This is an operation-count
improvement; no end-to-end latency or memory benchmark is claimed.


Notification panel polish: pointer-down on a card now selects its stable message
identity, including when interacting with More/Copy/Mark read controls, so later
keyboard actions target that card. Home/End selects the first/last message.
Keyboard guidance stays in a fixed footer; list scroll indices were adjusted
for removal of the old hint child. Panel width, heading and accent match Settings.
One native bundle build passed, and the rendered panel was inspected in the
isolated synthetic workspace. Screenshot: `target/notifications-polish.png`.
The UI tool reset during interaction checks; multi-message pointer/keyboard
selection, paging and unread-filter removal still need native acceptance.


Settings polish: Save, the settings-file link and keyboard guidance now stay
visible in a fixed footer while settings scroll. The panel uses the shared
purple selection/accent colors, gives labels more room, explains session-only
changes at the top, and names the setting in stepper tooltips. Keyboard navigation
to footer actions no longer scrolls the list. One native bundle build passed;
an isolated native session verified Cmd+, opening, End then Shift+Tab selecting
Save without scrolling, and Escape returning terminal focus. Screenshot:
`target/settings-polish.png`. No user settings were saved or sessions replaced.


Tab chooser sizing is now visually verified: a fixed preferred width with a
pane-width maximum, explicit search-wrapper width/minimum and clipped panel keep
the input inside the 520 px chooser. Rows fill its width. Native review caught
and corrected the previous max-width-only field overflow; the final debug bundle
renders correctly in the 12-tab fixture. Screenshot: target/tab-chooser-final.png.
Earlier native search/Enter selection evidence still applies; no shortcut mapping
was changed. This completes the local chooser presentation batch.

Tab chooser native review: debug bundle build passes. In an isolated 12-tab
fixture, Cmd+Shift+T opens the chooser, query 07 highlights Terminal 07, and
Enter selects it and scrolls the tab strip to expose it. Screenshot:
target/tab-picker-polish.png. Visual review exposed an overly wide panel and
content-width selection backgrounds; follow-up styling caps the panel at 520 px
within its pane and gives result rows full width. These final sizing edits have
not been rebuilt. Fixture shutdown completed cleanly; no user sessions changed.

Native tab chooser polish: a heading/count, explicit Close control, search
placeholder, padded field and separate keyboard footer replace the dense single
instruction line. Titles and directories highlight fuzzy matches with the same
Unicode-aware matcher as the command palette. Hover colors follow shared chrome;
Escape, outside click and Close share dismissal/focus restoration. Desktop cargo
check passes. Visual/narrow-pane review is pending with the next native batch.

Find/focus batch accepted for browser interaction: desktop cargo check and the
combined WASM build pass. Browser review confirms populated field/clear control,
empty placeholder and dim navigation, Clear retaining focus, Cmd+Z restoring the
query, and palette Escape returning typing to Find without changing shell input.
Screenshot: target/find-polish-batch.png. Shared native presentation compiles;
native pointer interaction was not repeated. Batch committed locally, without
release/deployment.

Find-bar polish joins the held focus batch: shared native/WASM query construction
adds a placeholder, the field gets a padded outline with an error tint, and a
Clear action resets through the normal undoable edit path while keeping Find
focused. Clear is hidden during composition; empty/error queries dim navigation
controls. The empty-state duplicate status label is suppressed. Diff check passes;
combined build and visual acceptance of this presentation change are pending.

Palette focus polish: browser dismissal (Escape, Close, backdrop or toggle)
restores the original focus handle, matching native behavior when invoked from
Find. Command execution/layout transitions still focus the active terminal and
clear the saved origin. WASM build passes. Browser flow verified: Cmd+F, type
rmux, Cmd+P, Escape, type " focus" updates Find to "rmux focus" and leaves the
shell prompt untouched. Screenshot: target/palette-find-focus.png. Held locally
for the next product polish batch.

Sidebar/tab batch review: native bundle and WASM compilation pass together.
Native screenshot target/sidebar-batch-review.png confirms title-line controls,
full-width details, readable single-line notification ellipses and full-height
tab selection. Review showed a fixed-length path suffix can still hide the final
folder beside a long branch; long paths now prefer …/final-folder, retaining a
character-bounded suffix for unusually long folder names. That final formatting
adjustment has a clean diff but has not been rebuilt. The isolated native review
process exited cleanly; user sessions remain untouched.

Continuing the held sidebar polish batch: branch and directory labels now have
a visible separator, and long directories retain their final 28 Unicode scalar
values with a leading ellipsis, keeping the destination visible instead of only
the root prefix. Full branch/path context remains in the hover hint. Unread
previews request an ellipsis, and browser title rows use the native 24 px minimum
height and tighter control spacing. Diff check passes; combined compilation and
visual review of these follow-up edits are pending. No additional build run.

Visible sidebar density polish: group disclosure, unread count, name, pin, add
button and shortcut now occupy the title line rather than being centered beside
the entire detail block. Descriptions, message previews and branch/directory
context use the available row width. Unread previews are bounded to one line;
tab selection fills the strip height. Native debug build and synthetic layout
review pass. Review caught a truncate() layout issue showing only an ellipsis;
using a one-line clamp restores readable previews. Screenshot:
target/sidebar-density-polish.png. Changes remain local; no release/deployment.

Sparkle generation check now exercises the real pinned generate_appcast tool
against a disposable ad-hoc-signed copy of the existing debug executable, with a
public Ed25519 test-vector key. scripts/check-generated-appcast.py confirms its
embedded Markdown notes, version-specific history link, ZIP URL/length and
signature shape pass the publication validator. Initial fixture failures exposed
missing bundle sealing and SUPublicEDKey in the fixture; both are now supplied.
No production keychain/signing credentials, native release build or publication
was used. This verifies actual feed generation, not native note rendering or
installation. Run via mise exec -- python3 scripts/check-generated-appcast.py.

Website promotion: GitHub Shared WASM demo run 44974683364 succeeded for
e76c9648739799f335b6484c18e0e10835ea9545 in 4m15s, including compiler dependency,
font/startup, engine adapter and incremental-search checks. Imported that artifact
and deployed directly from the local site/ops directories to Cloudflare deployment
bf36d2a9.rmux-site.pages.dev. The production rmux.app HTML references the new
versioned demo path and all three existing 0.1.5 downloads. Website source remains
off GitHub. Next-release notes are drafted in docs/releases/v0.1.6.md; package
version and native release remain 0.1.5.

Native visual review of 7454978: one debug bundle build succeeds, and an isolated
synthetic layout renders the shared palette, shortcut badges and sidebar colors.
Screenshot: target/native-chrome-polish.png. The machine's Metal HUD is visible;
its values are not a controlled performance measurement. Keyboard follow-ups did
not visibly change the palette and the UI tool subsequently reported no windows
available, so dismissal/input and notification mouse checks are not accepted.
The fixture exited cleanly through its RPC shutdown handler; the installed app
and user sessions were not replaced. Continue native interaction review when UI
access is available rather than rebuilding this unchanged batch.

Completed the combined chrome/input polish batch with notification-panel fixes:
desktop close, open and dismiss handlers now require standard clicks, matching
the existing read/copy/filter controls. The close control uses the shared 28 px
hit area and shortcut tooltip; notification selection/hover colors use the shared
chrome palette. Desktop compilation passes after these additions. Native runtime
notification acceptance remains pending; the prior browser review covers the
WASM changes. The complete batch is committed together without a release build.

Combined chrome and browser interaction batch: one WASM debug build passes.
Browser inspection confirms right clicks on new-workspace, new-tab and Commands
do not activate them; normal clicks create a workspace/tab and open the palette,
and Escape dismisses it. The browser workspace add button now shares native
geometry, vector icon and shortcut tooltip resolution. Tab selection/context menus
and workspace selection resolve stable IDs at event time rather than captured
positions. Empty-state creation controls also filter standard clicks. Screenshot:
target/chrome-polish-batch.jpg. Palette/sidebar colors visually agree; native
menu/list rendering remains pending. Changes remain local for batch delivery.

Application selection styling now shares one chrome color definition across
desktop and WASM. Palette results/search borders, terminal and workspace menus,
settings selection, memory/ports/status lists and copy-mode rows use the same
muted accent as the sidebar. Selected hover states and group controls also share
that treatment. Terminal theme colors remain independently configured. Desktop
and WASM cargo checks pass for the complete batch; no packaged build, deployment
or release was run. Visual acceptance of this latest color batch is pending.

Combined sidebar/toolbar/tab polish review passes desktop compilation and one
WASM build. Tab close buttons now share the native 22 px hit area, vector icon,
tooltip and hover treatment; left mouse-down does not start a parent tab drag.
Browser close clicks require a standard click and resolve the stable terminal ID
instead of using a captured positional index. Browser inspection confirms right
click opens the menu without closing, Escape dismisses the menu, and a normal
close removes only the targeted tab. Screenshot: target/tab-close-polish.jpg.
This batch is pushed together with the previously held sidebar and shortcut
tooltip changes; no native release is triggered. Hover hint appearance and native
pointer acceptance remain pending beyond the shared compile check.

Continuing the held polish batch: toolbar action tooltips now share the same
binding resolver between native and WASM hosts. Sidebar toggle, pane split/zoom
and browser new-tab controls resolve current shortcuts at hover time; pane
controls use their own active terminal's focus context. Browser close-tab controls
also gain the native description. Changes are held locally with the sidebar/tab
styling for a combined compile and visual review; only diff checks have run on
this continuation, avoiding a build for each individual polish edit.

Local sidebar/tab polish batch: selected workspaces now use a subdued lavender-gray
fill with a visible outline instead of saturated blue. Shared accent/detail
colors coordinate native/browser icons, metadata, shortcuts and native unread
badges. All rows reserve the same border space, avoiding selection size changes.
Focused tab accents use the same accent; inactive tabs retain a muted outline,
and tab hover feedback is explicit. Desktop check and one WASM build pass. Browser
inspection covers two panes with multiple tabs and active/inactive focus styling;
screenshot: target/sidebar-tab-polish.jpg. Native graphical review remains pending.
Held locally for a larger polish push; no release/deployment triggered.

Next-release quality batch: the macOS packaging script supplies the existing
Markdown release notes to Sparkle's pinned generate_appcast tool with
--embed-release-notes and a version-specific full-history link. Publication now
rejects a missing/duplicate version, incorrect display version/download URL/ZIP
length, malformed Ed25519 signature metadata, empty notes or wrong history link
before creating the GitHub release. A focused fixture checks valid metadata and
seven failure cases; it and shell syntax checks pass, with no app build. The
fixture also runs in release preflight. Actual generated notes and their native
rendering still need the next release run; existing 0.1.5 assets/feed are unchanged.
Signature shape checks are not cryptographic verification.

Native release follow-up: the installed /Applications/rmux.app (0.1.3) successfully
discovers 0.1.5 through rmux → Check for Updates. Sparkle displays the correct
versions and app icon. Screenshot: target/sparkle-015-discovery.png. No install or
Skip This Version action was taken; Escape/Cmd+W did not dismiss the update window,
which remains open. This verifies update discovery only, not download, replacement
or relaunch. Native app inspection is available again; earlier locked-Mac notes
are historical. GitHub currently has no open issues. This note is held for the
next batch commit rather than triggering another push/build.

0.1.5 is published from GitHub release run 44972522111 at 61b52a1. All three
platform builds and publication succeeded; the release includes macOS DMG/ZIP,
Linux tar.gz, Windows ZIP, checksums and the appcast. The main-branch Sparkle feed
now advertises the 0.1.5 macOS ZIP with its Ed25519 signature and matching asset
length. This verifies publication, not a completed in-app upgrade on a user's Mac.
Cloudflare deployment 7e1422a7 combines the new download links with successful
GitHub WASM run 44972516518 from the same source commit. Fetching rmux.app confirms
all three download URLs and the versioned demo path. Website source remains
off GitHub. Linux/Windows graphical runtime acceptance is still outstanding.

Prepared the 0.1.5 preview release as one batch: workspace/lockfile version bump,
user-facing notes, and an inexpensive release-workflow preflight before all
platform builds. Preflight checks note/version agreement, local rmux package
versions in both lockfiles, and absence of the release tag on origin. The actual
0.1.5 preflight passes; injected existing-tag and remote-failure responses reject
as expected. No local production build was run. All platform release builds,
signing/notarization and publication run on GitHub. Release completion and the
Sparkle feed remain pending until that workflow succeeds.

Current priority is polish and release quality. Related changes are batched before
focused checks/builds and pushes, per the user's instruction; full parity remains
the longer-term scope. The shared palette presentation batch aligns header and
Close controls, gives Close a 28 px height, shares bordered shortcut badges
between desktop and WASM, replaces space-padded footer text with wrapping keyboard
hints, and improves empty-state spacing. Desktop check and the WASM build pass
(one WASM delimiter typo was corrected during this batch). Browser inspection
confirms populated/empty layouts and Close returning focus to the terminal.
Screenshot: target/palette-polish-batch.jpg. Native graphical acceptance and
deployment of this batch are still pending.

Fuzzy highlighting now also covers palette result subtitles such as terminal
directories. Literal terminal-search excerpts and document previews retain their
existing behavior. Contiguous highlighting marks ranges directly without a new
per-word match vector; fuzzy words reuse one scratch vector per label. Nine
focused search tests and desktop compilation pass, including complete-word
rollback and directory matching against the ranker's greedy subsequence.
This is an allocation reduction by inspection, not a measured latency claim.

Command labels now highlight fuzzy subsequences as well as contiguous words in
the native and WASM palettes. For example, `splt blw` visibly marks the matching
letters in Split Below using the same greedy path as ranking. Incomplete words
do not leave misleading partial highlights; contiguous matches retain priority.
Literal document-preview highlighting remains unchanged. Nine focused shared
search tests, desktop cargo check and the WASM debug build pass. Local browser
inspection confirms the result; screenshot: target/palette-fuzzy-highlights.jpg.
The hosted demo still requires a new GitHub artifact deployment.

Workspace merge acceptance now checks per-pane tab indices and selected tabs,
plus complete notification records on a hidden tab, before and after both
same-window and cross-window merges. Notifications retain their revision,
Unicode title/body and unread state while ownership updates to the destination.
The native live-shell smoke passes these assertions alongside seven retained
shell PIDs/variables and source-window cleanup. This closes the narrow W15
tab-order/unread evidence gap; graphical focus, groups, persistence and other
platforms remain pending. No product behavior change was needed.

Release-documentation review: N03 already correctly records the opt-in native
notification dispatcher and its pending platform acceptance. README's blanket
claim that native notifications are absent was stale; it now distinguishes main
from installed releases, links the notification guide, and describes the
experimental browser as separate from workspace panes. Source inspection confirms
bounded delivery, focus suppression, and macOS/Linux click routing; this review
does not establish native permission, delivery or click runtime acceptance.

The shared WASM demo is now deployed from successful GitHub run 44970793969
(e3fc657), including the contextual Greek search-highlight correction. Cloudflare
deployment 77c82b1d completed successfully; fetching rmux.app confirms its demo
links and iframe reference that exact artifact. The existing Escape, Close button
and backdrop dismissal handlers remain in place; this deployment adds no new
dismissal behavior. Website source remains local/off GitHub. Native release
downloads are unchanged.

Offline --kind completion now offers supported event names by prefix and omits
already-selected kinds. The CLI and server capabilities/validation share one
rmux-protocol vocabulary with no dependencies, avoiding separate drifting lists
or a GUI/terminal dependency in rmuxctl. Both completion tests and all five event
journal tests pass. CLI-version completion remains offline; live target-ID and
path completion are still pending.

CLI completion now recognizes --follow, --json, --preserve-wraps and --bracketed
as switches instead of swallowing the following word as their value. Repeatable
event --kind remains offered after a prior kind, while single-use options stay
hidden once supplied. Both focused completion tests pass, including subsequent
value-taking options and literal payload handling. GitHub native run 44970793838
also passed its live replay/cross-window ownership step, closing the initial CI
runner evidence gap. Native graphical acceptance remains blocked by the locked Mac.

rmuxctl read-events --follow now writes flushed JSON-RPC batches continuously,
advancing the cursor only after successful output. Default waiting is 3 seconds;
zero-wait following is rejected. Empty batches remain checkpoints and restart/
retention gaps remain visible. RPC errors exit 1; transport errors stop without
retry (resume from the printed cursor). Broken pipes end cleanly, and slow output
consumers do not keep a server waiter alive between reads. Thirty CLI tests and
native smoke pass, including consecutive flushed event/empty batches with no
replayed closure. Follow-specific checks cover gap/cursor/filter preservation,
invalid replies, transport failure and closed-output handling. GitHub now runs
the small full CLI suite alongside this native smoke. This is long polling, not
server push subscriptions or automatic reconnection.

The GitHub-built e06bf79 demo (successful run 44970452287) is deployed to rmux.app
via Cloudflare deployment 60250455. The first Cloudflare request returned internal
error 8000000; one retry succeeded. Live browser inspection confirms the versioned
artifact URL, highlighted workspace matches and Escape dismissal. Screenshot:
target/live-palette-highlights.jpg. Website source remains local/off GitHub.
The later contextual Greek fix e3fc657 is committed but not in this deployment.

Shared match highlighting now uses whole-string lowercase like search ranking,
fixing contextual Greek final sigma (ΟΣ). A regression first reproduced the
missing highlight, then passed with original UTF-8 ranges preserved, including
an expanding İ prefix. All eight focused shared search tests pass.

The browser command palette now highlights matching query words in result labels,
using the same Unicode-safe range mapping as native previews. That helper and its
regression test moved into rmux-core; desktop retains its existing call sites via
an alias. Seven shared search tests and the debug WASM build pass. Local browser
inspection confirms highlighted workspace results and aligned shortcut hints;
evidence: target/palette-match-highlights.jpg. The hosted demo still requires a
GitHub-built artifact import/deploy.

The existing macOS GitHub job now builds the app/CLI with its already-used browser
feature set and runs the live event smoke plus the focused CLI event parser test.
CLI and smoke-script changes trigger this job. The smoke covers filtered replay,
concurrent clients, real shell metadata/bells, surface and workspace transfers,
PID continuity and lifecycle cleanup. The exact browser-enabled build/smoke command
passes locally; the new hosted step is not yet verified on a GitHub runner.

Whole-workspace moves now publish workspace.location_invalidated with the stable
workspace ID. Native replay smoke moves a populated workspace between two windows
and back, checks destination window IDs and retained terminal PID, and rejects
false creation/closure events. A same-window move is a no-op with no invalidation.
The smoke passes; rollback semantics remain invalidation-only, and detached-window
UI acceptance remains pending.

Replay now includes workspace.created/workspace.closed with workspace_id instead
of surface_id. Workspace ownership tokens avoid duplicate close records and move
with their entity; explicit close, empty-source removal and merge close promptly.
RPC filtering and rmuxctl --workspace select these lifecycle records independently
of surface filters. Native smoke passes creation, last-tab departure closure and
explicit closure alongside cross-window surface transfers. The focused CLI parser
check passes; workspace journal coverage checks filtered wake-up, wire fields,
invalid IDs and duplicate-close suppression. Restore/merge and whole-workspace
window transfer runtime acceptance remain pending.

Native replay smoke now moves a terminal to another window and back. Both moves
preserve surface ID and process PID, emit filtered location invalidations and
report the destination window/workspace. Sending a command after each move
produces three new terminal bells, checking live PTY input/output beyond retained
snapshot data. Moving the last tab removes its empty source workspace without
false close/create events. The complete smoke passes on macOS. Pointer-drag,
visual focus and other-platform acceptance remain pending.

Shared palette ranking now counts Unicode scalar positions consistently for
contiguous and fuzzy matches, so a multibyte workspace-name prefix does not
artificially lower its rank. Identical case-normalized queries reuse their prior
ranking. Six focused core search tests pass, including Unicode prefix ordering,
case-only edits and incremental/full-ranking equivalence. Desktop and WASM use
this shared matcher; no input-latency improvement is claimed from these tests.

The WASM workflow no longer rebuilds for changes confined to the native command
palette or Markdown parser. A fresh WASM compilation and rustc dependency-file
check confirm all 14 explicit native exclusions are outside the compiled demo.
Shared palette presentation and text-input files still trigger browser builds.
GitHub native/browser runs for `77cc11a` and `dd6cff4` passed; newer native runs
were still active/queued at inspection. No build-duration improvement is claimed
until subsequent CI runs provide measurements.

Surface location invalidations now cover attachment to existing/new panes,
workspace owner changes and window rebinding. Clients can refresh current
window/workspace/pane IDs without restarting or polling the terminal. Events
retain the stable surface ID; duplicate/rollback invalidations are explicitly
allowed, and in-pane tab reorder is outside this contract. Native replay smoke
passes a live transfer, filtered invalidation, updated workspace snapshot and
absence of false create/close events. Native pointer transfers and cross-window
interaction acceptance remain pending.

Code viewports now prepare their display lines once, share the resulting strings
with render elements, and recompute highlight ranges only when the query changes.
The cache belongs to the preview and is discarded on close/refresh; source limits
still bound its size. Desktop test compilation and the focused typography test
pass. No measured latency improvement is claimed.

Markdown code blocks now keep all captured source lines in a compact scroll
viewport instead of clipping to a four-line excerpt. Per-block scroll handles
retain position across redraws; a changed search reveals the first matching line.
Tabs retain Unicode-aware four-column stops and controls remain inert; copying
keeps original source. The existing file byte/line limits still apply. Fourteen
Markdown tests pass, including long Unicode lines, all 512 source lines and search
position. Native scrolling, horizontal layout and interaction acceptance remain
pending while the Mac is locked.

Markdown preview destinations now use immutable shared storage across table cells
and render elements. Click handlers, tooltips and the keyboard destination bar
clone shared string handles instead of copying URL contents on redraw. Desktop
test compilation and the focused typography test pass. This removes known copies;
no new latency or memory benchmark claim is made. Native visual acceptance remains
pending.

Markdown links now have a keyboard destination bar: Tab/Shift+Tab cycle the selected
block's links and Alt+Enter opens the displayed URL, while Enter retains copy
behavior. Hints follow remapped bindings. Selection is keyed to result identity,
not the changing filtered row index; stale indices clamp and empty blocks do not
open anything. IME composition suspends link actions. Native compilation plus
focused navigation/scoping/remapping tests pass. Native visual and interaction
acceptance remain pending.

Markdown reading blocks now render clickable HTTP(S) links with underlines and
destination tooltips, including inline emphasis and table cells. Opening a link
uses the default browser and preserves the preview; non-web/relative links remain
plain text. Link spans use the same Unicode excerpt projection and search styling
as other text. Stored destinations are capped per URL, block and document to
prevent repeated reference links from amplifying memory use. Thirteen Markdown
checks and the inherited typography check pass. Native visual/click acceptance and
keyboard link navigation remain pending while the Mac is locked.

Event replay now emits `surface.bell` independently of terminal focus; surface
snapshots expose `bell_revision` as a decimal string. Burst delivery may coalesce
several bells into one event, while the counter retains their count. Native smoke
passes three real shell BELs, filtered replay, no duplicate idle events and exclusion
of OSC terminator BEL bytes. Existing unfocused-only attention behavior is unchanged.

Shell integration CI run 44968164363 passed on Ubuntu, including the newly added
mise-managed fish 4.9.3 startup/native-marker checks. Linux graphical/PTY acceptance
and older fish fallback coverage remain pending.

Fish now defers to built-in directory/prompt integration in 3.7+ and respects the
native mark-prompt feature switch, avoiding duplicate OSC 7/133 hooks. Pinned fish
4.9.3 was installed through mise; startup tests and a real macOS PTY test pass for
Unicode cwd and prompt navigation. Fixed the existing startup fixture's collision
with a fish-created configuration directory. CI now runs the lightweight fish
startup test. Older-version fallback and Linux runtime evidence remain pending.

GitHub native/browser run 44967752333 (bfc37cf) passed, including the new AppKit
quit routing probe. Manual native confirmation UI acceptance remains pending.

`rmuxctl pipe-pane --json` captures child stdout/stderr and exit status as a JSON
object. Two concurrent readers drain output while the captured terminal snapshot
is written to stdin, retaining at most 1 MiB per output stream and reporting
truncation. Default inherited output is unchanged. All 28 CLI tests pass, including
a child that fills both streams before reading a 1 MiB input, literal Unicode
stdin, nonzero exit status and keeping local execution options off the RPC wire.

The macOS CI job now runs the production AppKit quit bridge in an isolated probe,
checking cancellation, retry and final approval under a ten-second process timeout.
It shares the existing browser-feature build cache. The native-only quit module
is excluded from shared WASM triggers; the compiler-dependency guard continues to
check that exclusion. Theme commit 38431ed passed both native/browser and Shared
WASM CI. The hosted probe passed in run 44967752333.

macOS AppKit termination requests now route through the existing quit confirmation
and bounded session/process cleanup. Repeated requests coalesce. The original
request is canceled and reissued after approval; deferred AppKit termination was
rejected after a real probe demonstrated it stalls GPUI's queued handler. A native
AppKit probe now passes cancellation, a second request and approved termination;
the native event smoke confirms hook installation and ordinary app cleanup.
Manual Dock/menu confirmation acceptance, logout/shutdown continuation and other
platform-originated quit routing remain pending. This does not intercept Force Quit.

Added Catppuccin Mocha and Latte presets to Settings and palette commands, with
ANSI, foreground/background, cursor and selection colors from the pinned official
Ghostty port. Bundled themes parse once into immutable cached data; no filesystem
reads are added to rendering. Explicit/imported colors retain precedence, as do
application cursor overrides. The upstream license is included in release notices.
All 28 preference tests pass, including preset parsing, override precedence and
existing theme behavior. Native screenshot verification remains pending: the Mac
was locked when checked this cycle.

Selection opacity is configurable in Settings and JSON (`selection_opacity`,
0–100 or null for theme default). The control uses 5% steps and an explicit reset;
Save Settings persists it without replacing existing selection colors. The
preview and terminal share the same fill function. Existing opaque custom colors
and translucent default blue remain unchanged until an override is selected.
Focused tests cover color preservation, bounds, reset and save/read round-trip.
Native visual acceptance remains pending.

PowerShell command completion now reports normalized success/failure from `$?`
without changing the status seen by the original prompt. The paired expression
is evaluated before assignment can reset status. Focused PowerShell 7.5.4 checks
pass for failed commands, successful commands after stale LASTEXITCODE, original
prompt preservation, idempotence and canceled/empty input. Exact native exit-code
reporting remains pending. Bash login auto-injection remains unresolved because
login shells ignore --rcfile; the non-login wrapper is unchanged.

Terminal input-focus transitions now publish `surface.focus_changed`, with current
`input_focused` in `surface.list`. Publication uses the existing deduplicated
focus path, including window activation and focus entering/leaving editors.
Selected tabs and actual keyboard focus remain distinct. Native build and the
isolated event smoke pass, including capability discovery and boolean snapshot
shape. Native focus transition interaction acceptance remains pending.

The shared text-editing WASM artifact from GitHub run 44966016000 (a9f2dde)
is deployed to rmux.app (Cloudflare deployment 45cdcaf4). Live browser checks
verified palette dismissal through Close, Escape and clicking the backdrop,
then terminal typing. Evidence: `target/live-palette-dismissal.jpg`.

Shared text fields now support macOS Control-A/E/B/F/H/D/K editing through the
existing Unicode movement, deletion and undo paths. Control+Option combinations
and active IME composition remain with the input system. Native cargo check and
the local WASM build pass. Browser interaction verified edge movement and
character movement/deletion, including `alpha beta` → `pha bet`, and truncating
`new workspace` to `new` with Control-K. Screenshot:
`target/palette-control-editing.jpg`. Native visual acceptance remains pending; the browser change is deployed as
recorded above. Public downloads are unchanged.

`system.capabilities` now describes the live event API: protocol version, event
kinds, filters, page/retention/wait limits, gap reasons and lack of push streams.
The values share constants with validation and admission; standalone discovery
returns null rather than advertising unavailable live events. The focused RPC
discovery test checks advertised options against request validation without
queuing UI work. Native smoke also checks discovery before event operations.

Event reads now accept a surface ID and a bounded list of event kinds, exposed
as `rmuxctl read-events --surface ID --kind KIND` (repeatable kinds). Replay
cursors advance over skipped records; waiting subscribers wake only on matching
events. Filtering keeps existing retention-gap semantics and scan bounds.
Focused tests cover skipped-page advancement, page limits, unknown/duplicate
kinds and unrelated wake suppression. The native smoke exercises filtered CLI
replay and a filtered socket waiter awakened by another connection.

Reduced unnecessary Shared WASM CI builds by excluding eight additional native
modules: RPC/events, recovery/restore/snapshots, preferences/settings and native
pane ownership. The browser supplies its own hosts for these responsibilities.
A fresh WASM cargo check passes; rustc dependency files confirm all 11 excluded
modules are absent. A CI guard now checks those exclusions against the optimized
artifact's dependency file and fails if a future shared import makes one unsafe.
The guard also passed a negative fixture that adds an excluded dependency.
Shared source edits still trigger builds; no build-speed percentage is claimed.

Event replay now reports terminal title, directory, program-status revision and
progress changes. Repeated shell title/directory values do not generate duplicate
events; local/remote directory updates coalesce within one delivered batch.
Manual tab title changes use the same event. `surface.list` now exposes current
program-status summary and progress so clients can resolve those invalidations.
The isolated native smoke uses a live shell reporting Unicode title/cwd, OSC 7501
status and OSC 9 progress; it checks events against the resulting surface state
and verifies unchanged title/directory reports do not emit again. Native manual
title interaction and cross-platform runtime acceptance remain pending.

Added `rmuxctl read-events --after CURSOR --limit N --wait-ms N`, preserving
opaque cursors exactly and validating page/wait bounds and duplicate flags.
Offline help and Bash/Zsh/Fish option completion expose the command. The native
event smoke now invokes the CLI against the isolated app's socket to confirm
cursor continuation and empty replay through the actual client transport.

`events.read` now supports bounded `wait_ms` long polling (0–3000 ms). An empty
read registers its wake-up in the same UI update, then waits in a separate task
so the shared RPC dispatcher can handle other requests. Up to 16 waiters each
hold one coalesced wake-up; timeout/completion drops the registration, and
disconnected clients retain it for at most the bounded wait. No idle polling is
introduced. Focused coverage checks the cap, coalescing and drop cleanup; the
native smoke exercises timeout and a second connection waking on a notification
without delaying the sending client. Push streams and full event parity remain
open.

Event replay now includes `surface.created` and `surface.closed`. A small
ownership token publishes closure once on explicit close, with a drop fallback;
it moves with the terminal, so tab transfers do not invent new lifetimes. Native
smoke initially exposed delayed GPUI entity release, confirming that entity-drop
notifications alone were insufficient. The revised smoke passes creation,
cross-workspace transfer without false close/create, and closure of both terminal
owners while another workspace remains alive. The journal remains UI-thread-local
and bounded; no process/client lock or timer is added.

Added live `events.read` metadata replay with a 512-record journal, 128-record
page cap and process-scoped cursors. Notification receive/read/dismiss/clear and
process exits publish small records without client IO or terminal-output copies.
Retention and restart gaps are explicit, and future cursors are rejected. Focused
journal/RPC tests pass. `mise exec -- python scripts/smoke-events.py` passed on
macOS against a fresh debug app, covering notification receipt/dismissal replay,
an empty subsequent read, absence of message bodies and restart gaps. This starts
A04; push subscriptions, broader event coverage and slow-client latency/platform
acceptance remain open. No public desktop release was made.

Native settings now expose cursor opacity (0–100%, default 100%) beside cursor
shape, with live application, keyboard adjustment, theme preview and persistence.
Partial opacity blends block cursor text toward the original foreground; zero
skips cursor paint entirely. Five focused cursor tests pass, including validation,
color blending, opacity bounds and disk round-trip. No timer or terminal engine
mutation was added. Native visual acceptance remains pending while the Mac is
locked; this is not yet in the published desktop release or browser settings.

Automatic session checkpoint failures now show a window notice explaining the
recovery risk and how to save elsewhere. Identical errors are suppressed until
a successful write, preventing the notice's own notification from creating an
idle retry loop. Disk work remains on the existing background writer. Four
focused recovery tests pass, including a failed-write/repaired-location retry
that verifies existing data is preserved. Native notice visuals remain pending.
Reconciled W07/S01 ledger gaps with the existing native smoke record: automatic
metadata capture, forced-stop restoration and corrupt-state fallback already
have runtime evidence; cross-platform visual acceptance is still open.

Native pointer selection, Select All, Clear Scrollback and viewport scrolling now
share a bounded ordered interaction queue. Busy parser admission is nonblocking;
frames and selection exports apply pending interactions before reading, so an
immediate Copy sees Select All or the latest completed gesture without requiring
a redraw. Adjacent drags coalesce while press/release and viewport boundaries
remain ordered. Deferred active gestures use the existing selection timer.
Clear Scrollback moved out of the PTY writer to avoid racing newer selection.
Held-parser tests cover immediate copy, later pointer override, 100 coalesced
drags, clear/select ordering, and scroll/input precedence. Desktop cargo check
passes. Native drag/autoscroll UX and cross-platform runtime acceptance remain
pending; these tests do not measure visible input latency.

Native clipboard copies can now cancel during streaming formatting, in addition
to parser-lock waiting. The formatter checks ownership at the first callback and
after each additional 4 KiB of emitted data, avoiding a predicate call per glyph.
Cancellation discards partial output and releases the parser; it does not alter
the selection. Callback panics are contained before returning across the C ABI.
Eight export tests and the parser-contention fixture pass; the explicit timing
fixture remains ignored. The new Unicode mid-stream cancellation case is in CI.
Native pointer-selection admission is now deferred by the shared interaction
queue described above; native gesture acceptance remains open.

Deployed cc15502 from successful GitHub Shared WASM run 44962351505 to
Cloudflare ece559ce. The live rmux.app embed references that source. Verified
Option-Left word motion, Command-Backspace deletion and Close-button focus
restoration in the embedded production demo; screenshot:
`target/live-palette-cc15502.jpg`. Native CI run 44962351552 also succeeded,
including the new queued input/paste/close/scroll checks. Native visual
acceptance remains pending while the Mac is locked; downloads remain v0.1.4.

Shared text fields now use browser platform detection for macOS editing keys
instead of treating the WASM compilation target as a non-Mac OS. Local browser
interaction verified `alpha beta` → Option-Left + `X` → `alpha Xbeta`, then
Command-Backspace → `beta`. The shared palette Close control also restored
terminal input focus. Screenshot: `target/palette-mac-editing.jpg`. This fixes
palette, rename and other shared-field editing rules; native macOS rules are
unchanged. Optimized GitHub artifact deployment and the live browser check are
recorded above.

Native palettes now expose the same visible Close control as the browser demo
through a shared header/button implementation, with vertical padding on the hit
target. Native dismissal uses the existing focus-restoration event; browser
dismissal restores its active terminal. Native windows below 300 px retain the
existing compact header omission and Escape/outside-click dismissal. Both desktop
and WASM cargo checks pass. Native visual inspection was attempted but the Mac is
locked; visual acceptance and deployment of this UI change remain pending.

Recorded a six-run, alternating 16-pane comparison of ffa98af against the retained
e8e486a style-only benchmark binary. Median parsed throughput was 108.71 versus
110.92 MiB/s; response-snapshot p99 was 4.87 versus 5.06 ms. All requests and
shutdown checks completed. Ranges overlap and the comparison bundles multiple
changes; no isolated speedup or GUI-latency claim. Raw binary provenance and all
samples: `docs/benchmarks/pty-post-scroll-mouse.jsonl`; interpretation is in the
performance plan. Native CI for ffa98af is queued behind e1dffb5.

Native mouse dispatch now reads a tracking flag published after each parsed PTY
batch. Local scrolling no longer locks the parser to check reporting; application
mouse events enter the bounded input queue and Ghostty encodes them on its writer.
The worker honors the mode at encoding time; a report whose application has since
disabled tracking is suppressed rather than replayed as unrelated local input.
Three mouse tests pass, including a real PTY that enables SGR reporting, receives
the exact wheel bytes after admission under a held parser lock, then disables
tracking and returns handling to local scrolling. Desktop cargo check passes.
Pointer selection itself still has synchronous parser operations; GUI latency,
mode-publication overhead and cross-platform runtime acceptance remain open.

Native scroll requests no longer wait for the parser lock. Busy requests are
applied before the next coherent frame; same-direction line motion coalesces,
opposite directions retain clamping order, and absolute navigation or later input
supersedes older requests. The queue holds at most 64 requests, reports saturation,
and retains its small allocation across gestures. No background scroll timer or
worker is added. The held-parser fixture verifies nonblocking admission, bounded
queue recovery, viewport edge behavior and typing-after-scroll precedence; CI
includes it. Mouse-report dispatch now uses the published-mode/queued-input path
described above. GUI scrolling latency and cross-platform acceptance remain open.

Explicit terminal shutdown now cancels queued paste before requesting process
termination, even if the session remains owned by UI code. Native and automation
paste admission reject a closing terminal. The retained-session close fixture
holds the parser lock, queues a paste, closes the session, and verifies the paste
permit is released before unlocking; subsequent pastes fail without retaining a
permit. The child then reaps normally. This fixture and the ten paste tests pass
on macOS; CI now includes the focused close fixture as well.

Native paste now reserves the bounded writer queue without acquiring the parser
lock on the UI thread. The writer resolves bracketed-paste mode immediately
before encoding and honors cancellation while waiting for the parser. The 1 MiB
limit, one-paste permit, ordered whole-payload delivery and completion wakeup are
unchanged. Automation retains its explicit busy/no-admission retry contract.
Ten paste tests plus the held-parser admission fixture pass; the concurrent
output fixture now requires native paste admission without any retry loop.
Native clipboard interaction and cross-platform runtime acceptance remain open.

The Shared WASM workflow excludes native `main.rs`, `window_view.rs`, and
`clipboard.rs` from its desktop source trigger. The browser supplies its own
versions of those hosts, so native-only edits no longer queue an unchanged demo
build behind useful work. All other desktop source remains conservatively
watched, and manual dispatch remains available. This reduces unnecessary runs;
it does not claim faster compilation of a changed browser artifact.

Browser split-tab and join-pane commands now use the same transfer cleanup as
dragging a tab: session/output ownership is preserved while source render caches,
selection gestures and held input are released. Splitting reuses `take_tab`;
joining releases every transferred tab, including inactive tabs, and drops the
source tab menu subscriptions. WASM cargo check passed. GitHub Shared WASM run
44960758565 succeeded for aed790b; its artifact is deployed to Cloudflare
53035589 and referenced by rmux.app. A live check moved Terminal 2 with existing
output into a right split, joined it back, and accepted subsequent input with
the original output intact and no stale text observed in that check. Screenshot:
`target/live-split-join-aed790b.jpg`. This is browser sample-session acceptance,
not native platform acceptance or evidence of a real shell in the demo.

Native clipboard exports now stop waiting for the parser when their owning UI
task is canceled (selection changes, history clear, or pane destruction). A
per-request drop guard complements global clipboard ordering without canceling
a newer copy in another pane. Focused tests cover dropping an unpolled task,
cross-request isolation, and cancellation while the parser remains locked.
Formatting also supports cooperative cancellation at streaming checkpoints, as
recorded above; successful complete copies remain bounded to 16 MiB.

The earlier bounded-writer Clear Scrollback implementation was superseded by the
shared interaction queue above, preserving ordering with selection and Copy.
The action's existing redraw notification drives deferred application even on a
quiet terminal; a copy/export also flushes pending history changes. Screen
preservation and clear/select ordering are covered by focused tests. Native
visual acceptance remains pending.

Deployed source 374b4c5 from successful GitHub Shared WASM run 44959331001
to Cloudflare 67b13589. Live rmux.app references it. Verified dragging the last
rmux terminal onto website preserves output, accepts further input and removes
the emptied rmux workspace. Screenshot:
`target/live-workspace-transfer-374b4c5.jpg`. Native snapshot optimizations are
separate from this browser artifact and have not been released in downloads.

Native snapshot extraction also reuses width metadata for consecutive equal
raw cells. Grapheme text is deliberately fetched per cell because packed base
values do not identify combining extensions. Existing snapshot tests and a new
adjacent-acute/grave plus wide-tail regression pass. No extra timing claim for
this follow-up; the recorded A/B run covers style reuse only.

Native snapshot extraction reuses resolved styles/colors for consecutive
identical raw cells within each row, avoiding redundant FFI queries and color
work without heap allocation. Five focused snapshot tests passed, including a
new packed-background/style-transition case. Three alternating before/after
16-pane release runs completed: median snapshot-batch p99 4.77 → 4.20 ms;
throughput approximately unchanged at 102 MiB/s. Small uncontrolled sample,
not a guaranteed speedup or GUI latency result. Details and raw provenance:
performance.md and `benchmarks/pty-row-style-cache.jsonl`.

Refreshed native release PTY pressure measurements after queued-input changes.
Fixed the responder harness to await asynchronous workload shutdown before the
next pane count. All 512 requests completed at 1/4/8/16 panes, output advanced
and shutdown checks passed. At 16 panes: 103.57 MiB/s aggregate flood output,
11.8 µs input-admission p99 and 5.94 ms response-snapshot p99. Raw provenance in
`benchmarks/pty-post-queued-input.jsonl`; scope/limits in performance.md. This
does not measure GUI latency or isolate the mode-publication query's cost.

Browser sidebar rows accept terminal-tab drops from another workspace. Source
tab identity, rename state and destination capacity are checked before transfer;
the existing terminal entity/session moves to the destination's active pane.
Empty destinations gain a pane, and empty source panes/workspaces are removed.
Grouped targets are revealed before selection. Local pointer checks moved the
same terminal twice with output/input continuity, covering empty source cleanup
and nonempty source retention. Browser build passed. Screenshot:
`target/browser-workspace-tab-transfer.jpg`. Not yet deployed.

Deployed source 69d0bfb from successful GitHub Shared WASM run 44958579013
to Cloudflare a417c7f9. Live rmux.app references this artifact. Verified a tab
transfer between split panes retained earlier output, removed the empty source
split and accepted subsequent typing. Screenshot:
`target/live-cross-pane-69d0bfb.jpg`. This publishes cross-pane dragging and
transient geometry cleanup. Native downloads remain v0.1.4.

Cross-pane pointer transfer now releases the moved terminal's rendering caches
and transient input gestures before changing owners. This preserves the engine
and session while rebuilding cursor/selection geometry at the destination.
Browser build and native-VT adapter checks pass; local transfer retained output,
removed the empty source pane and accepted subsequent input. Screenshot:
`target/browser-transfer-cache-reset.jpg`. The earlier possible source-pane
text artifact is not yet conclusively reproduced or attributed to this change.

Browser tab dragging now transfers existing terminal entities between panes in
the same workspace, inserting before a target tab or appending in empty strip
space. The workspace validates source/destination identities, rename state and
the 16-tab destination limit before mutation. Empty source panes are removed;
remaining source tabs stay alive. Local pointer checks covered sole-tab pane
cleanup and multi-tab source retention, with earlier output and subsequent
typing preserved. Browser build passed. Screenshot:
`target/browser-cross-pane-tab.jpg`. Not yet deployed; dragging between browser
workspaces remains pending.

Deployed palette dismissal and tab-reordering source ef467b6 from successful
GitHub Shared WASM run 44957843451 to Cloudflare 7ea03ba2. Live rmux.app uses
the new artifact. Verified the embedded palette's visible Close button and
subsequent terminal typing. Screenshot: `target/live-palette-close-ef467b6.jpg`.

Fixed missing browser palette dismissal paths: a visible Close button,
backdrop click dismissal and command-palette shortcut toggling now restore
terminal focus. Panel clicks do not dismiss it. Escape already worked in the
standalone live demo, but pointer users had no explicit dismissal control.
Browser build passed; local Close/outside-click/shortcut checks verified
subsequent terminal typing. Screenshot: `target/palette-close-controls.jpg`.

Browser terminal tabs support pointer reordering within their pane. Dropping
on another tab inserts before it; dropping in the strip's empty area appends.
Native and browser hosts use the same rmux-core insertion rule. Stable entity
IDs revalidate source/target and retain terminal contents, custom titles and
input focus. Local browser gestures verified both directions and subsequent
typing with prior output intact (`target/browser-tab-drop.jpg`). Browser build
and the existing native insertion-direction test passed. Cross-pane pointer
transfer in the browser is still pending; this change is not yet deployed.

Deployed source cab241d from successful GitHub Shared WASM run 44956760426
to Cloudflare deployment 1741f7d9. The live rmux.app page references this
artifact; a live pointer gesture reordered website above rmux and retained
terminal focus/selection. Screenshot: `target/live-workspace-drop-cab241d.jpg`.
This includes shared workspace/group drag behavior, bundled drag-preview fonts
and drop eligibility feedback. Experimental browser run 44956760421 also
passed. Native downloadable releases remain v0.1.4.

Sidebar drop highlights now consult a shared, non-mutating eligibility check
in both native and browser hosts. Self/stale drops and rejected pin-tier moves
no longer advertise a valid destination. Native external tab-drop validation
is retained. The same eligibility check guards the actual model mutation;
focused group/pin/identity tests and both host compile checks pass.

Browser workspace rows now share desktop's sidebar drop model: reorder rows,
move members into/out of groups, and move group anchors with membership intact.
Rejected top-level pin-tier changes leave membership unchanged. Extracted the
model into rmux-core and retained native cross-window tab-transfer handling.
Fixed the browser drag-preview panic caused by its detached view inheriting an
unavailable system font; it now uses bundled Inter and optional font fallbacks.
Focused group/pin/identity test, browser build and native cargo check passed.
Local browser gestures verified reorder, joining a group and leaving it again;
the palette remained usable after dragging. Screenshot:
`target/browser-workspace-drop.jpg`. Not yet deployed; cross-platform native
drag acceptance remains pending.

Deployed GitHub Shared WASM run 44955219219, source 5483941, to rmux.app
(Cloudflare 0bb664a7). This adds the shared workspace/group context menus,
palette access and embedded group icon to the previously deployed metadata
controls. Optimized build and font/startup/engine/search checks passed. Live
check verified the root page points to the correct source artifact, group
creation renders the folder SVG and right-click opens group actions. Screenshot:
`target/live-demo-workspace-menu-5483941.jpg`. User guide updated. Native
keyboard source 212d230 passed CI run 44955508491; native downloads remain
v0.1.4, so those keyboard changes are not released.

Added/run the release queued-keyboard pressure benchmark for native text and
report-all modes with 1/4/8/16 output-flooding PTYs. All eight cases admitted
128 samples, advanced output and cleaned up children. At 16 panes p99 native
text routing was 0.333 µs, report-all text admission 0.709 µs and report-all Up
admission 7.666 µs. Raw data/provenance: benchmarks/pty-queued-keyboard.jsonl;
method and limits in performance.md. This measures Session dispatch/admission,
not writer completion, native IME or presentation, and is not a matched
throughput comparison. Reproduce with `mise run bench-queued-keyboard`.

Native plain-key dispatch no longer acquires the terminal parser lock. The PTY
reader publishes negotiated report-all mode after each parsed output batch;
ordinary text stays on native text/IME handling, while report-all keys queue
committed text for encoding on the sole writer thread. If mode is disabled
before encoding, queued committed text falls back to normal bytes rather than
being discarded. Queue payloads remain bounded (128-byte key name/4 KiB text).
Eight queued-path tests, eight keyboard protocol tests and the parser-contention
fixture pass. A real PTY fixture verifies published mode, shifted A protocol
bytes, and mode reset. Native desktop check passes. This proves the lock
avoidance/protocol paths, not native visual IME acceptance or latency targets.
Not released yet.

Replaced the missing default workspace-group text glyph with an embedded folder
SVG. Native and browser hosts share the icon slot and selection contrast; custom
workspace/group symbols still use their existing text values. A fresh browser
group renders the icon without requesting optional fonts. Screenshot:
`target/browser-group-svg.jpg`. WASM build and native check pass. Shared WASM
CI now also triggers for desktop asset-only edits so embedded SVG changes cannot
be missed. Not deployed yet.

Browser sidebar workspace/group context menus now share native labels, command
order, frame/row geometry and action routing. Right-click captures the target
identity; group actions reject stale group membership. Menus support keyboard
navigation/typeahead, scrolling, pointer selection, outside-click/blur/Escape
dismissal and return focus. Show Workspace Actions is available in the browser
palette. UI checks cover palette opening, typeahead pinning, inactive-target
description editing, group menu content and terminal input after Escape.
Screenshot: `target/browser-workspace-menu.jpg`. WASM build and native desktop
check pass; not deployed. The default group text glyph appears missing in the
browser font and needs replacement with an embedded icon in a follow-up.

Fixed held-key ownership on native focus queue rejection. The tracker now
clears a release batch only after the bounded writer queue accepts it; rejected
keys remain available to key-up or the next focus transition. Focus gain also
retries leftover releases before the focus-in report. This is event-driven
recovery, not a promise of delivery while the queue remains full. Three core
keyboard tests (including full/closed queue rejection, shifted alias ownership
and successful retry) and the terminal release-before-focus-in/out encoding
test pass; native desktop check passes. Native CI now includes core keyboard
tests and triggers for rmux-core changes. The previous shared UI revision
ee11e6a also passed native CI run 44954176117. Native fix is not released yet.

Deployed GitHub Shared WASM run 44954176105, source ee11e6a, to rmux.app
(Cloudflare deployment 0d57063b). This includes workspace descriptions,
collapsed metadata editor reveal, workspace/group colors and icons, shared
preset pickers and on-demand sidebar Unicode fonts. GitHub optimized build,
font/startup/engine/search checks passed before import. Live browser checks
verified the site links the ee11e6a artifact, color preset saving and icon
picker font rendering. Screenshot: `target/live-demo-metadata-ee11e6a.jpg`.
Updated the user guide and W07 feature ledger; native downloads remain v0.1.4.
Website source remains local/off GitHub.

Color/icon preset pickers now use one shared desktop/browser module, including
choices, selection outlines, tooltips, sizing and draft-only click behavior.
Browser metadata edits notify their host so selection outlines follow typing
and preset clicks. Icon picker fonts load on demand; the Notes preset uses the
bundled-font-supported pencil (✏) instead of the missing lower-right pencil
glyph (✎). Existing custom icon values are preserved. Browser checks cover
color selection/save, icon selection/cancel, Notes rendering/save. Screenshots:
`target/browser-color-presets.jpg`, `target/browser-icon-presets.jpg`.
WASM build and native desktop check pass. Not deployed yet.

Browser workspace/group icon editing now shares native actions and the existing
Unicode grapheme validator (one symbol, up to 64 bytes). Empty input clears;
invalid input keeps the editor open with guidance. Group rows use the native
default symbol when no custom icon is set. Metadata edits now request optional
fonts on demand, and sidebar text uses the same CJK/emoji fallback list as the
terminal. This fixes missing glyphs for sidebar metadata without eagerly
loading optional fonts. UI checks cover invalid multi-character input, a
lightning symbol, clearing and a joined woman-technologist group emoji.
Screenshot: `target/browser-workspace-icons.jpg`. WASM build and native check
pass; this change is not deployed. Preset pickers and context-menu parity are
still separate browser gaps.

Browser workspace/group colors now route through the shared desktop actions
and use the same six-digit hex parser. Sidebar markers match native geometry;
empty input clears the color, invalid input retains the draft with guidance.
Browser metadata now has named description/color fields rather than another
tuple slot. UI checks cover invalid input, correction, workspace clearing,
group color and independent workspace color. Screenshot:
`target/browser-workspace-colors.jpg`. WASM build and native desktop check pass.
This browser parity increment is not deployed yet; native color behavior is
unchanged apart from shared routing/validation.

Native and browser metadata editors now reveal their target when its group is
collapsed and scroll the active workspace into view. Reopening an existing
editor also restores its visibility without replacing the draft. Browser UI
verification: create group and child, collapse with child active, invoke Edit
Workspace Description, observe expanded group and focused editor, save and
observe description on the child. Screenshot: `target/browser-metadata-reveal.jpg`.
WASM build and native desktop check pass; native visual verification remains
outstanding. Shared WASM run 44953302434 for the preceding description change
was still in progress at observation; neither change is deployed yet.

Browser workspace descriptions now use the shared desktop command action,
TextField editor and description normalization (trimmed, control-free, 256
characters). The sidebar uses native selected/unselected description colors.
Local WASM build and native desktop check pass. Browser UI checks cover saving,
cancelling and clearing a description; screenshot:
`target/browser-workspace-description.jpg`. Metadata remains session-local in
the browser and resets on reload. This change is not yet deployed.

Rebuilt native desktop/CLI and ran the live-shell RPC fixture at 59d83de with
queued focus/key input. All checks pass: pane swap/zoom guards, cross-window tab
and workspace transfers, same/cross-window merge, retained nested layouts,
seven live shell PIDs and Unicode variables, targeted input, stale-ID rejection,
and fixture-child cleanup. Evidence and binary hash:
benchmarks/native-queued-input-live-rpc.json. This is macOS debug lifecycle
acceptance only, not visual, OS activation or latency acceptance. Native CI for
cbbe959 also passed; newer native revisions were still building at observation.

The existing macOS terminal library suite passes after queued keyboard changes:
118 passed, four manual benchmarks ignored, 2.66 seconds execution. Added a real
PTY protocol fixture that enables focus reports and Kitty event types, queues
focus-in → Shift+Up press → release → focus-out, and checks the exact bytes read
by a raw-mode child plus clean process exit. That additional test passes in
0.03 seconds and is included in CI's queued-input filter. This complements the
parser-contention and in-memory writer checks; native UI/IME and other platforms
remain outside this evidence.

Native non-text/navigation and Ctrl/Alt key presses now enqueue encoding on the
PTY writer. Press/repeat/release order follows the bounded queue; unknown keys
return unhandled without locking, exited processes remain rejected, and accepted
input retains opportunistic follow-output behavior. Plain unmodified text keeps
the existing negotiated report-all decision so native text/IME is not swallowed
or doubled. Tests pass for exact Kitty press/repeat/release ordering and Ctrl+U
admission during parser contention; desktop check passes. The first contention
fixture used unsupported F35 and failed; replacing it with canonical Ctrl+U
exercises a real accepted control key without contaminating the responder line.
CI covers these queued-input regressions. Plain-text protocol decisions and mouse
input remain separate latency work. Not released.

Native ordinary key-up reporting now uses a bounded writer message instead of
synchronously acquiring the parser. It preserves the original pressed-key name,
modifiers and writer ordering, holds a weak terminal reference, and never follows
output/changes the viewport. Rejected releases remain tracked for later focus-loss
cleanup, with a visible error. Tests pass for exact Kitty modifier/release bytes,
ordering between ordinary input messages, closed-engine cleanup and admission
under a held parser lock. Existing queued-focus tests and desktop check pass.
CI's queued-input filter includes the new regression. Press/text handling remains
separate work; no end-to-end keyboard latency claim. Not released.

Published successful GitHub demo run 44951581189 (fab099c) to rmux.app,
Cloudflare deployment 8f4bd996. Hidden workspace/tab/zoom snapshot reclamation is
now live. Embedded production smoke verified both split outputs survive zoom,
workspace round-trip and layout restoration. Screenshot:
target/live-demo-reclamation-fab099c.jpg. Added the focused queued-input and
parser-contention tests to the existing native browser CI job; local results
were recorded in the preceding implementation cycles. Desktop downloads remain
v0.1.4; native focus queue changes are not released.

Added a queued-focus release benchmark alongside the retained synchronous mode.
At 1/4/8/16 output-flooding panes, focus-in admission p99 measured
2.875/4.875/7.666/6.792 µs; all admissions succeeded and fixture cleanup completed.
Status-detail probes record parser-busy misses instead of waiting. Raw results
and hashes are in benchmarks/pty-queued-focus.jsonl; performance.md distinguishes
queue admission from deferred delivery and full GPUI/display latency.

Native focus loss now queues held-key releases and focus reporting as one bounded
writer message. Releases are encoded before focus loss under the worker's parser
lock; UI focus changes no longer synchronously encode each held key. Payloads
hold weak engine references and enforce key-count/name bounds. One queue slot
admits the whole transition; full/closed queues report failure rather than partly
enqueuing it. Focused tests pass for Kitty release ordering, busy-parser admission,
focus-only notification/order and closed-engine cleanup; desktop check passes.
This does not change ordinary key-up delivery or establish end-to-end latency.
Not released.

Native TerminalView focus reporting and program-status acknowledgement now enqueue
on the existing bounded PTY writer instead of locking the parser on the UI thread.
Focus messages preserve writer order and hold weak engine references. Completion
wakes the existing nonblocking metadata observer even when focus reporting emits
no bytes; the redundant synchronous status read was removed from set_input_focus.
Queue admission failure remains visible through the terminal error path. Focused
macOS tests pass for busy-parser admission, report ordering, disabled-report
wakeups and closed-engine cleanup; desktop check passes. No full focus-latency
claim: held-key release and other focus-handler work still need separate review.
Not released.

Published GitHub demo run 44951200648 (05673c6) to rmux.app, Cloudflare deployment
5a84225c. Includes group closure, terminal font controls/inheritance and measured
cell-metric reuse. Production embedded smoke verified zoomed text inherited by a
new split, two-workspace group closure, and surviving split output/focus.
Screenshot: target/live-demo-fonts-05673c6.jpg. Updated the browser user guide.
Hidden snapshot/zoom reclamation changes remain in newer CI builds; desktop
release downloads are unchanged.

Browser panes hidden by zoom now release their terminal rendering caches and
snapshots using the same reclamation path as hidden workspaces/tabs. The host
tracks zoom visibility transitions, including directional focus changes while
zoomed, so ordinary output redraws do not repeatedly reclaim caches. Sessions
remain alive. WASM build passes; browser smoke verified zoom, Focus Left while
zoomed, restoring both panes with retained output, and subsequent terminal input.
Screenshot: target/browser-zoom-reclamation.jpg. Not deployed.

Added and ran a render-reclamation benchmark at 1/4/8/16 panes. Full engine/JSON
reconstruction medians are 0.325/1.285/2.601/5.264 ms; engine linear-memory capacity
is unchanged over 120 cycles per pane count. The benchmark validates full row
counts and rejects unknown scenario names. Raw results and scope limits are in
performance.md and benchmarks/browser-render-reclamation.jsonl. This is engine
boundary cost, not browser switching latency or a quantified live-memory saving.

Hidden browser terminals now release Rust cell snapshots and the Ghostty render
adapter, in addition to shaped rows/cursor caches. Terminal state, history, input
draft and selection remain owned by the session. A released frame is rebuilt
fully before paint or the next snapshot update, avoiding sparse patches against
empty rows. The shared desktop frame-release helper preserves viewport dimensions.
Adapter checks pass for repeated/idempotent reclamation, full row/selection/cursor
restoration and subsequent output. WASM build passes; browser smoke verifies tab
and workspace round trips with unchanged output and a draft that executes after
return. Screenshot: target/browser-frame-reclamation.jpg. Allocations are freed
for reuse; no browser RSS or linear-memory shrink claim. Not deployed.

Browser terminal rendering now reuses measured cell metrics across ordinary
redraws, pane resizing and workspace switches. The small cached value invalidates
on terminal font-size changes or optional-font registration; glyph/row caches
still release when a terminal is hidden. This removes repeated font resolution,
advance, ascent and descent queries from unchanged redraws. WASM build passes;
reviewed both invalidation paths against font loading and zoom. No runtime speed
or memory improvement is claimed without measurement. Not deployed.

Published GitHub Shared WASM demo run 44950500137 (a4740dc) to rmux.app via
Cloudflare deployment b179207d. The artifact includes workspace/group rename and
workspace pinning. Production embedded-demo smoke verified Cmd+Shift+R rename,
palette pin/reordering, and terminal input. Screenshot:
target/live-demo-workspaces-a4740dc.jpg. Group closure and terminal font controls
remain in newer GitHub builds; native downloads remain v0.1.4. Website source
stays outside GitHub.

Browser new tabs and splits now inherit the active source terminal's font size
at creation, matching desktop behavior. Existing terminals retain independent
sizes; initial workspaces still use the default. The constructor applies shared
CellMetrics bounds. WASM build passes; browser smoke verified a zoomed new tab,
matching split size, and resetting only the new split. Screenshot:
target/browser-font-inheritance.jpg. Not deployed.

Browser terminal font increase/decrease/reset now share native terminal action
routing and CellMetrics bounds (8–40). Palette commands and Cmd+=/Cmd+-/Cmd+0
(Ctrl on other platforms) update only the focused terminal. Font changes release
row/cursor caches and stale selection geometry; the existing resize path reflows
output and preserves the sample input draft. Reset uses browser default 14;
native reset still uses configured preferences. Native check and WASM build pass.
Browser smoke verified palette increase, shortcut increase/decrease/reset, draft
execution after reset, and retained font/output after workspace switching.
Screenshot: target/browser-font-zoom.jpg. Not deployed; new terminals currently
start at the default size rather than inheriting the current terminal's zoom.

Browser Close Workspace Group now shares native action routing and captured-member
resolution. Each member is resolved immediately before removal; moved members and
unrelated workspaces are preserved. Native confirmation and process-close policy
remain unchanged; browser sample sessions close directly. A focused model test
passes for moved-member exclusion. Native check and WASM build pass. Browser smoke
verified two-member closure, unrelated workspace survival, final-group empty state,
and Cmd+N recovery with working input. Screenshot: target/browser-close-group.jpg.
Not yet deployed.

Native browser CI now saves compiled caches per source revision, preferring the
latest cache with matching dependency/tool pins. Previously its immutable cache
key prevented successful later builds from updating compiled application code.
Exact reruns skip saving an existing cache. This follows the shared demo's cache
strategy; speed improvement has not yet been measured. Demo run 44950500137 is
queued behind the still-running 44950076655; workspace rename/pin changes are
not yet published to the site.

Browser workspace pinning now uses the native shared action route and core
ordering/navigation rules. Each browser workspace owns its pin state; sidebar
badges match desktop group/workspace semantics. Palette pin/unpin preserves the
active terminal, and numbered navigation follows the pinned order. Native check
and WASM build pass. Browser smoke verified pin, unpin, Cmd+2/Cmd+1 navigation
and retained output. Screenshot: target/browser-workspace-pin.jpg. Not deployed;
pin state, like other browser workspace state, resets on reload.

Browser workspace groups can now be renamed through Rename Workspace Group in
the palette or by double-clicking the group header. Native/browser group rename
actions share routing and the existing inline editor captures the group identity.
Group names remain separate from the anchor workspace's name. Native compile and
WASM build pass; browser smoke verified palette save, unchanged workspace header,
and double-click/Escape cancellation. Screenshot: target/browser-group-rename.jpg.
Not deployed; browser reload persistence remains absent.

Browser workspaces can now be renamed through the palette, Cmd+Shift+R
(Ctrl+Shift+R elsewhere), or sidebar double-click. The inline editor uses the
shared TextField; both hosts share RenameWorkspace routing and bounded name
normalization. Commit resolves the captured workspace identity and updates the
sidebar and workspace header without replacing pane/terminal entities. Escape
or switching workspaces cancels editing. Native compile and WASM build pass.
Browser smoke verified shortcut save, double-click/Escape cancellation and
retained terminal output after switching away and back. Screenshot:
target/browser-workspace-rename.jpg. Browser reload persistence remains absent.
Not deployed.

Published GitHub demo run 44949065490 (11fe7bc) to rmux.app, Cloudflare deployment
93090bbf. Optimized build and all font/startup/clipboard/engine/search checks passed.
Production embedded-demo smoke verified palette terminal-menu access, text-only
menu paste and Shift+F10 tab-menu shortcut hints. Added a concise browser-demo
section to the user guide. Screenshot: target/live-demo-menus-11fe7bc.jpg.
Website source remains local/off GitHub; native downloads remain v0.1.4.

Menu paste now uses a text-only browser clipboard adapter. Inspection showed
GPUI's general reader also fetches supported image payloads, which a sample text
terminal does not need. The adapter starts readText under user activation and
checks the session's 4 KiB UTF-8 budget before transferring text into WASM; JS
still receives the browser's initial text string. Byte-length measurement is
bounded by the preliminary string-length check. Focused tests cover immediate
invocation, Unicode limits, no image reads, missing API and rejected permission.
WASM build passes; browser menu paste succeeded and a 4097-byte clipboard showed
the limit message without changing the terminal draft. Screenshot:
target/browser-bounded-clipboard.jpg. Not deployed.

Browser terminal menus now include Paste via GPUI's asynchronous clipboard API.
The read begins during user activation, stays bound to the originating terminal,
and permits one pending read per terminal. Results use the existing Ghostty
encoding and transactional bounded sample-paste handler; unavailable clipboard
access produces a keyboard-paste fallback message. The task is owned by the
terminal entity. WASM build passes. Browser smoke verified menu type-ahead/Enter
pasted two lines, executed the complete command, and retained the final draft
across workspace switching for later Enter. Screenshot: target/browser-menu-paste.jpg.
Permission-denial and terminal-close-during-prompt UI cases remain unverified.
Not deployed.

Browser terminals now expose a right-click menu and Terminal Actions palette
entry for Copy, Copy with Line Breaks, Select All, Find and Clear Scrollback.
The menu shares desktop spacing/presentation and navigation/type-ahead, resolves
shortcut hints at opening, preserves the selection, and disables copy without
selected text. Actions restore terminal focus and use existing dispatch handlers.
WASM build passes. Browser smoke verified disabled copy, Enter selecting all,
palette invocation, enabled Copy and exact terminal text reaching the clipboard.
Screenshot: target/browser-terminal-context-menu.jpg. Menu Paste and directory/
link service adapters remain open; keyboard clipboard paste remains available.
Not deployed.

Copy with Line Breaks is now available in the browser palette through shared
terminal action routing. Normal copy still unwraps soft wraps; the explicit
action preserves visual row breaks, both with the existing 1 MiB copy limit.
Native configured copy policy remains unchanged. Native compile, WASM build and
real-engine adapter tests pass. Browser clipboard comparison produced six logical
lines versus ten wrapped lines with identical content after removing newlines.
Screenshot: target/browser-copy-line-breaks.jpg. Not deployed.

Copy, Select All and Clear Scrollback now bind through a shared terminal action
host on desktop and browser. The browser palette also uses the shared action
factory and resolves Select All's actual shortcut for its hint. Host services
retain clipboard error reporting, native asynchronous/cancellation behavior and
browser frame/search refresh. Native and WASM compile checks pass; existing
browser history/clipboard interaction evidence predates this dispatch extraction.
No presentation change or deployment.

Browser terminal Select All and Clear Scrollback now use the pinned Ghostty
engine APIs. Both are palette actions; Select All uses Cmd+A on macOS and
Ctrl+Shift+A elsewhere while text fields retain their own binding. Clear history
resets selection, preserves the screen and parser state, restores the history
budget and refreshes find state. Engine integration checks cover an incomplete
escape sequence, preserved visible text and resumed history retention. WASM build
passes. Browser smoke generated 80 log lines, cleared history from the palette,
selected/copied all, and confirmed only the retained visible lines 51–80 remained.
Screenshot: target/browser-clear-scrollback-selection.jpg. Not deployed.

Desktop terminal menus, desktop tab menus and browser tab menus now use the same
keyboard navigation resolver. Up/Down wrap through enabled items, Home/End pick
enabled endpoints, and stale selections are bounded before navigation. Empty or
entirely disabled menus return no target. Two focused navigation/type-ahead tests
pass with the native test build; WASM compile passes. This consolidates behavior
without changing menu presentation. No deployment.

Tab menus now open through shared ShowTabMenu action routing on both hosts.
The browser exposes Show Tab Actions in its palette and Shift+F10 from terminal
focus, anchored below the tab strip. Menu shortcut hints are resolved once at
opening against the originating terminal's actual keymap. Native compile and
WASM build pass; browser smoke verified Shift+F10, palette invocation and the
New Tab / Move Tab / Close Tab labels. Screenshot:
target/browser-tab-menu-shortcuts.jpg. Not deployed.

Browser tabs now have a context menu for new/rename, movement, close, close others
and close right. It shares desktop menu frame/row presentation, prefix type-ahead
and existing tab command routing. Right-click selects and captures the terminal
identity; activation revalidates ownership and availability. Escape, outside click
and blur dismiss; keyboard navigation skips disabled items. Browser type-ahead
uses web-time rather than unsupported WASM std::time::Instant. Native compile
and WASM build pass. Browser smoke passed right-click selection of an inactive
tab, type-ahead rename, End/Enter close-right and Escape restoring terminal input.
Screenshot: target/browser-tab-context-menu.jpg. Native-only directory/process
actions and shortcut hint labels are not yet included in the browser menu.
Not deployed.

Extended the native live-RPC fixture with pane-swap identity checks after the
shared pane-state migration. The fresh debug build and live-shell run pass:
RPC swap rejects a zoomed workspace without changing zoom/layout; after unzoom,
swap preserves surface-to-pane ownership and process IDs, and a second swap
restores the layout. Existing same/cross-window transfer and merge checks also
pass with live shell state retained. The initial test incorrectly expected RPC
swap to operate while zoomed; inspection confirmed its existing deliberate guard,
so the assertion now checks that contract. This verifies RPC state behavior, not
native pointer/keyboard presentation or cross-platform acceptance.

Published the optimized GitHub artifact from run 44947179213 (376e37b) to
Cloudflare deployment f2dcd3d8 at rmux.app. Build, font/startup/engine/search
checks passed. Production embedded-demo clipboard smoke verified a complete
pasted command executes and an unfinished second command stays editable.
The artifact includes shared pane ownership and recent tab rename/bulk-close/title
polish as well as sample-session extraction and multiline paste. Website source
remains local/off GitHub; native downloads remain v0.1.4.

Fixed sample-session multiline paste being silently discarded after Ghostty
encoding. The sample service now consumes CR/LF/CRLF-separated commands in order,
preserves an unfinished final draft and commits only when the whole paste
succeeds. Input is bounded to 4 KiB and generated output to 1 MiB; unsupported
control characters produce a visible error. Three focused sample tests pass,
including newline normalization and rollback after output overflow; WASM build
passes. Local browser clipboard smoke passed: pasting two complete echo commands
and a final draft produced the expected two responses, switching workspaces kept
the other session clean, and switching back retained the draft for Enter to run.
Deployment remains pending. This is
unbracketed sample-session behavior, not real-shell bracketed-paste support.

Browser sample command interpretation now lives in a rendering-independent
SampleSession service in rmux-core. The terminal view forwards input and renders
returned output; it no longer implements command dispatch, echo, interrupt or
draft editing itself. Existing bounded grapheme-aware input is retained. Focused
tests cover independent session drafts, interrupt, unsupported input, bounded log
output and input limits; WASM compile passes. This prepares the session boundary
but does not add a real shell or complete the desktop/browser backend interface.
No visual change or deployment.

Bulk tab-close target capture now belongs to the shared Tabs model. Both hosts
provide session identities and completion status; the model applies the same
scope and active-tab exclusion before any native confirmation delay. Native
process guards and confirmation remain in place. Five focused tab tests pass,
including captured identities surviving reorder, new tabs and selection changes;
native and WASM compile checks pass. No presentation change or deployment.

Measured startup readiness of the installed GitHub-built v0.1.4 release with its
bundled CLI and the isolated fixture shell. Three fresh processes exited normally;
median socket / terminal registration / readable shell output were
227.7 / 231.4 / 234.7 ms. Raw samples and binary hash are recorded in
docs/benchmarks/startup-release-v0.1.4.jsonl, with method and limits in
performance.md. This is a release readiness baseline, not first-frame latency or
controlled cold-start acceptance, and does not establish a speedup over debug.

Added optional native startup milestone measurement to the split harness:
socket availability, first terminal registration and parsed fixture shell output.
A fresh debug build and three isolated one-pane runs completed normally. Median
milestones were 228.5 / 232.3 / 235.7 ms; first-run shell output was 736.7 ms.
Raw samples and binary hash: docs/benchmarks/startup-current-debug.jsonl.
The 20 ms socket poll and CLI/RPC overhead are included. These are not release,
cold/warm, window-presentation or input-to-display acceptance results. Method and
limits are recorded in performance.md. No deployment/product behavior change.

Long browser tab names now use desktop's shared title view: 48–220 px width,
truncation and full-title tooltip. Native keeps its existing richer tooltip text;
browser uses the full custom/automatic title. Rename completion requests active
tab visibility. Native check and WASM build pass. Browser smoke verified a long
custom title truncates while its close button and neighboring tab remain visible.
Screenshot: target/shared-tab-title-width.jpg. Tooltip hover and overflowed-strip
scroll positioning were not separately exercised. Not deployed.

Close Other Tabs and Close Tabs to the Right are now available in the browser
palette through shared action routing. Both hosts share close-scope selection and
identity-based target revalidation; native process confirmation and prompt guards
remain in the native host. Existing focused tests passed for reordered/right-side
targets, newly opened tabs surviving a pending request, and finished-tab scope.
Native test build and WASM build pass. Browser smoke verified four tabs reduced
to the selected tab and its left neighbor, then to the selected tab alone, with
its sample output retained. Screenshot: target/shared-bulk-tab-close.jpg. Native
prompt UI was not exercised. Not deployed; full app/session parity remains open.

Browser tabs can be renamed through the palette or double-click, using shared
RenameTab action routing, TextField/editor presentation and bounded title
normalization. Empty input restores the sample automatic title; Escape cancels.
Custom titles live on terminal entities and survive tab-to-pane transfer. Native
check and WASM build pass. Browser smoke verified palette save, double-click
cancel and moving the renamed terminal to a new split with its title intact.
Screenshot: target/shared-tab-rename.jpg. Not deployed. The actual shared app
entry point and session-service abstraction remain open.

Pane swap target resolution and commit now belong to shared layout State. Native
commands/automation use the shared commit while retaining traversal-order view
updates; browser uses the shared directional transition. Rejected/stale swaps
preserve zoom, directional swaps are unavailable while zoomed, and next-pane
swaps reveal the layout without replacing session identity. Ten focused pane
tests and native/WASM checks pass. No UI runtime check or deployment this cycle.
A code audit found existing weak search task references, blur cancellation of
selection timers and TerminalEngine Drop cleanup; this is not runtime evidence
of leak freedom. Whole-app lifecycle memory remains unverified.

Extended the browser engine memory harness to five output and search/copy
create/write/close cycles at 1/4/16 panes, including destruction with live search
handles and explicit search closure. Recorded source and WASM hash with raw data
in docs/benchmarks/browser-engine-memory-search-copy.jsonl. All cycles ended with
zero registered sessions; retained linear capacity plateaued by cycle three.
At 16 panes the fifth-close capacities were 38.8125 MiB output-only and
69.8125 MiB search/copy. These are retained WASM capacities, not live allocation,
UI/process memory or a leak-free claim. Method and limits are in performance.md;
whole-app lifecycle memory and latency acceptance remain open. No product change
or deployment in this measurement cycle.

Pane State now commits insertion/removal for both native and browser hosts,
revealing the layout on success and preserving zoom/tree for rejected or stale
requests. Native split creation/tab-to-pane transfer and ownership removal use
these methods; browser split, tab-break, join and empty-pane callbacks do too.
The browser tab-break rollback restores its previous zoom target if transfer
cannot complete. Nine focused pane tests pass, including nested visual ordinals,
duplicate/stale insertion, stale removal, last-pane removal and reinsertion.
Native check and WASM build pass. Browser smoke passed zoomed nested-pane closure,
last-terminal closure, Cmd+T recovery and sample input. Screenshot:
target/shared-pane-ownership.jpg. Not deployed; the previous native live-session
smoke predates this transition change and was not repeated. Shared app/session
entry-point consolidation and platform/performance acceptance remain open.

Desktop Workspace and browser now own the same pane layout State: split tree,
explicit zoom target, zoom/focus transition, resize guard and equalization. Browser
zoom was previously a boolean coupled to its separate focused field; it now tracks
the zoom target by pane identity and renders that target. Native snapshot,
automation, transfer and split-drag access uses the shared owner. Process/session
and view ownership remain host-specific migration work. Eight focused pane tests,
native build/check and WASM build pass. Native live-session smoke passed restored
layouts, zoom/focus, transfers/merges and shell PID/variable preservation, including
fixture cleanup. Browser split → zoom → numbered pane selection → unzoom retained
independent output in both panes. Screenshot: target/shared-pane-layout-state.jpg.
Not deployed; this is layout-state consolidation, not the shared app entry point
or completion of platform/performance acceptance.

Published shared WASM source 246240b to rmux.app from successful GitHub run
44944665222. Cloudflare deployment c3c998c4; the local site imported the exact CI
artifact and verified WASM hashes before uploading static output. Website source
stays off GitHub. Production smoke passed sidebar multi-selection, Cmd+P, the
correct grouping hint, palette grouping and Cmd+F search/match selection.
Screenshot: target/live-demo-246240b.jpg. Native downloads remain v0.1.4. This
updates the published demo through the recent shared workspace/search/copy work;
it does not establish full application parity or performance acceptance.

Resolved the missing grouping hint: this was a real keybinding conflict, not a
lost focus handle. Cmd+Shift+G for grouping was shadowed by terminal Find Previous.
Grouping now uses Ctrl+Cmd+Shift+G on macOS; Ctrl+Alt+Shift+G elsewhere is unchanged.
Both hosts share the corrected binding and the user guide documents it. Browser
palette hints are resolved once from the opening context, like native, rather
than repeatedly querying bindings while rendering rows. Native check and WASM
build pass. Browser verified the hint after sidebar selection, grouping from
terminal focus with the full modifier combination, and Cmd+Shift+G opening Find
Previous. Screenshot: target/group-shortcut-conflict-fixed.jpg. Not deployed;
native runtime/non-mac shortcut checks and full app migration remain open.

Browser host now handles CommandPalette when sidebar clicks move focus outside
the active workspace's action path. It restores the active terminal context and
opens that workspace's palette without clearing multi-selection. WASM build
passed. Browser smoke verified Cmd-click selection → Cmd+P → query, then Escape
and sample `echo focus-restored` input, with both selected rows retained.
Screenshot: target/sidebar-palette-focus.jpg. Shortcut hint was missing in this
opening path; hint/context parity remains open. Not deployed. Native code was
unchanged; full shared application entry point remains open.

Group Selected Workspaces now uses shared workspace ownership transitions in
native UI and browser, with native automation also using the same multi-member
creation operation. Source anchors retain their slots, selection follows identity,
and sessions are not recreated. Both hosts share the action/palette factory and
Cmd+Shift+G / Ctrl+Alt+Shift+G binding. Twelve lifecycle tests pass, including
multi-member extraction, source anchor promotion, sidebar member order and invalid
request atomicity. Native check and WASM build pass; browser keyboard grouping and
palette regrouping passed. Screenshot: target/shared-group-selection.jpg.
Browser Cmd+P did not open after modified sidebar selection (Commands toolbar did);
that focus-routing issue remains to fix. Native runtime and non-mac shortcuts were
not exercised. Not deployed; full shared application migration remains open.

Sidebar multi-selection now belongs to shared workspace State, including its
range anchor and cleanup when a workspace closes or transfers out. Native uses
that model; the browser now supports Cmd/Ctrl-click toggles, Shift-click visible
ranges, selection counts and ordinary-click reset without switching terminals
on modified clicks. Eleven focused lifecycle tests pass, covering pinned order,
collapsed children, stale identities, unchanged active/MRU and removal cleanup.
Native check and WASM build pass; browser Cmd-click and Shift-click smoke passed.
Screenshot: target/shared-sidebar-selection.jpg. Native runtime and non-mac
modifier behavior were not exercised. Browser Group Selected Workspaces routing
and full shared application entry point remain open. Not deployed.

Moved workspace Up/Down keybindings into shared workspace defaults, adding missing
browser shortcuts: Cmd+Option+Shift+Up/Down on macOS and Ctrl+Alt+Shift+Up/Down
elsewhere. Terminal/find-input search bindings now come from one shared factory;
native copy-mode-specific Find bindings stay in their existing context. Native
check and WASM build pass. Browser smoke verified keyboard reordering followed by
Cmd+F search and match selection. Screenshot:
target/shared-workspace-find-bindings.jpg. Non-mac runtime shortcuts were not
exercised. Not deployed; full application parity remains open.

Workspace Up/Down now routes through shared actions and `State::reorder` in both
hosts. It preserves selection by identity, reorders children within their group,
keeps anchors fixed within groups and prevents top-level moves across pin tiers.
Browser palette exposes both commands. Native check and WASM build pass; ten
focused lifecycle tests pass, including pin boundaries, child/anchor restrictions,
collapsed-group moves and an out-of-range delta. Browser smoke verified moving
rmux below website while retaining its selected terminal/output. Screenshot:
target/shared-workspace-reorder.jpg. Not deployed.

Browser keyboard copy catches Ghostty/JS errors at the wasm-bindgen boundary and
reports a terminal notice instead of letting an exception escape; successful copy
clears that notice. Browser menu/OS copy now catches failures, prevents fallback
copying from hidden inputs/document selection, leaves clipboard data untouched,
and emits a console warning. Menu-copy failures do not yet display a terminal
notice when no keyboard action occurs. Focused handler checks cover failure,
no-selection and success; real-WASM selection/copy checks and browser build pass.
Oversized keyboard-copy notice was not visually exercised. Not deployed.

Browser clipboard extraction now uses the bounded streaming formatter introduced
for search seeds, preserving OUT_OF_SPACE on oversized copy without a full
selection sizing retry. A focused 10,000-line/4 KiB-limit local engine comparison
measured median 1.916 ms for fixed-buffer retry versus 0.025 ms for streaming;
method and limitations are in performance.md. Real-WASM selection/copy and search
checks pass. No UI semantics changed; the production artifact remains older.

Browser Find now seeds from selected terminal text when it is nonempty, at most
4096 UTF-8 bytes and contains no control characters; otherwise it uses the last
query. Older/newer navigation keeps preferring the remembered query. The adapter
uses Ghostty's streaming formatter with a reusable typed WASM writer callback,
rejecting overflow without the format_buf full-selection size-probing retry.
One function-table slot is allocated lazily per engine instance; each seed uses
a 4 KiB buffer and releases synchronous callback state in finally. No upstream
Ghostty modification or extra tool installation is needed.
Real-WASM checks pass for Unicode, exact-limit output, oversized/multiline rejection,
empty selection and disposal; the WASM build passes. Browser smoke verified double-
clicking Ghostty then Cmd+F/Enter seeds/selects it. Screenshot:
target/selection-seeded-find.jpg. Not deployed; broader application parity remains open.

Browser terminals retain their last find query and reopen search for older/newer
match actions when the bar is closed, matching desktop's remembered-query flow.
Initial queries initialize Ghostty search before constructing the bar; bounded
scan scheduling and deferred navigation remain intact. WASM build passes. Browser
smoke verified Cmd+G reopening the saved query and selecting the older match,
then palette Find Newer Match reopening and selecting the opposite end. Screenshot:
target/search-reopen-navigation.jpg. Selection-seeded queries remain a browser
gap; this does not claim complete search parity. Not deployed.

Added the browser palette's missing Find in Terminal / Find Older Match / Find
Newer Match entries. Native and browser palettes use the same find action factory;
browser entries display shortcuts from current bindings and dispatch after focus
restoration. Native check and browser build pass. Browser smoke verified the Cmd+F
hint, palette-to-find focus handoff and matching text selection. Screenshot:
target/palette-terminal-find.jpg. Not deployed. CI run 44942264961 completed its
revised mise setup in 23 seconds (previous observed two-step setup totaled 31s),
but the build was still running at inspection; no full-run speedup claim yet.

Browser palette now includes dynamic Move Workspace to Group destinations using
the desktop title format and group IDs. Both hosts route the parameterized move
action through the shared workspace binder into shared `State::move_to_group`.
Browser destination results refresh when opening the palette and do not inherit
numbered-workspace shortcut hints. Native check and WASM build pass. Browser
smoke verified moving the website workspace into an existing group while keeping
its terminal selected and its output intact. Screenshot:
target/shared-group-destination.jpg. This closes the previously noted browser
group-destination UI gap; it does not complete application parity. Not deployed.

Published successful GitHub run 44941953915 (source 482e836) to rmux.app,
Cloudflare deployment 46da0198. Production now includes shared workspace
ownership/navigation, group creation, current-workspace grouping, pin commands,
anchor selection and member removal. Embedded production smoke verified sample
input, grouping the current workspace, new-member inheritance and anchor
promotion. Screenshot: target/live-demo-482e836.jpg. This artifact predates the
shared pin/chevron presentation, latest move-to-group refactor and CI tool-cache
change; those remain pending. Website source stays off GitHub; native downloads
remain v0.1.4. Overall parity/performance work remains open.

Workspace-to-group moves now run through shared `State::move_to_group`; native
host retains availability checks and UI notice/focus handling. Consolidated
anchor-slot capture/restoration into shared state for native group operations,
group creation, member edits and moves. Nine focused lifecycle tests pass,
including invalid destination nonmutation, moving an anchor between groups,
retaining another selected workspace, revealing the destination and repeated
same-group moves. Native and WASM checks pass. Browser group-target selection UI
is still missing, so this does not claim that browser move-to-group is exposed.
CI run 44942148273 remained pending when checked; tooling speedup is unmeasured.

Sidebar group collapse controls and pin indicators now use shared presentation
functions in both hosts. Browser groups use native SVG chevrons, hover treatment,
tooltip and standard-click guard; pinned groups now visibly show their pin. Native
pin tooltips distinguish groups from workspaces and set the SVG color directly.
Browser names use desktop semibold weight and truncate within available width.
Native check and browser build pass. Browser smoke verified visible pinning and
collapsing a group containing the active child without changing that terminal.
Screenshot: target/shared-sidebar-group-controls.jpg. Not deployed yet.

Shared workspace state now owns group member removal, anchor promotion and group
pin toggling, preserving the selected identity and promoted-anchor sidebar slot.
Native routes all three through the shared workspace action binder, and the
browser palette exposes them through the same commands/state transitions.
Native check and WASM build pass; eight focused lifecycle tests pass, including
anchor promotion/removal, pin retention and stale/ungrouped no-ops. Browser smoke
verified promoting a child to anchor and then removing it while keeping that
workspace selected. The pin command was exercised; pin state/order is covered by
core tests rather than a distinct browser pin indicator. Screenshot:
target/shared-group-edit.jpg. Not deployed; full application parity remains open.

Added the browser palette's missing New Group from Current Workspace command,
routed through shared workspace actions and shared group-creation state. Native
uses the same action binding instead of a separate listener. Browser grouping
retains the existing terminal, reveals the sidebar, and scrolls to the active
group; new empty groups use the same path with explicit generated provenance.
Native check and WASM build pass. Browser smoke verified grouping an existing
sample session, new-workspace group inheritance, and switching back with retained
output. Screenshot: target/shared-current-workspace-group.jpg. Not deployed yet.

Single-workspace group creation and automatic group naming now live in shared
workspace `State`, used by native and browser hosts. Moving an existing anchor
into a new group preserves the old group's promoted-anchor sidebar slot and the
selected workspace identity; generated-anchor provenance remains explicit. Native
multi-selection group creation also uses the shared name allocator while retaining
its existing automation transaction. Seven focused lifecycle tests pass, including
anchor extraction, unique names, invalid-index nonmutation and generated provenance.
Native and WASM checks pass. Full shared application/session integration remains
open; this change has not been deployed.

Published successful GitHub Shared WASM demo run 44941009912, source dae6f46,
to rmux.app via Cloudflare deployment eae8c1f4. Production now includes the shared
find controls, bounded incremental Ghostty search, query history and unified
keyboard/status handling. Browser smoke on the root site's embedded demo verified
sample input, Cmd+F query, Enter match selection, Escape/reopen and Alt+Up history
recall. Screenshot: target/live-demo-dae6f46.jpg. Website source stays local and
off GitHub; only the built site/artifact was uploaded. The later shared workspace
state/navigation changes are not in this deployment. Native downloads remain
v0.1.4 and the overall parity/performance roadmap remains open.

Workspace navigation now belongs to shared `State`: logical ordering, cycle,
numbered/last selection targets and sidebar-row resolution. Both hosts call the
same methods, including pinned/group ordering and collapsed-member header mapping.
Destination lookup is read-only; host focus/activation remains separate. Six
focused lifecycle tests pass, including the new composed-state case for pin order,
collapsed visibility, selection reveal, closed-history removal and empty-state
navigation. Native and WASM checks pass. GitHub source dae6f46 finished successfully;
workspace-state source 03a7d1d was still building when checked. Production remains
on the previously recorded deployment.

Both native `WorkspaceWindow` and browser `BrowserHost` now own
`rmux_core::workspace_lifecycle::State`: workspace entries, active index, groups,
and previous selection. Their select/insert/close/ungroup paths call state methods
instead of passing independently owned fields into free functions. Native restore,
RPC, move/merge, notifications and sidebar accesses now use the shared ownership
model. Platform view entries, process services and focus remain host-specific;
fields remain accessible during migration, so orchestration is not yet fully
encapsulated or a single application entry point.
Native build and browser build pass, along with five focused lifecycle tests.
The live-shell RPC smoke passes restoration, switching, cross-window moves/merges,
PID/variable preservation and fixture shutdown. Browser smoke passes creation,
switching with retained output and closing to the neighboring workspace.
Screenshot: target/shared-workspace-state.jpg. Not yet deployed.

Consolidated find-bar Alt+Up/Down handling into the shared presentation module,
including modifier filtering, composition guard, draft replacement/selection and
propagation control. Native and browser status now use one core formatter for
empty queries, pending scans, no matches and selected counts, removing separate
browser wording. Both native and WASM checks pass. GitHub run 44940804954 was
still building the earlier search UI integration when checked; no new production
deployment is claimed. Shared application/session orchestration remains unfinished.

Moved terminal query history into `rmux-core::search_state`, used by native and
browser find bars. Both now use the same deduplication, draft restoration,
eight-query/16 KiB limits and 4096-byte query bound. Browser history survives
closing/reopening its terminal's find bar and supports Alt+Up/Down without
intercepting composition. Focused shared-core history tests pass, as do the native
check and browser build. Browser smoke verified saved-query recall after reopening
and restoration of an unfinished draft. Screenshot: target/shared-find-history.jpg.
Full application/session consolidation remains open; production deployment pending.

Connected browser terminal search to the desktop's shared find-bar presentation
and text field. Browser scheduling runs one bounded Ghostty search step per timer,
cancels replaced queries, and refreshes on output/resize. Cmd+F, Enter/Shift+Enter,
Cmd+G/Shift+Cmd+G, match counts, selection, and Escape focus restoration are
wired up; terminal-focused navigation defers selection updates to avoid nested
entity borrows. Native cargo check and browser build pass. Local browser smoke
verified forward/reverse selection, terminal-focused navigation, close and resumed
input; screenshot: target/shared-find-bar.jpg. Full application/session consolidation remains open. This change is not deployed yet.

Added lazy Ghostty search operations to the direct browser engine adapter:
query, bounded feed/tick step, explicit output/resize refresh, older/newer selection
and close. It uses the pinned ABI manifest and the same search/selection APIs as
desktop, enforces 4096 UTF-8 bytes before replacing a query, and frees search
references before terminal disposal. No idle search timer or eager handle exists.
Focused real-WASM checks pass for matching, Japanese text, navigation/selected
text, output refresh, rejected queries preserving prior results, reopen and
repeated disposal; the existing engine adapter checks pass too. Build packaging
includes the new module and GitHub CI runs the search check. The browser UI integration is described above; the production demo still awaits
a newer GitHub artifact.

Published successful GitHub demo run 44939199824 (source a18ec98) to rmux.app,
Cloudflare deployment b6c3a2a9. The live demo now includes shared workspace
insertion/selection/removal/ungrouping, workspace/tab action routing, numbered
pane/tab shortcuts and the desktop zoom status/restore control. Browser smoke
verified splitting, direct pane selection, sample input, zoom and click-to-restore
on the production page. Screenshot: target/live-demo-a18ec98.jpg. Website source
remains local/off GitHub. Native process shutdown fixes are newer and remain
unreleased; desktop download links still point to v0.1.4.

Extended explicit process shutdown to window-close paths. Closing without a
prompt, accepting the close prompt, and allowing the OS window-close callback
now request shutdown for currently owned workspace/pane sessions before removal.
Pending workspace, split and tab launches receive cancellation. Workspace close
reuses the same ownership traversal; move/detach entry removal remains separate.
Desktop compilation passes. The native window-close gesture/prompt was not
runtime-tested this cycle, so window teardown acceptance remains unverified.
Not released.

Fixed process shutdown being delayed by retained UI entities. Current debug
source reproduced one original child surviving workspace close; explicit shutdown
requests in confirmed single/batch tab close and workspace close now leave none
in the two-second eight-pane sample. Workspace closure cancels pending split/tab
launches. Move/detach now use a separate entry-removal path to retain live sessions;
merge sources have already surrendered their panes. A focused retained-Session
shutdown test passes, desktop builds, and live-shell RPC transfer/state smoke
passes. Final sample retains only app + survivor shell after close and no original
children after quit. Evidence: docs/benchmarks/rpc-process-close-current.jsonl.
Not released; native Windows and Linux teardown runtime remain unverified.

Added optional process-tree accounting to the native split benchmark. It records
app/descendant RSS, identity-matched CPU deltas, appeared/disappeared processes,
and original child survival after workspace close and app quit; no command-line
arguments are collected. Two focused tests cover nested descendants, unrelated
process exclusion, PID reuse and CPU parsing. One bounded eight-pane v0.1.4 run
recorded 114.27 MiB summed RSS (18.92 MiB children), then 99.97 MiB after close.
One original child survived the two-second post-close interval, none survived
normal quit. This is a release observation, not current-source acceptance or a
physical/GPU memory total. Raw evidence is in docs/benchmarks/rpc-process-tree-
release-v0.1.4-eight-panes.jsonl; investigate current-source close retention next.

Zoomed panes now show the desktop's shared 28px status/restore bar in the demo,
including hidden-pane count, shortcut hint and restore action. Native's previous
inline bar was replaced with the same presentation/control. Browser smoke exposed
focus loss when using generic focused-action dispatch after clicking the bar;
the control now calls the shared PaneCommandHost handler directly. Clicking
restored both panes and subsequent typing reached the active terminal. Desktop
check and WASM build pass. Screenshot: target/shared-zoom-status.jpg. Native
runtime was not re-tested. Not deployed; full application migration remains open.

Numbered-pane shortcuts now share bindings/action routing: Cmd+Option+1–9 on
macOS, Ctrl+Alt+Shift+1–9 elsewhere. Browser resolves visual leaf positions and
moves terminal focus, retaining zoom on the chosen pane as desktop does. Invalid
positions do nothing. Browser smoke split two panes, selected/typed in pane 1,
zoomed it, then selected/typed in pane 2 while retaining zoom. Screenshot:
target/shared-pane-numbers.jpg. Desktop check and WASM build pass. Native runtime
and non-mac/alternative-layout input were not tested. Not deployed.

Numbered-tab selection now shares bindings and action routing on desktop/browser:
Cmd+Shift+1–9 on macOS and Ctrl+Shift+1–9 elsewhere select exact tab positions.
Bindings retain numeric keys and add shifted-symbol aliases because browser GPUI
reports US Shift+1 as ! (the first browser smoke exposed this mismatch). After
the fix, browser smoke created three tabs and selected/typed independently in
tabs 1 and 2 using the shortcuts. Screenshot: target/shared-tab-numbers.jpg.
Desktop check and WASM build pass. Native runtime, non-US keyboard layouts and
non-mac modifier shortcuts were not tested. Not deployed.

Tab action routing now shares one GPUI module between native PaneView and browser
SamplePane for New Tab, next/previous tab, move left/right and close active tab.
Both palettes use the shared action factory; the browser retains its empty-layout
New Tab path. Native startup and close confirmations remain in the session host.
Desktop and WASM compilation pass. This routing refactor was not runtime-tested
this cycle and is not deployed. Numbered-tab selection and advanced tab menu
commands remain native-specific, and full application unification is unfinished.

Workspace action routing now shares one GPUI module between WorkspaceWindow and
BrowserHost: create/close workspace, create/ungroup group, next/previous/last
workspace, toggle sidebar and numbered selection. Both palettes use its action
factory; browser palette dispatch restores focus before bubbling the action to
the host. Native handlers retain async shell startup, close confirmation and
rename cleanup. Desktop check and WASM build pass. Browser smoke created a
workspace with Cmd+N, closed it from Cmd+P, then typed into the selected successor;
output/focus remained correct. Screenshot: target/shared-workspace-actions.jpg.
Native runtime routing and every command were not re-tested. Not deployed.
The full shared application entry point and GPUI backend boundary remain open.

Workspace insertion now uses the shared lifecycle in desktop configured creation
and browser sample creation. It accepts an already-created entry, resolves origin
by identity, handles group inheritance/placement, reveals the parent and preserves
the outgoing active identity until explicit selection. Duplicate identities and
missing origins reject before membership/list mutation. Native directory lookup,
process creation, subscriptions and placement preferences stay in host services;
browser limits and sample construction remain its adapter. Five lifecycle tests
pass, covering grouped insertion, top insertion before the active workspace,
invalid origins/duplicates and empty-window creation, plus prior close/select/
ungroup cases. Desktop and WASM checks pass. Runtime startup behavior was not
re-tested; not deployed. Full shared views, terminal services and application
entry-point migration remain unfinished.

Workspace selection now commits active identity, previous-workspace history and
collapsed-group reveal through one core lifecycle operation. Native regular
selection, terminal targeting, same/cross-window tab transfer and focused creation
use that operation alongside BrowserHost selection. It deliberately does not
focus a terminal: transfer hosts retain their no-intermediate-focus behavior.
Repeated selection preserves the last-workspace toggle, while invalid indices
leave state unchanged. Four focused lifecycle tests and desktop/WASM compilation
pass. Native transfer UI and browser runtime were not re-tested this cycle; not
deployed. Workspace creation and unified application entry remain open.

Ungrouping now uses the shared workspace lifecycle in both hosts. It materializes
logical order including collapsed members, removes group ownership without
recreating sessions, preserves individual pin flags and restores selection by
identity. Invalid group requests leave state unchanged. Browser ungrouping also
scrolls to the retained selection and safely ignores an empty workspace list.
Three lifecycle tests pass, including hidden member order differing from storage,
selection and individual pins after dissolution; desktop and WASM checks pass.
No runtime UI verification this cycle; not deployed. Workspace creation, selection
orchestration and a unified application entry point remain migration work.

Published GitHub shared-demo run 44937216212 (source 33a210d) to Cloudflare
44619191 and rmux.app. This brings Unicode sample editing, optional CJK/monochrome
emoji fonts, shared pane command routing, pane joining and tab-to-split actions
to production. Browser smoke entered Japanese/emoji sample output, split the
terminal, switched to the clean website workspace, then returned with the first
workspace's output/layout retained. Screenshot: target/live-demo-33a210d.jpg.
Website source stays local/off GitHub. The later shared join guard and workspace
close lifecycle commits are not included in this deployment.

Warm shared-demo CI run 44937216212 restored the preceding commit's 763 MB cache.
Its build task took 97.01 s: Ghostty step about 33.11 s, release Cargo 59.92 s,
and size-report step about 1.40 s. Earlier baseline run 44933212995 took 209.66 s
with roughly 104 s Ghostty and 51 s reporting. This is an observed pair of runs,
not a controlled benchmark or a guaranteed speedup; source and runner state
changed. The tracked Zig global cache and Brotli quality-6 report are working.
Brotli figures use a different quality from older reports; hosted compression
and release optimization settings were not changed.

Workspace close now has one ownership/lifecycle operation in rmux-core, called
by native WindowView and BrowserHost. It removes the entry and group membership,
forgets closed previous-workspace destinations, promotes anchors in the original
sidebar slot, and selects the successor in logical pinned/grouped order while
preserving other active workspaces by identity. The removed entity remains owned
until host cleanup finishes. Native process confirmation/sidebar cleanup/window
closure and browser empty-state focus stay in host adapters. Two focused tests
cover anchor promotion with separated storage, active/inactive removal, pinned
order, invalid indices and closing the final entry; desktop and WASM checks pass.
Runtime UI was not re-tested this cycle. Not deployed; shared application
entry-point and renderer-version convergence remain unfinished.

Pane join targeting and layout guards now run through one core transition in
both desktop and browser hosts. Join Next follows the layout's visual leaf order;
directional joins use the same neighbor resolution. Missing/single panes are
no-ops, and zoom, resize or pending split rejects ownership transfer with the
same message. Native process/editor guards and browser capacity remain host
checks. Seven focused pane-command tests pass, including nested join targets,
invalid focus and each busy state; desktop and WASM compilation pass. No new
runtime or visual claim, and not deployed. Full shared application entry-point
migration remains open.

Browser now supports Move Tab to Split Right/Below/Left/Above through the shared
pane action factory/handlers. It validates capacity/layout acceptance before
taking the active tab, builds a pane around the existing entity and label, keeps
the source's neighboring tab selected, and exits zoom. No terminal is created.
Desktop handlers use the same routing; native session transfer behavior remains
in its host. Desktop check and WASM build pass. Browser smoke moved Terminal 2
right, continued input, and joined it back into Terminal 1's pane with title and
output preserved. Screenshot: target/shared-tab-split.jpg. Other directions and
rejection boundaries were not browser-tested this cycle. Not deployed yet.

Browser now supports Join Pane into Next/Left/Right/Up/Down through the shared
pane action routing. Native and browser append transferred tabs using the same
core ordering/selection rule, then perform host subscriptions/cache cleanup and
focus exactly once. Browser joining retains terminal entities and labels, removes
the empty source split, blocks zoom/resizing and enforces its 16-tab pane limit
before ownership changes. Command failures now remain visible outside the palette.
Focused transfer test, desktop check and WASM build pass. Browser smoke joined a
two-tab source into a one-tab destination, verified all three outputs and retained
typing focus on the source's active tab. Screenshot: target/shared-pane-join.jpg.
Native runtime and browser capacity/zoom rejection UI not re-tested. Not deployed.

Pane keyboard/action routing now uses one shared GPUI module in both hosts for
19 split/focus/zoom/resize/swap commands plus next/previous pane. Both palettes
use its action factory; the browser restores focus and dispatches pane actions
through the same event handlers instead of running a separate palette path.
Added browser Split Left and Split Above with shared insertion policy. Desktop
check and local WASM build pass. Browser smoke verified left/above placement,
keyboard zoom and palette equalize restoring all three panes with their output.
Screenshot: target/shared-pane-routing.jpg. Native runtime routing was not
re-tested; workspace orchestration/platform services remain separate. Not deployed.

ASCII sample typing now appends one-cell graphemes directly in Rust, avoiding
whole-draft clones and native-engine/JSON measurement calls for the common path.
Non-ASCII drafts still remeasure through Ghostty so cross-event combining and
ZWJ clusters remain correct. A focused test fills the 4 KiB input cap without
invoking the engine callback, checks rejection/clear/backspace behavior, and the
existing engine-backed Unicode editing test still passes. WASM check passes.
This removes a known per-keystroke boundary cost; no input-to-display timing or
allocation-total improvement has been measured. Not deployed yet.

Optional font loading now supports independent CJK and emoji requests. Reused
Rustable's licensed, unmodified 1,982,596-byte Noto Emoji outline font, outside
WASM and only fetched on emoji input/preedit. The shared loader rejects unknown
asset kinds and bounds/verifies each asset; both fonts' success and failure-path
tests pass, along with the WASM build. Browser smoke rendered common emoji,
then CJK in the same session while retaining prior output. Screenshot:
target/shared-emoji-cjk.jpg. Emoji are monochrome, and exhaustive sequence/color
coverage and actual IME behavior remain unverified. No new deployment yet.

Added on-demand, same-origin CJK fallback using Rustable's existing unmodified
SIL OFL Noto Sans CJK JP asset. CJK input/preedit requests it once; ASCII-only
sessions do not. The 16,467,736-byte font stays outside WASM, is bounded and
SHA-256 verified before registration, and remains shared across panes until page
close. Terminal row/cursor caches invalidate when it arrives. A visible error
reports download/registration failure without replacing existing fonts. Loader
success/HTTP/truncation/oversize/hash-failure tests and native check/WASM build
pass. Browser screenshot target/shared-cjk-font.jpg confirms Japanese, Chinese
and Korean output. Licensing/source notices ship with the asset and release
notices. Not yet deployed. Emoji fallback, full script coverage and actual IME
validation remain open; no startup or memory improvement is claimed.

Sample session input now uses Ghostty grapheme segmentation (mode 2027 enabled),
accepts a bounded 4 KiB Unicode draft, and backspaces entire clusters including
combining/ZWJ sequences split across input events. A horizontal draft viewport
keeps editing on one row through narrow-pane resizes; submission records the
full command, and bundled `echo <text>` supports exercising Unicode. Draft
measurement failures/oversize edits preserve existing input. Native engine-backed
model tests and the local WASM build pass. Browser smoke verified accented text,
joined-emoji deletion, long draft resizing, backspace and full command history.
Screenshot: target/shared-unicode-input.jpg. CJK glyphs still render as missing
boxes with the browser's current bundled fonts; actual IME events, multiline
paste and full shell editing remain open. Not yet deployed.

Published GitHub artifact 44934228048 / e52115e to rmux.app through Cloudflare
deployment 3783d1ac. This brings sidebar resizing, parallel startup, native paste
encoding, on-demand memory diagnostics and shared zoom/resize/equalize policy to
production. Browser inspection confirmed the versioned iframe URL, terminal help
input and splitting. Screenshot: target/live-demo-e52115e.jpg. Website source
stayed off GitHub. More recent pane cycling/close/insertion/swap changes are not
included; their optimized GitHub builds were still pending/in progress.

Browser palette now includes all five desktop pane-swap commands: next, left,
right, up and down. Both hosts share target selection, including directional
swap restrictions while zoomed. Swaps keep the existing pane/session identity;
they do not recreate terminals. Six focused pane-command tests, desktop check
and local WASM build pass. Browser smoke confirmed Swap Pane Left exchanges
existing output and preserves typing focus in the moved pane. Screenshot:
target/shared-pane-swap.jpg. Not deployed yet; native runtime and the remaining
browser direction/zoom interactions were not re-tested in this cycle.

Shared-demo build optimization: successful CI run 44933212995 spent ~104s in
Ghostty source build, 52.38s in Cargo, and ~51s generating the compressed-size
report. CI now puts Zig's global cache under already-cached `.deps`; a subsequent
warm CI run is required to establish its benefit. Report-only Brotli changes from
Node default quality 11 to explicit quality 6, recorded in the manifest. On the
same local GitHub artifact 44931038517 (20,413,507-byte UI WASM), one serial sample
measured quality 6 at 225.9ms / 5,832,677 bytes versus quality 11 at 22,888.4ms /
5,021,623 bytes. These are report generation timings, not runtime performance or
actual HTTP transfer sizes. Executing the updated reporter against both artifact
modules confirms raw sizes, hashes and gzip sizes unchanged. Release compiler
optimization and Cloudflare settings remain unchanged.

New desktop terminals, desktop tab-to-split transfers and browser splits now use
one shared pane insertion transaction. It validates the target, unique pane
identity and nesting limit before changing the tree, and returns visual order
for native entity storage. Native new-pane subscriptions are registered only
after insertion succeeds. Five focused pane-command tests cover empty layouts,
before/after placement, stale targets, duplicate IDs, depth rejection and the
previous focus/zoom transitions. Both host checks pass; this insertion change
has not yet had a runtime smoke. GitHub run 44934228048 was confirmed still
building optimized UI; run 44934453970 was pending at the start of this cycle.
No new demo deployment was made.

Pane cycling and focus-after-close now use shared core transitions in both hosts.
They follow depth-first layout order, preserve focus when an inactive pane closes,
and choose the next/previous survivor when the active pane closes. Browser no
longer allocates/sorts its pane map for cycling. Nested-layout focused tests and
both host checks pass. A local development WASM build passed nested split cycle
wraparound and active-pane close/input recovery; surviving output remained intact.
Screenshot: target/shared-pane-lifecycle.jpg. This is not native runtime or
optimized performance validation; full application orchestration is still separate.

Desktop and browser now invoke shared pane-command transitions for zoom, resize
and equalize. This fixes browser-only single-pane zoom, resizing hidden splits
while zoomed, and equalize leaving zoom active. Two focused policy tests and both
host compilation checks pass; these changes have not yet had a browser smoke or
been deployed. Platform lifecycle guards remain in desktop.

Exposed pinned Ghostty grapheme measurements through native and WASM adapters,
with shared UTF-8 endpoint/cell-width data. Native and WASM fixtures agree for
wide, combining, ZWJ emoji, flags and variation selectors; controls and oversized
input are rejected, as are malformed JS surrogates. This is preparation for the
sample editor, which still accepts only bounded ASCII. It does not establish
Unicode editing/IME parity. Full shared-application migration remains open.

Measured GitHub v0.1.4 through a mise-managed installation: one eight-pane idle
fixture instance, 10s settling, 60s observation. App RSS 94.86 MiB, quantized app
CPU 0.0167% of one core, split RPC p50/max 6.07/6.76ms. Native fixture shutdown
succeeded. Raw sample and limits are recorded in docs/performance.md. Harness
now supports bounded pane-count/repetition options for short targeted runs.
This is not complete helper/GPU, latency, output-pressure or cross-platform acceptance.

Added on-demand UI/engine WASM capacity diagnostics and a reproducible engine
session-lifetime benchmark. At 1/4/16 panes with 2,000 sample log lines per pane,
engine linear-memory capacity was 3.50/10.56/38.81 MiB. All sessions close and
capacity stays stable over three identical cycles. This excludes UI/JS/GPU/browser
memory and does not prove allocator leak freedom. Raw results and scope are in
docs/benchmarks/native-vt-memory.jsonl and native-vt.md. Startup and engine adapter
checks pass; browser diagnostics have not yet been sampled on an optimized build.

Published the first shared Rust/GPUI demo to rmux.app using successful GitHub
artifact 44931993466 / source 0f97882. Cloudflare deployment 82ef2eae succeeded;
live page and terminal help input verified (target/shared-demo-live.jpg). Website
and ops source stayed local, off GitHub. Versioned artifact directories prevent
mixed module versions; site build verifies WASM hashes. Newer resize/startup/paste
changes are not included in this deployment. Full app parity remains incomplete.

Browser terminal paste now uses pinned Ghostty paste encoding with current mode
2004 and the same 1 MiB UTF-8 byte limit as desktop. ABI fields come from the
manifest, temporary buffers are released in finally, and encoding errors show
a terminal message. Adapter checks cover Unicode, bracketed on/off, empty input
and over-limit multibyte input; WASM build and browser clipboard help smoke pass.
Evidence: target/shared-native-paste.jpg. The sample interpreter still rejects
Unicode/multiline command input; these adapter checks do not establish shell parity.

Demo startup now downloads/compiles UI WASM alongside terminal-engine loading,
then instantiates UI only after the engine host is ready. Streaming compilation
uses application/wasm responses; other MIME types use the same downloaded bytes.
Three startup-order/failure tests pass and run in CI. Development WASM build and
browser help-input smoke pass (target/shared-parallel-startup.jpg). Initialization
marks are exposed through browser Performance entries; no measured startup speedup
or first-paint claim yet. Loading/title/error copy now says demo rather than probe.

Native sidebar drag capture, width constraints, divider and double-click reset
now live in shared sidebar_interaction used by both hosts. Browser starts at the
native 200px width and cancels a drag when hiding its sidebar. Native check and
WASM build pass. Browser verified drag to 320px, double-click reset to 200px and
continued help input without losing focus. Evidence: target/shared-sidebar-resize.jpg.

Sidebar toggle control and default shortcut are now shared by both hosts. Browser
supports the toolbar icon, palette command and Cmd+B (Ctrl+Shift+B off macOS).
A mouse smoke check caught lost terminal focus after collapsing; browser now
restores the active terminal/empty-workspace focus. Verified mouse collapse, help
input, Cmd+B restoration and retained output. Native check and WASM build pass.
Evidence: target/shared-sidebar-toggle.jpg.

Workspace toolbar frame/title styling is now shared through workspace_presentation.
Browser uses the native 36px toolbar, border, spacing and truncated semibold title;
its title now follows the workspace instead of always saying rmux. Native check
and WASM build pass. Browser switching to website and typing ls verified, with
screenshot target/shared-workspace-toolbar.jpg. Platform-specific controls remain
separate; this does not complete full toolbar action parity.

Optimized artifact 44931038517 (source c83e72a) passed a browser smoke check:
logs in the left split, help in the right, switch to website and run ls, return
with both original pane outputs preserved, Cmd+P zoom search, execute zoom and
restore splits with Cmd+Shift+Return. No stale workspace text or elongated cursor
was observed in this check. Screenshot: target/shared-optimized-workspace-smoke.jpg.
UI WASM is 20,413,507 bytes raw / 7,093,564 gzip / 5,021,623 Brotli. This checks
optimized behavior, not input latency or runtime memory; it predates IME work.
Latest IME layout artifact 44931530153 remains pending behind 44931419990.

Shared IME layout now covers platform range bounds and pointer-to-UTF-16 hit
testing as well as painting. Browser no longer returns a fixed cursor rectangle
for every range or zero for every point. The terminal origin stays separate from
the composition caret, avoiding applying selection offsets twice. Native and
WASM checks pass; actual IME interaction remains unverified.

Extracted native terminal IME overlay into shared preedit_view: underlined text,
selected-range background and composition caret placement now serve both hosts.
Platform shaping stays in text_layout adapters, including native input bounds.
Browser commit/unmark always invalidates the view so rejected sample input cannot
leave the old composition overlay visible. Native and WASM checks pass. Actual
IME event/visual verification remains pending; the sample interpreter still
rejects non-ASCII committed text, so this is not full Unicode input support.

Last Workspace now uses shared PreviousSelection state in desktop and browser.
The browser palette exposes the command; selecting the current workspace keeps
the previous destination, switching back toggles it, and closing the remembered
workspace clears it. Focused core test and native/WASM checks pass; browser
palette interaction verification remains pending.

CI update: packaging run 44929803785 succeeded. Runs 44930447459 and
44930926951 were cancelled; current workspace-placement run 44931038517 is
in progress. These statuses supersede the older queue notes below.

Workspace creation now uses the same top-level insertion policy in desktop and
browser: after-current placement resolves grouped origins through their anchor
and keeps new unpinned rows out of the pinned tier. Browser previously appended
every new workspace. Core edge-case test, native check and WASM build pass.
Browser smoke confirms Workspace 1 inserted between rmux and website and accepts
help input; evidence: target/shared-workspace-placement.jpg.

Desktop and browser now share grouped-workspace reveal and visible-row resolution.
The browser sidebar tracks scroll position and requests the selected row into view
on workspace switches, matching the native host. Focused core navigation test and
both native/WASM cargo checks pass. Browser visual verification of this scrolling
change remains pending; full workspace orchestration is still not unified.

First optimized GitHub artifact succeeded: run 44929617746, source 5d7c703.
Downloaded `rmux-shared-demo` reports UI WASM 20,368,659 bytes raw / 7,085,501
gzip / 5,019,884 Brotli, compared with development 86,695,622 raw / 12,455,794
gzip. This is a 76.5% raw and 43.1% gzip payload reduction, not a runtime memory
or latency result. Native engine WASM is 913,686 raw / 305,701 gzip / 235,124
Brotli. Packaging follow-up 44929803785 is running; key-release build 44930447459
is queued. Optimized browser interaction and deployment remain pending.

Shared delivered-key tracking between desktop and browser, preserving original
press spelling through shifted aliases/repeats and suppressing orphan releases.
Browser encoded control/non-text keys now use explicit Press/Repeat/Release
actions; focus loss and hidden-terminal cache release drain held keys. Terminal
blur also cancels selection/scrollbar gestures. Core pairing tests and bridge
Kitty press/repeat/release checks pass, as do native check and WASM build.
Printable input still uses the composition handler; negotiated report-all
printable routing and full IME behavior remain unfinished.

Added the Shared WASM demo GitHub workflow and optimized `build-release` artifact
path. Development stays on the quick unoptimized build; normal builds no longer
install/copy the retired ghostty-web benchmark dependency. Release packaging
includes dependency/font notices, source licenses and raw/gzip/Brotli WASM size
reports. The development UI module measured 86,695,622 bytes raw / 12,455,794 gzip.
First optimized CI run: 44929617746; packaging follow-up: 44929803785. At this
checkpoint the first is compiling and the follow-up is pending, so optimized
sizes and successful artifact production are not yet verified. Native/browser
parity documentation was consolidated to remove superseded migration claims.

Aligned browser terminal initialization with desktop: shared Rust foreground
(220,223,230), background (18,20,26), and 4 MiB scrollback limit are passed through
the WASM adapter. Native initialization now also reads the existing shared color
constants; session constructors use the extracted history default. No duplicate
browser color literals are needed. Adapter tests verify configuration validation,
OSC color overrides/reset and zero-history behavior. Native check and WASM build
pass. Browser rendering confirms the shared colors; evidence:
target/shared-terminal-defaults.jpg. This bounds retained history, not total
per-pane memory. The replacement demo remains local.

Completed the browser palette's basic tab lifecycle commands using native
CommandIds and existing keyboard action handlers. Palette execution restores
terminal focus before dispatch, rather than implementing another path for tab
movement. Closing the last terminal now focuses the empty workspace, allowing
Cmd+P / New Terminal Tab and Cmd+T to recreate a terminal. WASM build and browser
checks pass for creation, previous-tab selection, palette close, final-tab close
and recovery through both palette and shortcut. Evidence:
target/shared-tab-palette-recovery.jpg. The published demo is unchanged.

Both hosts now consume shared tab-reorder transitions and keyboard bindings.
Move Tab Left/Right are shared command identities and available in both palettes.
Browser terminal titles are stable per session rather than position-derived;
selection and reorder reveal the active tab in the strip. Three tab-model tests,
native check and WASM build pass. Browser verified moving Terminal 3 from third
to second by Ctrl+Shift+PageUp, then to first through the palette, preserving
output/focus and refusing to wrap past the boundary. Evidence:
target/shared-tab-reorder-wasm.jpg. The shared demo remains local.

Extracted ordered pane tabs and selection into `rmux-core::tabs::Tabs<T>` and
migrated native PaneView and browser SamplePane to it. Hosts no longer maintain
independent active indices. The collection reconciles removal/insertion/reorder
and drain transitions, clamps restored indices, and shares wraparound navigation.
Native labels, title subscriptions, process launch, focus and persistence stay in
their platform adapter. Two focused lifecycle tests cover surviving selection,
right/last close fallback, empty/recreated panes, transfer insertion, swaps,
invalid restore and wraparound. Native check and WASM build pass. This is shared
application state, but full workspace orchestration and browser parity remain
unfinished; the published demo is unchanged.

Browser text selection now schedules native AUTOSCROLL_TICK events while dragging
beyond the viewport. The browser and desktop share the 30 ms cadence; native
gesture state and 64-bit scrollbar bounds determine whether to continue. The
browser cancels its single pending task on release, returning inside, switching
hidden terminals or starting scrollbar dragging. Direct engine tests verify
up/down motion, selecting historical text, re-entry and history-bound stopping.
Native check and WASM build pass. Held-pointer timing in a browser remains an
interaction check; no deployment or full browser-parity claim is made.

Desktop and browser now share draggable scrollback geometry and absolute history
row commands. Dragging preserves the thumb grab position and clamps to history
limits; the existing indicator stays hidden at the live bottom. Three focused
core scroll tests, native check, WASM build and direct engine ROW checks pass.
Browser dragging reaches the expected history rows. An apparent idle repaint
failure in displayed tool screenshots was not supported by raw captures: three
raw captures were byte-identical, including the visible text (27,567 bright
pixels in the terminal text region). Experimental cache/layer
changes were removed. Selection edge autoscroll remains pending; this shared
browser build is still local and has not replaced the published demo.
Evidence: target/shared-scrollbar-drag-verified.png.

Desktop and browser now render the same extracted scrollback position indicator:
3px width, minimum 8px thumb, native color and hidden at the live bottom. Native
check and WASM build pass. Browser verified top/middle placement and hiding after
Shift+End; evidence: target/shared-scrollbar-wasm.png. This matches the current
desktop indicator, which is not draggable. Drag interaction remains unfinished
on both hosts, along with selection edge autoscroll.

Desktop and browser now share the Scroll command type and Shift navigation
mapping in rmux-core. Shift+PageUp/Down overlaps one row between pages;
Shift+Home/End selects top/bottom. Browser palette exposes native Scroll to Top
and Scroll to Bottom commands for the active terminal. Two focused scroll tests,
native check and WASM build pass. Browser verified Shift+Home, the PageDown
overlap and palette return to the bottom without errors. Evidence:
target/shared-scroll-navigation-wasm.png. Scrollbar controls and selection edge
autoscroll remain pending.

Measured and optimized the current direct browser-engine adapter, rather than
reusing timings from ghostty-web. In the 80×24 synthetic benchmark, dense updates
across 16 panes fell from 24.418 to 5.650 ms median (26.689 to 6.594 ms p95);
single-row updates fell from 1.057 to 0.296 ms median. Cached memory views,
packed single-codepoint reads and consecutive per-row style reuse remove repeated
allocation/decoding while retaining the full-grapheme path. Payload size and dirty
row counts are unchanged. Engine checks and WASM build pass. Raw measurements and
scope: docs/benchmarks/native-vt.md. This excludes Rust decode, shaping, GPU and
presentation latency; browser/per-pane memory and end-to-end latency still need
measurement before performance parity claims.

Browser wheel scrolling now calls the native Ghostty viewport API. Desktop and
browser share fractional wheel accumulation and the existing 32-line dispatch
bound through rmux-core::scroll. Typing returns to the bottom; the bundled logs
command produces 80 sample lines for scrollback interaction. Focused wheel and
engine history/selection tests, native check and WASM build pass. Browser verified
scrolling to older lines, copying an exact historical line and typing ls back at
the live prompt with no errors. Evidence: target/shared-scrollback-copy-wasm.png
and target/shared-scrollback-input-wasm.png. Scrollbar UI, scroll navigation
shortcuts and selection edge autoscroll remain pending.

Browser mouse selection now uses the same libghostty-vt gesture API as desktop:
fractional cell geometry, tracked anchors, word/line behavior and rectangular
ranges stay engine-owned. Shared Rust row painting displays selection ranges.
Copy uses Cmd+C/macOS or Ctrl+Shift+C elsewhere, plus browser copy events for OS
menu shortcuts that arrive without keydown. Only the focused terminal supplies
that event; palette copying remains independent. WASM build/engine tests pass;
browser verified drag, double-click word, triple-click line, exact clipboard text
and palette copy isolation with no console errors. Screenshot:
target/shared-native-selection-wasm.png. Rectangle geometry is engine-tested;
Alt-drag browser coverage, edge autoscroll and terminal context menus remain.

The local browser now runs desktop's pinned libghostty-vt 1.0.1 directly, replacing
ghostty-web in its runtime path. The new render-state adapter emits dirty rows,
cursor and selection updates, wide/combining glyphs, custom underline colors,
curly underlines and overlines. Rust retains sparse rows and selection ranges.
Key names resolve against the engine ABI rather than the older wrapper's numeric
enum. WASM build and focused engine checks pass; browser verified typing, Ctrl+U,
split isolation and workspace switch/restore without errors. Evidence:
target/shared-native-vt-engine-wasm.png. Mouse selection/copy UI, scrollback,
keyboard release routing and full IME remain incomplete. The old adapter is
isolated for historical benchmarks; its timings do not measure the new adapter.
No public deployment yet.

Direct libghostty-vt WASM adapter now owns reusable key encoder/event handles,
synchronizes all terminal keyboard modes before encoding, and supports press,
repeat and release through the same C calls as native. Focused runtime tests
cover Ctrl+C/U, Alt prefix, normal/application arrows, per-session mode isolation,
Kitty press/repeat/release, Unicode key text and return to legacy release behavior.
Enum/struct layout comes from the pinned ABI manifest; key text/output are bounded
and handles are freed on disposal. These checks pass; the browser now routes
its control keys through this adapter. Release-event routing remains pending.

The pinned desktop libghostty-vt 1.0.1 source now has an isolated WASM build
target via mise (892 KiB ReleaseSmall, all engine features retained). The older
ghostty-web wrapper does not expose desktop's native selection gesture API.
Rather than duplicate those semantics, native-vt.js starts a direct adapter to
the same C API, reading enum values/struct sizes from the engine ABI manifest.
Focused WASM runtime tests pass for select-all, Unicode copy, soft-wrap policy,
bounded copy, clear-selection, session isolation and disposal. This adapter is
not wired into the browser UI yet; snapshot decoding, keyboard encoding and
gesture input must move across before replacing ghostty-web. No public deploy.

Browser terminal now registers ElementInputHandler during paint, matching the
desktop committed-text boundary and reusing its Composition model. Printable
text goes through platform text commits; symbolic/control keys remain Ghostty
encoded. This fixes the dropped rapid typeText characters observed in the prior
pane-control check and enables single-line ASCII paste. WASM build passes;
browser verifies rapid help, pasted ls and split → draft → Ctrl+U → ls with no
console errors. Evidence: target/shared-terminal-text-input-wasm.png. The sample
interpreter still limits commands to 48 ASCII bytes; multiline paste, Unicode
commands and rendered IME preedit remain unfinished.

Browser panes now use shared native framing, 28px control hit areas and bundled
SVG icons for new tab, split right/below and zoom. Removed the extra diagnostic
pane header and duplicate focus border. Shared tooltips explicitly select the
host UI font: browser hover previously panicked resolving `.SystemUIFont`.
Native check and WASM build pass. Browser checks cover visible tooltip, repeated
split buttons, zoom/unzoom and subsequent individually delivered terminal keys,
with no console errors. Evidence: target/shared-native-pane-chrome-wasm.png.
Rapid browser typeText delivery dropped a character in this check; input-event
ordering remains an open issue before public replacement. The public demo is
unchanged and the browser host still has separate application orchestration.

Browser Close Workspace command removes its workspace entity/panes/sessions and
uses shared neighbor_after_removal plus group anchor preservation. The last close
shows an empty state with a New Workspace button and focused host for keyboard
creation. Names avoid collisions with existing workspaces. WASM build passes;
browser verified first close selects website, last close shows empty state, then
Cmd+N creates a fresh terminal accepting ls. Evidence:
target/shared-workspace-recreation-wasm.png. Group-anchor close interaction and
measured engine/memory disposal remain unverified; no native process-close prompts
are needed for these bundled sample sessions.


Browser palette displays shortcut hints resolved from the active keymap against
the originating terminal's focus handle, using the extracted native key-label
formatter. Unsupported/unbound actions show no invented hint; numbered workspace
results map to the actual native number bindings. Preserving origin avoids empty
hints while the new palette input has not entered the rendered dispatch tree.
All 15 native shortcut tests and WASM build pass. Browser capture verifies workspace,
group, navigation, split and zoom labels: target/shared-palette-hints-wasm.png.
Browser shortcut overrides and non-macOS visual verification remain pending.


Browser now selects macOS or Windows/Linux keymaps from browser platform metadata
instead of hard-coding macOS. Workspace/directional/split definitions are shared
native sources; editor, tab and pane-cycle defaults match desktop equivalents.
Split/Below/Zoom actions invoke the same browser command execution as the palette.
Native check, WASM build and engine/platform selector checks pass. Available browser
selected macOS; Cmd+D, Cmd+Shift+D and Cmd+Shift+Enter split/zoom/unzoom verified
with restored nested layout. Windows/Linux interaction is not validated here and
reserved browser shortcuts can still require the Commands button. Evidence:
target/shared-split-shortcuts-wasm.png.


Browser directional focus now uses the same PaneTree::neighbor geometry as native,
with shared macOS Cmd+Ctrl+arrow / desktop non-macOS Alt+Shift+arrow definitions.
The browser currently selects macOS bindings; automatic platform selection remains
pending. Focus commands are also in the shared palette catalog. Native check and
WASM build pass. Browser verified Up from bottom-right to top-right then ls, and
Left to the full-height left pane then help, retaining other sessions. Evidence:
target/shared-directional-focus-wasm.png. Zoomed navigation uses the same focused
pane state but has not been separately checked in this iteration.


Resolved the nested equalization investigation: temporary event tracing confirmed
correct nested path and click counts 1/2; the subsequent frame restored equal
heights. Prior captures were premature, not evidence of a hit-test defect. Trace
code was removed. Added all four Resize Pane directions and Equalize Pane Sizes
to the browser palette using shared command identities and core tree operations.
Clean WASM build passes; browser verified Resize Pane Up moves the divider and
Equalize restores equal sizes with sessions retained. Evidence:
target/shared-resize-commands-wasm.png. Broader browser/platform parity remains.


Fixed the observed post-drag missing-text frame by notifying the shared split
host when mouse release clears its drag state. The final non-dragging scene now
renders without needing another pointer action. A prepaint invalidation attempt
did not fix the issue and was removed. Native check and WASM build pass; the same
nested vertical drag now leaves all three terminals visible immediately afterward.
Nested double-click at the moved divider still did not equalize; hit testing/
retained child geometry remains an open investigation. Evidence:
target/shared-resize-repaint-fixed.png. Do not treat this as complete resize parity.


Native and browser now call one recursive split renderer, including branch bounds,
weighted child layout, divider hover/cursor/tooltip, drag start and double-click
weight equalization. Hosts supply only leaf content and layout/drag state. Native
check and WASM build pass. Browser nested right/below layout and vertical dragging
work, but exposed a transient repaint issue: upper-pane text disappeared after
shrinking height and reappeared on the next pointer interaction at unchanged
geometry. Output was retained, but vertical resize rendering acceptance is NOT
complete. The attempted nested double-click did not visibly restore equal heights;
check hit geometry/measurement as well as painting next. Evidence:
target/shared-nested-split-wasm.png captures the later repaint, not a bug fix.


Extracted native divider drag capture/state updates into split_interaction and
consumed it from both hosts. Browser dividers measure their branch, update shared
PaneTree weights with the native 48-pixel minimum policy, show resize cursors and
equalize adjacent weights on double-click. Layout mutations/workspace switching
clear pending browser drags. Native check and WASM build pass. Browser verified
horizontal drag from equal widths to an unequal split and double-click restoration
with both live sessions/output retained. Evidence: target/shared-divider-resize-wasm.png.
Nested/vertical drag and native interaction regression remain unverified.


Browser command results now use GPUI ListState/list virtualization, native
list_navigation::reveal and shared viewport-based paged_result. Search/catalog
updates reset selection and scrolling. Native palette uses the extracted paging
function unchanged. Six native navigation tests pass and WASM build passes.
Browser check with 10 workspaces verified Up wrapping to the last off-screen
result, Page Up revealing the correct earlier result, and filtering resetting to
a visible first match. Evidence: target/shared-palette-scroll-wasm.png. This closes
the pending long-list keyboard visibility gap; other browser parity work remains.


Browser palette now snapshots workspace names/identities in logical order when
opened, combines them with shared command titles in RankedSearch, and dispatches
workspace selection by stable entity identity. Result labels follow native
Switch to Workspace N: name wording. A scroll container bounds the combined
catalog. WASM build passes; browser verified searching website, entering ls,
switching away, then searching back with output and terminal focus retained.
Evidence: target/shared-workspace-search-wasm.png. Collapsed-member search uses
the existing reveal path but was not separately rechecked. Keyboard scrolling
of off-screen results, contextual commands and shortcut hints remain pending.


Connected shared CommandId entries for workspace creation, new empty group,
next/previous workspace and ungrouping to browser host events from the native
palette presentation/input. Ungrouping materializes logical order and preserves
selected entity/session, following native behavior. WASM build passes; browser
verified searching/running new group, adding a child, running ls, then searching/
running ungroup with output and selection retained. Captured the eight-command
palette at target/shared-workspace-palette-wasm.png. Other native commands,
workspace-name search, contextual availability and shortcut hints remain pending.


Connected browser group creation through the shared native Ctrl+Cmd+G binding.
New workspaces inherit the active group through Groups::add_new; sidebar rows use
the model's visible order, anchor titles and member indentation. Disclosure toggles
keep the active session unchanged and highlight its collapsed group. Numbered or
cyclic selection includes hidden children and reveals the selected member. Native
check and WASM build pass. Browser verified creation → add child → ls → collapse
→ select another workspace → Cmd+4 reveals the child with output/focus retained.
Evidence: target/shared-workspace-groups-wasm.png. Group editing, ungrouping,
drag/reorder and command-palette exposure remain unfinished in the browser.

Moved the existing native workspace group model into rmux-core, including pin/
member ordering, anchor promotion, insertion/removal rules, restore validation
and its tests. Desktop re-exports the shared model; browser owns the same Groups
model and uses its logical order. Both hosts now use shared numbered-target and
cycle-target rules (9 selects last; cycling wraps in logical order). Workspace
icon validation is shared too. All 14 focused group/navigation tests pass, along
with native and WASM checks. This is a behavior-preserving extraction; browser
group creation/editing controls and full application-state integration remain.


Browser workspaces now retain independent pane trees, tab selection and live
sample terminal engines. Sidebar/header/row presentation and workspace shortcut
definitions are actual native sources consumed by both hosts. Hidden workspaces
release shaping/cursor caches, retaining sessions. Native check and WASM build
pass. Browser checks verified split/tab/output restoration after switching,
Cmd+N creation with terminal focus, Cmd+1 selection and Cmd+Option+Up wraparound
restoring output. Captures: target/shared-workspaces-wasm.png and
 target/shared-workspace-shortcuts-wasm.png. Host state remains browser-specific;
full native workspace services, groups, persistence and browser platform keymap
selection are not yet shared. Public demo remains unchanged.


Resolved the Ctrl+U interaction question with a clean browser regression:
type ab → explicit Control+U → ls → Enter clears the draft and runs ls correctly.
The earlier generic ctrl chord was not delivered to the page. Batched draft
erasure into one WASM write instead of one per character; an empty draft now
skips snapshot/allocation work. WASM check passes. Ctrl+C browser delivery and
broader keyboard/IME behavior remain separate acceptance work.

Connected shared symbolic keys to the pinned ghostty-web encoder through an
explicit numeric adapter (browser and native Ghostty key enums differ). Each
session owns/disposes its encoder; cursor/keypad modes are queried before encode.
Sample-shell input now consumes encoded bytes, including control-byte handlers.
WASM build and engine assertions pass for plain text, Enter, Backspace, Ctrl+C,
Ctrl+U and normal/application arrows. Browser verified plain ls input end to end.
Control-key browser interaction remains unresolved: some automated chords never
reached DOM, and the attempted mixed control/plain sequence needs further checks.
Do not claim control-key UI parity yet. Kitty state synchronization, release events,
IME and full native input-view sharing remain pending. Temporary trace logs removed.

Moved symbolic terminal key normalization, modifiers and press/repeat/release
identity into rmux-core::keyboard. Native keyboard encoding maps shared symbols
to Ghostty FFI constants; the browser sample handler uses the same normalization
for control keys. Shifted punctuation/letter aliases, distinct Unicode and F1–F24
bounds are covered by a focused core test; all eight native keyboard tests pass,
including negotiated press/repeat/release encoding. WASM check passes. Browser
IME, full Ghostty key encoding and a shared terminal input view remain pending;
this extraction does not claim full input parity or expand sample-shell commands.

Added browser tab close controls, middle-click and Cmd+W. Closing a tab drops
its terminal entity/engine; selection adjustment follows native behavior and
only replaces keyboard focus when the closed terminal held it. Empty panes emit
an event and are removed through shared PaneTree::remove; the final pane leaves
a New Terminal empty state. Pane limits now count live panes, allowing repeated
close/recreate cycles. WASM build passes; browser verified close active tab →
remove split → close final pane → recreate → type ls. Background-tab close focus
and middle-click still need interaction coverage. Public demo remains unchanged.

Added multiple live terminal tabs per browser pane using extracted native
30px tab-strip and active/inactive tab presentation. Native retains its existing
handlers, badges and drag/drop behavior. Both hosts share shifted-bracket tab
bindings and core cycle calculation; browser adds aliases for retained Shift on
brace key events. Cmd+T/plus creates a session, clicking or cycling switches it,
and hidden tabs release shaped caches while retaining engine state. Native check
and WASM build pass; browser verified distinct tab output and Cmd+Shift+[ return
to retained ls output. Closing/reordering tabs, metadata, workspace integration
and native input sharing remain pending. Public demo remains unchanged.

Fixed first-pane prompt/cursor displacement in the browser. A temporary resize
trace showed the initial zero-space canvas resized the live engine 52×16 → 1×1
→ 150×30; reflow moved the prompt cursor from column 2 to column 1. Skip engine
resize until content fits at least one full cell. Actual visible resizes remain
unchanged. WASM build passes; fresh browser load → split → previous pane → ls
now preserves the prompt space. Temporary trace logging was not committed.

Shared native Cmd+[ / Cmd+] pane bindings with the browser host and moved
wraparound index calculation into rmux-core. Browser cycles in layout leaf order,
retains zoom state, and routes typing to the selected terminal. Shifted brackets
remain reserved for tab navigation, which the browser host has not implemented.
Focused core test, native compile and WASM build pass; browser verified previous
pane → ls and next pane → help without mouse focus. A visual discrepancy remains:
the first pane's initial prompt loses its trailing space after resize; investigate
engine reflow/cursor position before public deployment.

Compacted browser cell transport to tuples, retaining dirty-row updates, resolved
colors, grapheme strings, widths and style flags. Rust reconstructs the shared
Cell type with inline small strings and unchanged-row Arc reuse. Engine checks
and WASM build pass; browser split/typing retains colors and cursor placement.
The same 16-pane benchmark reduced full-update payload from 7.34 MB to 0.99 MB
(86.5%) and median engine/serialization time from 7.93 to 2.85 ms. Results exclude
Rust decoding and UI rendering; full performance acceptance remains open.

Added a repeatable browser engine/JSON boundary benchmark for 1/4/16 panes,
80×24 cells, clean/one-row/full-viewport workloads, isolated Node processes and
20 warmup + 100 measured updates. Dirty-row assertions pass in all nine cases.
On M4 Max, 16-pane full updates serialize 7.34 MB at 7.93 ms median / 9.14 ms p95;
one-row updates take 0.57 ms median. This excludes Rust deserialization and UI
rendering. Raw results and limits are in docs/benchmarks/browser-engine.md.
Compact cell transport is the next measured optimization target, not a claim
that end-to-end browser performance acceptance has passed.

Added a guarded batched ASCII shaping path to the browser adapter. It shapes a
run once, verifies every caret boundary and trailing width against terminal
columns, then retains the full run if aligned. Unsupported shaping falls back
to exact per-character placement. Native shaping is unchanged. WASM build passes;
browser split/typing rendered correctly with 22 batched runs and zero fallback
runs in the sample. This reduces shaping calls there, not a measured latency or
memory claim. Wide/combining glyph coverage and performance acceptance remain.

Fixed browser focus routing after palette actions and dismissal. Executing a
split focuses the new terminal; Escape returns focus to the selected terminal;
pane headers focus their terminal as well. Cmd+P opens the shared palette.
WASM build passes. Browser verified split → immediate ls, Cmd+P → Escape →
immediate help without clicking the terminal. Broader native input/IME sharing,
workspace/tab state and browser shortcut coverage remain pending.

Extracted native block/bar/underline/hollow cursor quads into cursor_view and
compiled the same drawing code into the browser. The browser uses the existing
shared CursorCache for block-cursor glyph inversion, honors hidden cells and
cell width, and draws hollow cursors when unfocused. Desktop compile and WASM
build pass; browser verified focused block placement after typing alongside an
unfocused hollow cursor. Blink scheduling, preference policy and native input
sharing remain pending; this is not full cursor/input parity yet.

Browser terminal dimensions now follow pane content bounds through the same
CellMetrics::grid_size and padding calculation used by native. Split/zoom resize
the existing Ghostty instance; unchanged geometry skips resizing and snapshots.
The adapter forces full row replacement after resize so old-width cached rows
cannot survive. Three native cell-metric tests, WASM build and engine resize
checks pass; browser split/zoom preserves visible session output. Cursor/input
sharing and workspace integration remain the next browser parity work.

Connected the shared browser row painter to live ghostty-web 0.4.0 engine
instances per pane. The host transfers dirty rows only and Rust retains unchanged
row Arc identities; there is no idle polling. ASCII sample commands now exercise
real VT parsing through the shared painter. Adapter checks cover sparse updates,
colors, isolation, clear and reused WASM memory; browser checks ran ls/help in
separate panes. WASM build passes. Cursor painting, resize, native input/IME
sharing, workspace integration, decoration gaps and performance remain pending.
The public demo is unchanged; this work remains in the compatibility host.

Connected the actual native row_cache and paint_row sources to the isolated
GPUI CE browser host through small host adapters. Native keeps its existing
fixed-width shaping call; browser currently places ASCII glyphs individually
using measured Fira Mono columns. Frame serialization supports a reproducible,
synthetic native Ghostty capture. Browser rendering verified two panes with
colors, bold, underline, strikethrough and concealed-text suppression. Eight
native row-cache tests and the WASM build pass. Live browser VT parsing, cursor
and selection interaction, wide/combining-font coverage and performance are
still pending. This is a shared renderer milestone, not full app parity; the
public demo remains unchanged.

Moved the native renderer's terminal style-run grouping into
rmux-core::terminal_runs and routed row_cache through it. This preserves exact
column/style boundaries, hidden-text suppression, wide/non-ASCII isolation,
selection-from-background splits, decorated blanks and one-allocation ASCII
runs. All eight focused row-cache tests pass, including engine-backed fixtures;
core compiles for WASM. Browser glyph shaping still needs an adapter: GPUI CE's
shape_line lacks the old fixed-cell-width argument, so column alignment must be
verified explicitly rather than dropping that constraint silently.

Moved terminal Cell/Color/CursorShape/Frame/FrameStorage and bounded text export
unchanged into rmux-core::terminal_frame. The native engine re-exports these types
and retains the Ghostty FFI color conversion; desktop row_cache now consumes the
shared types directly. This removes native engine/PTY linkage from the snapshot
contract while retaining Arc row identities and inline cell text. Four snapshot
tests plus bounded UTF-8 export pass; desktop and WASM compile pass. No browser
terminal renderer or VT-engine adapter is connected yet.

Extracted native pane sizing, weighted children, branch orientation and divider
appearance into shared split_presentation components. Desktop retains its existing
drag capture and resize handlers. The browser host now executes shared command
identities for Split Right/Below through rmux-core PaneTree::split_positioned and
supports focused-pane zoom without replacing the tree. Browser check passed
right split → nested below split → zoom → unzoom, preserving all three panes.
Native compile and WASM build pass. Browser pane contents are clearly labeled
placeholders; terminal rendering, tab/workspace integration and divider dragging
remain pending. The public site remains unchanged.

Moved all 136 static palette command identities/display names into rmux-core.
Native command registration now references those identities while preserving its
existing order, availability conditions and action dispatch. The browser probe
uses the same identities instead of repeating command labels. Moved unchanged
RankedSearch/tab scoring into core and updated its benchmark import. All 20 core
tests, desktop compile and WASM compile pass. Dynamic workspace/file entries and
platform services remain host-owned; this does not expose unsupported actions
or claim that the full browser application is connected.

Extracted palette panel, heading, search surround, result row styling, footer and
arrow navigation into one shared `palette_view` source consumed by desktop and
the WASM probe. The probe also imports the actual RankedSearch implementation.
Native compile and WASM build pass. Browser check verified compact panel sizing,
fuzzy `splt` filtering and Down/Enter selection of Split Below. Commands remain
fixtures with explicit probe-only feedback, not a port of application actions;
full palette/workspace/terminal integration and native visual review remain pending.

Published v0.1.4 with all six GitHub-built assets. All native jobs succeeded;
publication succeeded, then the Sparkle-feed push collided with a concurrent main
update. Recovered the unchanged generated appcast from the published release.
Added bounded retry that reapplies only the feed when main advances; it never
force-pushes and still fails on permission errors or exhausted retries.

Built and ran the actual shared rmux text field in the browser through GPUI CE.
The isolated host bundles licensed Inter, uses a mise-managed wasm-bindgen loader,
and binds editing shortcuts. Browser interaction verified typing, select-all,
replacement and undo restoration. This is a single real rmux view, not a full web
application; IME, clipboard, workspace/palette/terminal rendering and browser
performance acceptance remain pending. The public showcase is unchanged.

The actual desktop text field now compiles under both native GPUI 0.2.2 and the
isolated GPUI CE 0.3.0 WASM host. Added small host-specific text layout/focus/paint
adapters while retaining one source for editing, selection, IME, undo and visuals.
CE caret mapping uses backend byte-index hit testing and caret bounds. Native
calls preserve the previous implementation. Both compile checks pass; browser
rendering, font loading and interaction acceptance remain pending.

Added an isolated `tools/web-probe` using pinned GPUI CE 0.3.0 and mise-managed
WASM target. It imports the actual desktop text field, composition, undo and
control tooltip sources. The backend dependencies compile for WASM; the shared
field exposes concrete focus/text-shaping/paint API differences. The probe README
records diagnostics and explicitly marks the probe as failing, not a working demo.
Extracted the existing ControlHint unchanged for reuse. Production renderer stays
on GPUI 0.2.2; web runtime/font/input acceptance remains pending.

Started the shared-browser architecture work after confirming Rustable hosts its
actual application UI while rmux's current site is a separate JavaScript showcase.
Moved the existing pane tree, directional navigation and focus history unchanged
into `rmux-core`, consumed by desktop. This is shared behavior groundwork, not a
shared browser UI yet. All 15 core tests and the desktop compile pass.
`docs/browser-ui.md` records the migration path and browser
backend candidates; no renderer dependency has changed.

Prepared v0.1.4 and dispatched GitHub Release run 44915722957. Publication is
pending. Fixed the deployed WASM showcase's stale terminal cells with an explicit
initial reset/clear, reproduced and checked across repeated WASM reuse; hid the
contenteditable browser caret and verified clean workspace switching on rmux.app.
The site remains local-only; it is not the actual Rust UI compiled for the web.

Added `rmuxctl focus-back --window ID` and `focus-forward --window ID` with
optional `--activate`, command help and offline shell completion. They map to
the existing focus-history RPC and require explicit window ownership. A focused
CLI test verifies both directions, activation opt-in, required selector and
unknown-flag rejection. This is unreleased; v0.1.3 remains the published build.

Live focus-history RPC smoke exposed dropped rapid focus visits when relying
only on deferred metadata notifications. Explicit terminal/workspace selections
now record visits immediately; native focus notifications remain supported.
The isolated four-terminal/two-workspace smoke now passes tab/pane/workspace
Back/Forward, workspace.select integration, forward-branch invalidation and
boundary no-op, with automatic process cleanup. Added
`scripts/smoke-focus-history.py` for this focused runtime check. This verifies
logical selection through live RPC on macOS, not visible UI/OS activation or
Linux/Windows acceptance.

Exposed focus history as explicit-window `window.focus_history` RPC with
`back`/`forward`, opt-in activation and moved/destination results. It shares the
native history path and existing busy-window guard; exhausted history is a
successful no-op. Strict parameter validation rejects invalid IDs, directions,
types and extra fields. Focused RPC validation/routing test and desktop compile
pass. Live socket/GUI acceptance is pending; no broad automation parity claim.

Built the current debug app for focus-history integration review. Inspected the
existing terminal focus → pane metadata → workspace metadata notification path,
which records terminal visits after focus changes. Prepared isolated Back/Forward
shortcuts, but native interaction was blocked by the locked Mac; no runtime
focus-history acceptance is claimed. Closed the temporary review process.

Added bounded Back/Forward terminal focus history across a window's workspaces
and pane tabs. Menu, palette and configurable `focus_back`/`focus_forward`
actions reuse existing selection routing; no default bracket shortcuts changed.
History holds at most 64 visits, skips closed targets, preserves forward visits
on navigation notifications and branches on a new focus choice. Two focused
history tests and desktop compilation pass. Native focus/zoom interaction and
cross-platform acceptance remain pending; this is an unreleased W12 slice.

Palette cancellation entries now reflect pending work at opening: workspace
launch, focused-pane tab launch, active-workspace split, and focused-terminal
queued/image paste. Idle cancellation commands are omitted without changing
the underlying menu/shortcut actions or their completion-race guards. State is
sampled before the palette takes focus and requires no polling. Desktop
compilation passes; native busy-operation interaction acceptance is pending.

Palette command/workspace titles and detail rows now highlight contiguous query
words using the existing Unicode-safe preview highlighting helper. Search ranking
and fuzzy matching are unchanged; noncontiguous fuzzy matches may have no marked
span. This reuses merged, original-byte-boundary ranges and virtualized visible
rows. Desktop compilation passes; visual acceptance is pending. Unreleased.

Hardened release publication after the v0.1.3 recovery: explicitly create or
verify the release tag against the built SHA, reject mismatched existing tags,
then invoke release creation with `--verify-tag` and no `--target`. GitHub's
release API documents workflow-write checks when target_commitish differs from
main's workflow files; that matches the observed contents-write token and 403.
The exact built source stays pinned without adding token privileges. Syntax
validation passes; CI acceptance awaits the next actual release.

Published v0.1.3 with all six assets from GitHub run 44868403072: macOS DMG
and ZIP, Linux tar.gz, Windows ZIP, appcast and checksums. Native build jobs
succeeded; the publish job failed twice with GitHub-token 403 responses. Recovered
using the authenticated local GitHub client to upload the unchanged runner
artifacts, then committed the generated Sparkle feed. Confirmed the published
asset inventory and feed enclosure URL/size. The release tag points to cef7bdd,
the built revision. CI publication permissions still need investigation; no
native release was built locally.

Release caching now includes Zig's global cache (`~/.cache/zig`) alongside the
existing source/local-cache and Rust artifacts. The pinned Zig reports that
directory separately from `.deps`, which the previous portable cache covered.
This targets future Linux dependency rebuilds; no speedup is claimed until a
subsequent runner restores it. The in-flight v0.1.3 run is unchanged.

Prepared v0.1.3 with the requested first-impression fixes and dispatched GitHub
Release run 44868403072 (macOS, Linux, Windows). Publication and Sparkle feed
advancement remain pending that run. The local-only Cloudflare site now has a
searchable palette with Cmd/Ctrl+P, arrow/Enter navigation and bracket pane
cycling. Verified filtering and Enter-to-zoom on rmux.app. Content-hashed CSS/JS
prevent old browser assets from mixing with newly deployed markup. Download
links remain on v0.1.2 until the new release exists.

Native palette review passed on the current debug build: Cmd+P opened the
palette, selection spans the panel, shortcuts align on the right, and the
padded search field/footer render without clipping. Everyday actions lead the
list and idle workspace cancellation is absent. Captured the actual window at
`target/palette-review/palette.png` using an isolated configuration; this is
not a release build or proof of full keyboard/layout acceptance.

The command palette now opens with workspace/tab creation, splitting, zoom,
Find, SSH and Settings before diagnostic and group-management commands.
Cancel New Workspace is included only when a workspace launch is pending at
palette creation; its existing execution guard still handles completion races.
Search retains the remaining commands. Focused desktop compilation passes.

Changed the macOS command-palette default to Cmd+P as requested; Linux/Windows
retain Ctrl+Shift+P. Palette rows now explicitly fill the list width so selection
backgrounds and trailing shortcut badges align. Added a padded, bordered search
field, neutral hover color and separated concise footer. Updated user-facing
shortcut documentation. Desktop compilation passes; native visual review of
these unreleased changes is pending.

Added the requested macOS Cmd+[ / ] split-pane cycling, alongside the existing
Cmd+Shift+[ / ] tab cycling. Both pairs appear in the native menu. Shifted
brackets also accept GPUI's macOS `{` / `}` events, which omit the Shift bit;
the previous bindings covered only explicit Shift events. A focused matching
test passes for all six forms and distinguishes pane from tab actions. Existing
cycle handlers wrap within the workspace/pane and preserve zoomed navigation.
These changes are on main; installed-release native acceptance remains pending.

Restored SSH tabs now show a compact Not connected banner with a Connect SSH
action. It opens the saved destination in a new workspace through the existing
guarded asynchronous launch path and preserves the restored local shell. Long
destinations truncate with a full tooltip; non-primary clicks do not launch.
Focused desktop compilation passes; native interaction acceptance remains pending.
Added the requested MIT license
for rmux, inherited Cargo metadata, and license copies in future release bundles.

Finished SSH terminals now label the inherited launch action Reconnect SSH and
offer a separate Local shell action. Both open a new tab and retain the finished
terminal's output; local terminals keep New shell. Existing focus routing and
async sibling-launch paths are reused. Desktop compilation passes; native SSH
interaction acceptance remains pending. The preceding turn prepared organization
avatar/social-preview images; the user chose to upload those personally.

Organized the tab context menu into creation, details, movement and close sections
with thin separators. Separators remain part of action rows, preserving keyboard
selection and scroll indices. New Tab is labeled New SSH Tab for a targeted remote
session, matching the existing inheritance behavior. Focused compilation passes;
the installed release predates these changes, so visual acceptance remains pending.
The preceding turn added bounded CPU sampling rather than changing idle scheduling
without evidence of an unconditional repaint loop.

Inspected terminal redraw scheduling: output waits on channel notifications,
animation-frame retries occur on parser contention, and compression scheduling
stops when complete. No unconditional terminal idle repaint loop was found in
those paths. Added optional app CPU-time delta sampling to the existing release
split harness and exercised it with short windows. Startup variance is retained
in raw evidence, with explicit exclusion from steady-state idle acceptance.
The preceding turn restored native UI observation and recorded a real baseline.

Native inspection now succeeds for the installed v0.1.2 main app. Captured the
actual 4096×2560 single-workspace layout without changing user sessions. Sidebar
selection, directory/shortcut metadata and neutral terminal/tab chrome render;
Metal HUD obscures the upper-right controls. The local screenshot is excluded
from Git and does not validate unreleased controls or full cmux parity. Updated
the visual acceptance notes to remove the stale blanket locked-desktop blocker.
The preceding turn added sibling-tab creation to the context menu.

Added New Tab to the tab context menu, using the targeted tab's existing sibling
launch path for local directory/SSH inheritance. The action is disabled during
another tab launch or shutdown and displays the configured New Tab shortcut.
Menus initially select the first enabled action, so a disabled launch row does
not trap Enter. Desktop compilation passes; native rendered-menu acceptance is
pending. The preceding turn removed the unnecessary macOS runtime-icon change
after the user confirmed a full restart resolved Dock/Cmd+Tab display.

The user confirmed v0.1.2's Dock and Cmd+Tab icon appears after a full restart
following the update. Removed the unnecessary explicit AppKit icon workaround,
restored version 0.1.2, and canceled the proposed v0.1.3 release run. The bundled
icon remains unchanged. Primary-click guards for the new-tab and split toolbar
controls are retained as an independent interaction fix; focused compilation had
passed. The preceding turn completed v0.1.2 publication and website updates.

v0.1.2 is published under rmuxapp/rmux after all native GitHub build and publish
jobs succeeded. Direct artifact inspection confirmed the macOS icon/plist, Windows
PE icon resource group 1 at four sizes, and Linux SVG/desktop entry and executable
permissions. Sparkle feed was updated by CI; website download links now target
v0.1.2. See releases/v0.1.2-verification.md for evidence and runtime limits.
The preceding turn added the post-release pane-zoom toolbar control.

Added a pane-toolbar zoom control using an embedded expand SVG, existing 28px
control hit area, and shortcut-aware tooltip. Clicking focuses that pane before
dispatching the existing zoom action; nonstandard clicks are ignored. The existing
restore-layout strip still provides the zoomed-state exit. Focused desktop check
passes; rendered compact-layout acceptance remains pending. This is on main after
the v0.1.2 release commit, so it is not part of that in-flight release. Windows
v0.1.2 packaging has succeeded; macOS is notarizing and Linux is compiling.
The preceding turn added close-workspace release memory evidence.

Extended the release split harness with optional workspace-close sampling. Nine
fresh-instance runs verified that closing the measured workspace leaves only one
survivor surface. For the 32-pane cases, median app RSS fell from 147.16 to 126.77
MiB after closing and settling for one second. Raw samples and limits are recorded;
this is not a full leak test or child-process cleanup proof. v0.1.2 platform builds
remain live. The preceding turn added release-artifact split/RSS measurements.

Added release-artifact inputs and binary hashing to the split/RSS harness, then
measured the GitHub-built macOS v0.1.1 app through isolated mise installation.
Median app RSS at 1/8/32 idle panes was 115.11/123.91/150.78 MiB after one second;
raw samples and scope limitations are recorded in performance.md. This provides
release-build evidence without claiming frame pacing or zero input lag. The
v0.1.2 native icon release is running on GitHub (44865848547). The preceding turn
made progress by completing the organization transfer and starting that release.

Prepared v0.1.2 with Windows executable icon resource 1 (the ID GPUI loads),
multi-resolution ICO generation, and matching Linux window application IDs.
macOS cargo check passed; native release jobs will verify platform linking.
Documented mise prerelease filtering for issue #2 and verified macOS ZIP
installation plus CLI execution in isolated mise directories, including a
preserved .app layout. Transferred the repository to rmuxapp/rmux, confirmed
release assets and secret names transferred, and updated repository/feed links.
The previous turn made progress by deploying performance-focused website copy.

Linux release packaging now includes the new SVG mark and a maintained desktop
entry with Icon, TryExec and search keywords. Optional per-user launcher setup is
included in LINUX-DESKTOP.txt; portable execution still needs no installation.
Reviewed the packaging paths and parsed the desktop entry, with Python syntax
validation. Linux graphical launcher acceptance remains pending. The preceding
turn integrated the macOS icon; these assets will ship in the next release.

Integrated the new mark into macOS packaging via a multi-resolution ICNS and
CFBundleIconFile, copied before signing for both development and distribution
bundles. An AppKit renderer reproduces the SVG geometry using the existing SDK;
its output was generated successfully and visually inspected. The prebuilt icon
avoids adding release build work. Finder/Dock appearance awaits the next bundle;
Windows executable icon integration remains pending. The preceding turn made
progress by deploying the website and introducing vector branding.

The root-domain Cloudflare site is deployed and its WASM sample terminals were
verified in the browser. Website and Terraform sources remain in separate local
repositories with no GitHub remotes. Added a vector pane-and-prompt logo, a live
demo headline, concise app descriptions, and direct v0.1.1 OS downloads with icons;
download filenames were checked against published release assets. The logo is
used on the website and README; native bundle icon integration remains pending.
Confirmed the existing macOS rmux menu includes Check for Updates when Sparkle
initializes in a configured release bundle; plain cargo builds omit it.

Published v0.1.1 with signed/notarized macOS DMG and Sparkle ZIP, plus Linux
and Windows archives built on GitHub. Reorganized the user guide into a short
task guide and linked topic references. Release caches now use matching
commit-specific restore/save keys on all platforms, preferring the current
dependency/toolchain hash before broader fallback. This allows macOS to save
new compiled workspace state after source changes instead of repeatedly restoring
an immutable older cache; exact retries skip redundant cache uploads. Cache
changes have focused diff review; their time savings await a subsequent release.
The local-only Cloudflare site infrastructure is prepared, but deployment is
waiting for a token with Pages write permission. The preceding turn made progress
by committing that configuration and identifying the API permission failure.

Browser address chrome now follows native navigation/redirect URLs when unfocused
and unchanged since its last synchronization or successful submission. Focused
edits, IME composition and unsent drafts survive metadata updates and blur; Escape
and Cmd+L explicitly restore the page URL. A focused address-policy test covers
those cases, and browser compilation passes. The toolbar background/text now use
the existing neutral pane-chrome palette rather than a separate blue tint; terminal
theme settings remain scoped to terminal content. Native visual/focus acceptance
is pending. The preceding goal turn shipped Stop Loading and its state regression.

Added native Stop Loading to the optional browser. Reload becomes Stop during a
load; Browser menu and Cmd+. expose the same action. The action checks native
loading state to cover metadata-delivery delay, cancels via WKWebView, and clears
queued pre-cancel start/finish events. Separate loading/stopped state prevents a
late finish from replacing cancellation status and preserves process-failure
recovery messages; a new start re-enables normal loading, including coalesced
start/finish callbacks. Eleven focused browser tests pass, covering cancellation,
late finish, reload and process-failure precedence; native slow-load/redirect/error
races remain pending. The preceding goal turn made progress with weak native-view
lifetime instrumentation and correction of the retain-call interpretation.

Corrected the previous turn's native-retention concern after tracing objc2 0.6.5:
Message::retain returns a Retained guard, whose destructor releases the object.
WRY's standalone retain expressions therefore balance immediately and do not
establish a leak. No dependency patch is justified by those expressions. Added
weak native-view lifetime tracking to the composition probe: create/release cycles
can now be sampled manually for total created, still alive and reclaimed objects,
without keeping the objects alive. Dead entries are pruned and 64 still-live
entries cap further creation. This supplies native lifetime evidence alongside OS
memory measurements. The probe builds and links; runtime reclamation remains
unmeasured while the Mac is locked. GitHub browser run 44862240579 passed for the
Find-controls commit; inspector run 44862384089 is still in progress.
The preceding turn made progress by adding Web Inspector controls, but its initial
retention interpretation was incomplete and is superseded by this correction.

Added native Web Inspector toggle controls to the optional macOS browser: toolbar,
Browser menu and Cmd+Option+I target the active browser window, with a status hint
before page creation. WRY devtools support is explicit for browser-enabled release
builds. The host requests inspector closure before releasing its webview. Focused
browser-feature compilation passes; no redundant unit test was added for this thin
native API dispatch. Native inspector/shortcut/cleanup acceptance remains pending
while the desktop is locked. Console/error recording and workspace-pane routing
are still open. Source review also found WRY 0.57.3's macOS drop path explicitly
retains its native webview and manager; the performance plan now records this as a
reclamation gate rather than assuming Rust host drop frees native memory. The
preceding goal turn made progress by shipping browser Find controls.

Added a compact Find bar to the optional macOS browser: Cmd+F/menu/toolbar entry,
Enter or Cmd+G next, Shift+Enter or Cmd+Shift+G previous, case toggle, wraparound,
status and Escape-to-page focus. Search uses native WKWebView find configuration
and callback APIs rather than copying the DOM. Requests are limited to one at a
time; generation checks discard stale results after editing, close, navigation
or process failure. The callback captures no browser owner and uses the existing
bounded metadata channel. Matching-result counts are not exposed by this API.
Ten focused browser tests pass, including a new stale-result/request-bound test,
and the browser test executable links. Native visual/focus/highlight acceptance
was attempted but the Mac remains locked; screenshot evidence is unavailable.
GitHub shell-integration run 44861923760 also passed for the preceding PowerShell
commit. This is standalone-window progress, not browser workspace-pane parity. The preceding
goal turn made progress by shipping PowerShell command-start/completion hooks.

PowerShell now emits command-start (OSC 133 C) after its existing console reader
returns nonempty input, and a single completion (D) at the next prompt. The wrapper
delegates editing/history to the original PSConsoleHostReadLine function, following
the host integration point also used by
[VS Code](https://github.com/microsoft/vscode/blob/main/src/vs/workbench/contrib/terminal/common/scripts/shellIntegration.ps1).
Completion intentionally omits an exit code rather than reusing stale LASTEXITCODE;
the original prompt still runs first to preserve its view of $?. Focused runtime
checks pass for reader delegation, one-time start/completion, empty/canceled reads
and failed-command prompt status. The real macOS PTY fixture also confirms the
wrapper executes before a command while directory/prompt navigation still works.
Windows, complex themes and native history/completion interaction remain pending.
The preceding goal turn made progress by shipping automatic PowerShell startup.

PowerShell tabs now load prompt/directory hooks automatically after ordinary
profiles using a private temporary script retained for the session lifetime.
The script path is passed as environment data, never interpolated as code;
the wrapper removes its environment variable and leaves execution policy alone.
RMUX_SHELL_INTEGRATION=0 retains ordinary startup. The mise-managed PowerShell
7.5.4 real-PTY test passed on macOS: custom profile prompt, wrapper environment
cleanup, Unicode/space/percent directory reporting, scrollback prompt navigation
and clean exit. Windows and native PSReadLine/theme acceptance remain pending;
command-start/completion hooks remain open. The preceding turn made progress by
updating the authorized GitHub notarization issuer secret.

Added opt-in PowerShell prompt/directory integration and bundled its script on
macOS. It invokes the original prompt first, then returns escaped filesystem OSC 7
and OSC 133 A/B boundaries; non-filesystem providers omit file URIs. Repeated sourcing
is idempotent and RMUX_SHELL_INTEGRATION=0 opts out. Installed PowerShell 7.5.4 through
mise only and ran the focused runtime fixture successfully: original prompt text,
failed-command $? visibility, LASTEXITCODE, Unicode/space/percent paths, duplicate
source and non-filesystem handling pass. The test is pinned in a mise task and a
Linux GitHub job; no Windows validation was added. Automatic injection, PSReadLine/
interactive PTY compatibility and command start/completion remain open. The previous
goal turn made progress with corrected PowerShell Unix login argument ordering.

Reviewing PowerShell integration exposed an existing Unix launch-order bug:
PowerShell requires -Login first, but the common launcher appended -i then -l.
Recognized pwsh basenames now receive -Login/-Interactive in the documented order;
non-login pwsh gets -Interactive, and other shells keep their existing flags.
The focused argument-policy regression passed for path spaces, case variants,
non-login mode and unrelated shell names. No tool was installed, no user profile
was changed, and native PowerShell/Windows execution was not validated. Prompt/OSC
hooks remain a separate open step. Reference: Microsoft's about_Pwsh -Login docs
(https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_pwsh?view=powershell-7.5#-login--l).
The previous goal turn made progress with snapshot recovery diagnostics and CI.

Opt-in snapshot_recovery diagnostics now count placeholder prepaints and completed
wait intervals from first placeholder to fresh engine frame. Total/max nanoseconds
exclude time after hiding the pane again; normal frames without a preceding stall
do not count. The focused synthetic-timeline/serialization test passed and desktop
compilation succeeded. These counters are not input-to-photon measurements; native
refocus/blank-frame acceptance remains open. GitHub browser CI now also runs the
focused row-cache and diagnostics regressions. Its 9893146 run completed successfully;
d011765 is active, superseding the previously queued cb3786d run. The prior turn
made progress with committed/pushed logical snapshot-release accounting.

Opt-in render metrics now include cumulative snapshot_release_totals (frames, cell
capacity and heap text payload bytes) recorded before hidden desktop snapshots are
released. The existing retained-snapshot flag prevents repeated accounting while
hidden, and storage scanning runs only with diagnostics enabled. Counters saturate
and are explicitly logical, non-deduplicated totals rather than claimed RSS savings.
The focused accounting/serialization regression passed and desktop compilation
succeeded. Native RSS/refocus measurements remain pending. GitHub browser CI for
18f5012 completed successfully; 9893146 is active and cb3786d is queued at the last
observation. The preceding goal turn made progress with committed/pushed hidden
snapshot reclamation.

Hidden terminal cache cleanup now also releases desktop-owned frame cell snapshots,
using shared empty-row placeholders that preserve viewport dimensions. A retained
snapshot flag makes cleanup idempotent even when shaping never occurred; successful
prepaint snapshots restore the flag. Engine contents/scrollback/processes remain
untouched. Eight focused row-cache tests passed, including engine-text continuity,
dimension preservation and weak-reference proof that placeholder rows do not hold
old cells. Desktop compilation passed. A busy engine on refocus can briefly paint
background until the existing nonblocking retry obtains a frame; native refocus,
blank-frame UX and RSS measurements remain acceptance work. The previous turn made
progress with committed/pushed compact browser controls. GitHub's f6ba628 browser
run completed successfully; subsequent source revisions continue through CI.

The experimental browser toolbar now shares rmux's compact 28-pixel navigation
controls and shortcut-aware tooltips. A bordered address field, 44-pixel toolbar
and 24-pixel status row leave more room for page content; the separate zoom label
resets to actual size, and status/URL text has a full-detail tooltip. Go and Enter
now share one IME-aware submission path with identical pending/error status.
Browser-enabled compilation passed. Native spacing, glyph appearance, tooltip
stacking against WKWebView, clipping and input acceptance remain pending. The first
GitHub browser run 44859718234 completed successfully (tests, probe link and default
dependency isolation); f6ba628's run 44859910910 is active and 18f5012's run is pending.
The previous goal turn made progress by separating browser view/window ownership.

Separated browser content into a BrowserView GPUI entity owned by a small standalone
BrowserWindow wrapper. Native webview/control/metadata ownership stays with the
view; window title and close policy stay with the wrapper, which subscribes to
metadata updates. This is groundwork for pane ownership, not a claim of mixed
surface tabs: PaneView still stores terminal entities, and browser focus/visibility/
transfer/restore integration remains open. Nine focused browser tests and feature
compilation passed. GitHub run 44859718234 has reached cache saving after its test,
probe-link and default-dependency checks; final run completion is still pending.
The prior goal turn made progress with committed/pushed blocked-navigation feedback.

Experimental browser navigation now applies URL length/control/credential policy
to page-initiated requests as well as explicit input. Blocked navigation, popups
and downloads show concise static feedback rather than silently doing nothing or
echoing secret-bearing URLs. Notices coalesce through the bounded mailbox, clear
on new navigation and cannot mask a content-process failure. Nine focused browser
tests passed, including notice deduplication/reset and failure precedence. The
existing GitHub run 44859718234 is confirmed live in browser test/build execution;
no result yet. CI now lets active runs complete and keeps the latest pending run,
so rapid pushes do not repeatedly abandon a cold build before caching. Native
interaction acceptance remains open. The preceding turn made progress by adding
and starting the GitHub browser workflow.

Added GitHub-hosted macOS CI for the optional browser feature, using the existing
pinned mise/Xcode runner setup. Relevant pushes/PRs run focused browser tests, link
the native composition probe, and check that WRY is absent from default builds.
The workflow has read-only repository permissions, no signing/publish steps,
separate compilation caches and cancellation of outdated runs. Pushed as 248f09c.
The first run, https://github.com/rmuxapp/rmux/actions/runs/44859718234,
is confirmed in progress (mise setup at last observation); its result is pending.
No hosted-build success is claimed yet. The previous goal turn made progress with
committed/pushed browser metadata throttling.

Browser metadata publication now has a 100 ms minimum interval during bursts,
independent latest-value coalescing and change-only redraw notification. A page
rapidly changing its title can no longer notify rmux chrome on every callback.
The timer starts only after an event and does not poll while idle; native page
rendering/callback work is outside this bound. Extracted PageState applies batches
without copying current strings, preserves termination across late finish/title
callbacks, and reports whether visible chrome changed. Eight focused `browser_`
tests passed, including duplicate/empty-batch suppression and cross-batch failure/
recovery state. Feature compilation passed. Native callback-storm CPU/latency and
adjacent-terminal acceptance remain pending. The prior turn made progress with
committed/pushed browser zoom controls.

Experimental browser windows now have per-window 25–500% page zoom with discrete
steps, Cmd+=/Cmd+-/Cmd+0 bindings and native Browser menu actions. The status line
shows the current percentage; actual size resets to 100%. Native zoom failures
retain the previous setting, and pre-navigation choices apply when creating the
webview. The focused zoom bounds/reset test and browser-enabled compilation passed.
A fresh desktop availability check still reports the Mac locked, so native zoom,
shortcut/focus, clipping and adjacent-terminal acceptance remain open. B09 now
records this partial zoom implementation without claiming screenshots/viewport/
highlighting support. The preceding turn made progress with committed/pushed
browser process-recovery UI.

The experimental macOS browser now observes WRY content-process termination and
shows a clear reload recovery message, removes stale page titles and labels the
reload control Reload Page. Failure state persists across late finish/title events;
a new load-start event restores normal loading status. No automatic retry loop is
introduced. The callback reuses the bounded coalescing mailbox. Six focused
`browser_` tests passed, including termination/late-completion/new-load event order;
the browser-enabled desktop compiled. Native forced-termination/reload and memory
recovery remain unverified while the Mac is locked. The preceding goal turn made
progress with committed/pushed bounded browser titles.

Experimental browser windows now show document titles in native window labels,
retaining the rmux/experimental suffix. Titles are limited to 256 Unicode characters
and control characters become spaces. Page/title callbacks share one queued wakeup
while retaining independent latest values, so a title burst cannot overwrite load
status/URL updates. New loads clear stale titles; empty titles use the app label.
Five matching `browser_` tests passed, including a new Unicode/title-bound and
coalesced-load-state regression; the browser-enabled desktop compiled. Native
window-title and callback-order acceptance remains pending. The previous goal turn
made progress with committed/pushed terminal-to-browser link opening.

Browser-enabled terminal context menus now expose Open Link in rmux Browser.
The menu retains its captured hyperlink target, uses the browser's scheme/credential/
length policy for enablement, and reports opening failures through the terminal
error display. The new window starts with the URL in its address field and defers
native creation/navigation until measured bounds exist. The normal menu shape and
external Open Link/modifier-click behavior remain intact. Four focused `browser_`
tests passed, including bounded/control/credentialed terminal-link rejection, and
the browser-enabled desktop compiled. Native context-menu/focus/first-navigation
acceptance remains pending. The previous turn made progress with committed/pushed
local development-server URL normalization.

Browser address entry now defaults bare localhost/subdomain-localhost and IPv4/IPv6
loopback addresses to HTTP, while retaining HTTPS for other hosts and preserving
explicit schemes. Reparsing the original authority retains explicit port 443 when
switching the implicit scheme; URLs inside query/path text no longer confuse scheme
detection. Two focused address tests passed, covering local ports, IPv6, numeric
IPv4 normalization, trailing-dot localhost, mixed-case explicit HTTPS, lookalike
hosts, LAN defaults, embedded URL queries and malformed/credentialed addresses.
This verifies normalization and feature compilation, not native HTTP navigation.
The prior goal turn made progress with committed/pushed browser keyboard/history
controls. Native/browser-pane/cross-platform acceptance remains open.

The experimental browser now exposes scoped Cmd+L/Cmd+R/Alt+Left/Alt+Right bindings
and native Browser menu actions routed only to the active browser window. Address
focus selects the page-load URL; Escape restores that address and returns focus to
the webview. Back/Forward query native history availability and dim/no-op when
unavailable; Reload is disabled before webview creation. Native parent focus is
requested before focusing the GPUI address field. Action routing preserves active
address IME composition. The browser-enabled desktop test compilation and existing
URL policy regression passed; native keyboard/menu/focus/history interaction is
not verified while the Mac remains locked. The previous goal turn made progress
with committed/pushed experimental browser-window navigation.

The optional macOS `browser` feature now exposes an experimental browser window
inside rmux via File and the command palette (`mise run browser`). It has a native
GPUI address field and Back/Forward/Reload/Go controls, creates an ephemeral WRY
webview on first navigation, and updates a committed-URL/load status through a
single coalesced wakeup. URL parsing accepts HTTP/HTTPS/about:blank, defaults bare
hosts to HTTPS, and rejects embedded credentials; popup/download/external-scheme
flows await implementation. A focused URL policy test and feature compilation
passed. Native runtime acceptance is pending while the Mac is locked. These are
separate windows, not workspace surfaces, and are not restored in session snapshots.
The previous goal turn made progress with the committed/pushed composition probe.

Started the P3 browser backend composition work with an opt-in macOS GPUI/WRY
example, launched through `mise run browser-probe`. WRY 0.57.3 is pinned behind
`browser-probe`; default dependency-tree inspection confirms it is absent from
normal builds. The example lazily creates an ephemeral local HTML webview,
positions/resizes it from GPUI bounds, and offers explicit release/recreation and
sidebar-width controls. The macOS debug example compiled and linked successfully.
No interactive/native, helper-RSS or busy-terminal performance claim is made: the
Mac remains locked. Actual pane integration and cross-platform backend acceptance,
especially Wayland composition, are open. Architecture notes record the prototype
and concrete next gate. The previous turn made progress with committed/pushed
Markdown table alignment.

Markdown table headers/body cells now honor left, center and right column markers.
Only the eight visible columns retain alignment metadata in each row; the total
column count and complete row/source text remain intact. Alignment resets for each
table, avoiding leaked styles when a wide aligned table precedes a narrow/default
one. The focused `table` test filter passed all three matching tests, including
mixed alignments, header/body agreement, reset and a 100-column bounded fixture;
desktop compilation passed. Native alignment/wrapping/spacing acceptance and
horizontal table scrolling remain pending. The previous goal turn made progress
with the committed/pushed structured-table implementation.

Markdown tables now parse into bounded virtualized header/body rows, preserving
empty cells and inline styling. The reader draws equal-width separated cells,
bold headers and search highlights. At most eight columns per row are displayed;
wider tables have an explicit first-eight/copy-for-all label. Full tab-separated
row text remains searchable/copyable and original Markdown source remains intact.
Cell text sanitizes embedded tabs so they cannot create spurious columns. Eleven
Markdown tests passed, including Unicode cell style offsets, empty rows, literal
code tabs, visible-cell bounds and the 512-block cap. Desktop compilation passed.
Alignment markers, horizontal scrolling and native visual acceptance remain open.
A fresh desktop availability check confirmed the Mac is still locked. The prior
goal turn made progress by committing/pushing selection-cache reclamation.

The terminal renderer now releases selection-specific glyph/decoration variants
when a row leaves the selection, the selection clears, or its foreground setting
changes. Previously variants remained attached to unchanged shaped rows until a
content/style replacement or hidden-pane cache release. Still-selected rows retain
their variants; no PTY/engine/scrollback state changes. Seven focused row-cache tests
passed, including selected-variant retention, deselection, shortened selection
arrays and color clearing/replacement. This proves ownership cleanup, not measured
RSS savings or native selection-drag latency; those remain acceptance work. The
previous goal turn made progress with committed/pushed monospace Markdown runs.

Inline Markdown code now uses the terminal's monospace family within otherwise
proportional text. A render-time text component resolves inherited row styling
before creating byte-aligned runs, preserving heading weight/size, nested emphasis,
strikes and search colors without changing excerpt bounds or copying. Four focused
inline tests passed, including Unicode run coverage, surrounding-font restoration,
heading-weight retention and search/strike composition. Native font shaping and
visual/clipping acceptance remain pending. The previous goal turn made progress
with the committed and pushed inline-code/strikethrough implementation.

Markdown inline code now receives a shaded background, and the parser enables
strikethrough with nested span retention. Search colors override code shading
without discarding strike/bold/italic attributes. Code delimiter contents remain
literal, style state ends at its matching event, and copied text stays unchanged.
All ten Markdown tests passed, including a new nested-code/strike/search regression;
desktop compilation passed. Native visual acceptance, inline-code monospace font
selection, full document layout and assets remain pending. The preceding goal turn
updated the authoritative GitHub notarization secret and therefore made progress.

Markdown reading blocks now retain nested bold/italic spans. Excerpt projection
maps original UTF-8 ranges through control-character sanitization, clipped-prefix
ellipses and list/task markers. Search colors and emphasis merge into sorted,
disjoint renderer runs so one does not overwrite the other; copy text is unchanged.
Seven existing Markdown tests passed, followed by two new focused regressions for
nested/reset styling and Unicode/control/search-overlap projection. Desktop test
compilation passed. Native shaping/style/clipping acceptance and remaining inline
code/strikethrough/table/asset rendering are pending.

Markdown/source switching now has the configurable `toggle_markdown_preview`
action with Ctrl+Alt+V in file previews. Active-key hints reflect overrides, and
searchable Read Markdown / View Markdown Source commands complement the toolbar.
The handler respects IME composition and reuses bounded asynchronous loads and
mode-specific refresh-state rules. The focused keymap test passed for disable/
remap and exclusion from terminals, file browsers and Settings, with desktop
compilation. Native switching/focus/compact-layout acceptance remains pending.

Fixed Markdown block-cap reporting: documents containing exactly 512 reading
blocks no longer become partial solely because closing parser events remain.
The cap is enforced when another nonempty block would be emitted, including
horizontal rules, without exceeding retained block bounds. The focused regression
passed for exact-cap headings, list items and quotes, and overflowing text/code/
rules. Source-byte/source-line truncation remains independently reported. Native
reading-view acceptance and richer document rendering remain pending.

Markdown task-list parsing is now explicitly enabled. List metadata preserves
checked state independently of body text, including nested/ordered tasks and
loose continuations. Rows show read-only checkbox markers plus searchable
Pending task / Completed task labels, while copy and source-copy semantics stay
separate. Six Markdown regressions passed, including unchecked/lowercase/uppercase
checked markers, nested depth, ordered starts and literal bracket text outside
lists. Native checkbox glyph/layout acceptance remains pending.

Markdown reading blocks now preserve H1–H6 identity, including Setext headings.
Heading rows use a font-size hierarchy, searchable level labels and two-line
wrapped excerpts with matching row heights for paging/virtualization. Other
block excerpts retain their previous sizes. Five Markdown parser regressions
passed, including all ATX levels and both Setext forms, with desktop compilation.
Native heading spacing/clipping and full document appearance acceptance remain
pending.

Markdown code excerpts now preserve separate source lines and indentation in
wrapped mode, instead of flattening all whitespace into prose. Four bounded
monospace rows are selected around the first contiguous query-word match; tabs
expand at four-column stops with Unicode width accounting, and omitted content
is marked. Copy/search retain original block text. Four Markdown regressions
passed, followed by the focused updated code-excerpt test for wide/combining text.
Native line layout/clipping and full code-block scrolling remain pending.

Markdown reading blocks now retain list depth, ordered start/next numbers and
item-marker identity. Rows display numbered/bullet markers and bounded nested
indentation; loose continuation paragraphs do not repeat markers. Nested list
transitions flush parent content before changing context and restore parent-item
state afterward. Three parser regressions passed, including ordered starts,
nested bullets, loose continuations, bounds and inert content. Native list layout
and full document rendering remain pending.

Started F03 with an explicit Markdown reading mode from file previews, using
pinned pulldown-cmark 0.13.6 without default features. Parsing runs on the bounded
file worker; the existing source cap and a 512-block cap limit retained work.
Virtualized rows distinguish headings/code/list/quote blocks, support search and
block copy, preserve source copying, and offer View Source plus mode-correct
refresh. Raw HTML, link labels and image alt text remain inert. Two parser tests
passed for structure/text/asset handling and block caps, with desktop compilation.
Native visuals, full inline/code layout, tables, links/images and appearance
controls remain pending; F03 is in progress, not accepted.

External-editor invocation now preserves the originating file browser/preview
instead of closing its palette and restoring terminal focus. The existing view
retains query, selected entry/line, wrap mode and scroll state across editor
launch, including failed/missing-editor requests; Refresh remains available on
return. All external-edit entry points share this event path. Desktop compilation
passed. Native UI access was rechecked and the Mac remains locked, so actual
editor activation/return-focus and visual acceptance remain pending.

Settings now exposes a keyboard-navigable External Editor configuration row,
with configured executable summary/full-path tooltip and direct Settings JSON
access. Executable paths containing NULs fail configuration validation before
launch. Focused reload coverage verifies valid setup, invalid relative/NUL paths
and missing placeholders, retention of previous valid settings, and removal of
the editor configuration. That regression and Settings navigation test passed;
native configuration-row layout/focus and external editor acceptance remain open.

External editing now has an `edit_selected_file` action with Ctrl+Alt+E defaults
in the directory-browser and file-preview scopes. It resolves the current
preview or selected regular-file row, ignores directory/symlink rows and IME
composition, and reuses the launch/error/lifecycle path. The browser button shares
the action handler; visible navigation hints reflect remapped keys. The focused
scope/remapping regression passed with desktop test compilation. Native shortcut,
editor launch/focus and layout acceptance remain pending.

External-editor reaping now reports unsuccessful exits and status-collection
errors to the originating window, instead of silently treating every spawned
process as successful. Reaping remains asynchronous and independent of preview
closure. Two focused tests passed, including real Unix child-process execution
with literal space/quote/shell-punctuation filenames, success/nonzero exit and
directory/symlink rejection. These are launcher-path checks, not native GUI editor
focus/file-loading acceptance; successful detached launchers cannot prove that
another application opened the document.

Started F02 external-editor integration. Directory selection and text/hex previews
now expose External Editor using explicit file_editor program/argv configuration.
Paths remain one OS argument (including non-UTF-8 paths); no shell interpolation
or file-association execution is introduced. Regular-file checks and spawning run
off the UI thread. Eight active launches are permitted; asynchronous timer-based
reaping avoids a blocked worker per editor. Argument/validation regression passed
with desktop test compilation. Native launch/focus acceptance, embedded editing,
safe saves and external-change conflict handling remain pending.

Same-directory refresh now retains the filter and selected entry by path/type,
rather than unstable row position. Ranking updates before deferred selection
restoration; removed, type-changed or nonmatching entries fall back to the first
result. Different-directory/history navigation starts fresh, and cancellation
clears retained refresh state. Selection lookup borrows paths instead of cloning
one per candidate. The focused reorder/removal/type-identity regression passed
with desktop test compilation; native refresh/focus/scroll acceptance is pending.

Pending file/directory/SSH-host loads can now be canceled through File, the
command palette or a custom `cancel_file_loading` shortcut. The notice button
shares the same handler: signal the worker, discard the UI task and preview
restore state, and clear only the matching loading notice. In-flight OS calls
retain their worker permits until returning. The existing pre-filesystem
cancellation regression passed with desktop test compilation. Native menu/key
acceptance and cancellation during blocked OS reads remain pending.

Directory browsing now has configurable `files`-scoped defaults for Back/Forward,
Parent, Refresh and location entry. Navigation works independently of list
filtering, follows existing asynchronous/history paths and ignores unavailable
history, root-parent requests and IME composition. Visible hints resolve the
active keymap. The focused remap/scope test passed with desktop test compilation;
native text-field dispatch, focus and compact navigation layout remain pending.

File previews now have scoped keyboard refresh (F5), directory return (Alt+Up)
and text/hex switching (Ctrl+Alt+M), available even with a filtered list. The
`file_preview` shortcut context supports remapping/disabling all three in text
and hex modes; current keymap hints appear alongside the persistent navigation.
Actions share existing async preview requests and refresh-state preservation,
with IME guards. The focused scope/remap test and desktop compilation passed.
Native keyboard/focus/compact-layout acceptance remains pending.

Replaced the hardcoded wrap-excerpts key handler with a registered action and
text-preview-only shortcut scope. `toggle_preview_wrap` accepts custom bindings
under `context: preview`; Ctrl+Alt+W remains the default and can be disabled.
Button hints resolve the active keymap instead of advertising a stale key.
The focused keymap test passed for disable/remap and exclusion from Terminal,
ordinary CommandPalette and Settings contexts, with desktop test compilation.
IME guarding remains in the shared action handler; native key dispatch and
visual hints remain pending.

Same-file/same-mode preview refresh now retains the query, wrapped-excerpt mode
and selected command/line identity. Input updates rebuild ranking before deferred
selection restoration; removed/nonmatching lines fall back to the first result.
State is captured before closing the old palette, consumed by the next preview
request and never carried into a different path or text/hex mode. Existing async
focus/cancellation gates still control publication. Desktop compilation passed;
native refresh/query/scroll/focus acceptance remains pending.

File-preview search now highlights contiguous query-word matches in both compact
and wrapped excerpts, including hex preview text. Lowercase expansion maps back
to original UTF-8 scalar boundaries; overlapping/adjacent highlights merge. Work
is restricted to rendered, bounded excerpts and empty queries skip preparation.
Copy source text and fuzzy ranking remain unchanged. The focused Unicode/range
regression passed with desktop test compilation; native highlight appearance
and wrapped-text visual acceptance remain pending.

Fixed memory-tree completeness during process arrivals. Totals now compare
before/after membership and count newly observed descendants as missing samples,
so a child created during a refresh cannot leave an understated total labeled
complete. Departures and PID-generation replacements remain excluded from RSS;
changed membership withholds aggregate CPU and uses the existing partial-count
UI/JSON. Five active sampler tests passed (three manual benchmarks skipped),
including deterministic arrival/departure/replacement/unchanged cases. Native
panel acceptance and broader process-churn performance remain pending.

Extended process-tree sampling measurements to live child workloads: each of
1/8/32 shells owns four idle children, with explicit startup readiness and
parent-managed child cleanup. The ignored mise benchmark checks complete tree
ownership, exact sample counts and valid CPU baselines. Warm debug medians were
42.014/41.833/50.199 ms over five samples per size; raw summaries and limitations
are recorded in performance.md. Native UI remains locked, confirmed this cycle;
no visual/interaction acceptance is claimed. Deep/churning/capped trees and
release/UI performance remain open.

Text previews now offer wrapped excerpts through a persistent control and
Ctrl+Alt+W. Up to four visual lines per row improve readability while retaining
bounded excerpts and virtualized rendering. Toggling resets measured row sizes,
keeps selection/search and updates paging distances; copying retains original
captured text. Hex rows keep their fixed layout. Desktop compilation passed;
native wrapping/keyboard/scroll acceptance remains pending. This is preview
readability work, not completion of the F02 file editor.

Imported palettes now cover all 256 indexed colors. Terminal creation, async
preparation/stale-settings checks and live reloads use the complete palette;
ANSI JSON overrides retain priority for slots 0–15 without erasing imported
extended entries. Missing extended entries and replacement imports resolve from
cached engine defaults. Engine regression coverage confirms existing-cell updates,
truecolor preservation, OSC override priority/reset and default reset. Thirteen
focused desktop/engine tests passed; native extended-color visuals remain pending.
Settings swatches continue showing the 16 ANSI colors.

Dynamic selection foregrounds now complete cell-reference support for imported
cursor and selection colors. Selected text runs split at background changes when
using cell-background; underlines and cached overline spans follow the same
resolved colors. Explicit JSON values override imports, and Settings preview
resolves both references. Alternate rendering remains lazy and bounded to one
variant per cached visible row. Sixteen selection-related tests passed, including
mixed-background run boundaries, concealed text, imports and override round-trips.
Native inversion/decorations/drag-performance acceptance remains pending.

Selection backgrounds now accept dynamic cell-foreground/cell-background in JSON
and imported themes, with explicit settings taking precedence. Painting merges
adjacent matching cell colors into spans without allocating a background buffer;
literal/default selections retain a single-quad fast path. Settings preview uses
the same cell-color resolution. Fourteen selection-related tests passed, including
new import/override/serialization and mixed-color span/clipping cases. Native
selection visual acceptance and dynamic selection foregrounds remain pending.

Cursor fill now supports dynamic cell-foreground/cell-background references in
JSON and imported themes. Dynamic settings clear the engine's configured cursor
default; snapshots expose whether an explicit engine color is present so
application OSC cursor overrides retain priority until reset. Rendering and
Settings preview resolve references against their current cell colors. Five
focused tests passed for source flags/OSC reset, dynamic fill/text precedence,
literal color round-trips and import/reload/save behavior. Native cursor visuals
and dynamic selection-color references remain pending.

Block-cursor text now supports `cell-foreground` and `cell-background` in imported
themes and JSON settings. References resolve against the current engine cell,
including application color changes, and the single-glyph cache still keys the
effective result. Settings preview shares this resolution. Two focused tests
passed for literal/import fallback, dynamic cell changes, explicit precedence,
serialization and invalid references, with desktop test compilation. Native cursor
visuals and dynamic cursor-fill/selection references remain pending.

Settings now previews block-cursor glyph color and selected text alongside terminal
colors and ANSI swatches. Selection fill resolution is shared with actual terminal
painting, retaining the same translucent default. Override messaging includes the
new cursor/selection fields, and imported filenames expose full paths on hover.
The sample remains configuration-only with no PTY, engine creation or polling.
Desktop compilation passed; native preview spacing/contrast and hover acceptance
remain pending.

Palette assignments now use the pinned Ghostty parser for decimal, hexadecimal,
octal and binary indices and literal colors, while retaining rmux's 0–15 ANSI
limit. The bounded FFI wrapper rejects oversized/control-containing entries.
Also replaced the temporary terminal used for factory palette discovery with
Ghostty's direct default-palette API: only the resulting 16 colors are cached.
Four focused engine/import tests passed for numeric bases, invalid/overflow
indices, sparse inheritance, factory-color agreement and OSC/extended-color
preservation. Native visuals and extended palette overrides remain pending.

Theme import now delegates literal color parsing to the pinned Ghostty engine,
adding X11 names, short/wide hex and rgb:/rgbi: specifications without a duplicate
color-name table or temporary terminal. The Rust boundary caps input at 256 bytes
and rejects controls; theme errors retain line context. Two focused engine/import
tests passed for named/hex/RGB values, malformed values, controls and bounds.
Explicit rmux JSON colors remain strict #RRGGBB. Dynamic cell-color references,
extended palette indices and native theme visuals remain pending.

Theme imports now accept partial ANSI palettes. Missing slots inherit the selected
built-in preset; Dark resolves the pinned engine's factory palette through a
one-time tiny terminal allocation without a PTY, then caches only 16 colors.
Resolution happens during background import/reload, never painting, and a new
import cannot inherit stale entries from the previous import. Explicit JSON
palettes retain highest precedence. Two sparse-import tests and the engine ANSI
palette/OSC override regression passed, including inherited slots, replacement
imports, factory-color agreement and extended-color preservation. Native import
and palette visuals remain pending.

Added imported `cursor-text` and explicit JSON `cursor_text` for block-cursor
glyphs. Explicit settings override imports; absent values preserve the cell's
background fallback. The one-glyph cursor cache keys the effective color so reloads
cannot reuse stale glyph colors; row shaping is unaffected. The focused import,
override, serialization and preset-fallback test passed with desktop test
compilation. Native block-cursor visuals and dynamic Ghostty color references
remain pending.

Added imported/explicit selection foreground colors. Prepaint lazily shapes one
alternate text variant per selected row, reusing it across selection movement;
changing/disabling the color clears variants, and row-cache release drops them.
Normal and selected variants paint in disjoint masks to avoid antialiased color
bleed. Existing shaping still excludes concealed cells and preserves font styles;
selected underlines/overlines follow the override. Desktop compilation, the
selection-color precedence/persistence test and four row-cache tests passed.
Native glyph clipping, decorations, drag latency and memory acceptance remain
pending: the Mac was confirmed locked. Alternate shaping is opt-in and bounded by
visible cached rows, not scrollback.

Added imported `selection-background` and explicit JSON `selection_background`.
The renderer resolves the configured color once per paint and applies it to the
existing selection quad, without changing shaped-row cache identities or copying
text. Explicit values override imported values; no configured value retains the
translucent blue default. Two focused tests passed for import parsing, override
precedence, serialization and clearing an import. Selection foreground styling,
native selection visuals and contrast acceptance remain pending.

Added Import Terminal Theme to the menu, command palette, keyboard-navigable
Settings row and custom `import_theme` action. The native single-file picker uses
the shared prompt gate; bounded theme parsing runs off the UI thread under the
settings-operation gate. Invalid/canceled imports preserve settings, newer edits
win over delayed results, and a closed originating window cannot publish an import.
Successful imports apply to the session and persist only with Save. Desktop
compilation, import/override/save test and Settings navigation test passed. Native
picker/focus/visual acceptance and broader Ghostty theme syntax remain pending.

Added configuration-driven `theme_file` import for a strict Ghostty-style color
subset: foreground/background/cursor RGB and complete 16-entry ANSI palettes.
Reads are explicit at startup/reload, capped at 64 KiB and regular-file only;
unsupported settings/colors fail with line context. Explicit rmux overrides win,
then imported colors, then built-in defaults. Settings displays the import and
choosing a preset clears it; saving can recover from a broken imported file.
Three focused import tests passed for parsing, bounds/nonregular files, precedence,
reload failure retention and preset persistence. Also repaired an older row-cache
test initializer and passed its release test. Native theme reload/visual acceptance,
interactive import picker and broader Ghostty theme syntax remain pending.

Fixed Bash prompt-hook detection for exported indexed arrays (`declare -ax`).
Attribute-aware detection preserves array elements and order instead of applying
the scalar rewrite to element zero. The fixture now checks plain/exported array
structure and source-twice idempotence on all Bash versions, plus user-hook order
and command-status preservation on Bash 5.1+. Local Bash 3.2 compatibility and
[GitHub Linux Bash 5.2.21 run 44851031041](https://github.com/rmuxapp/rmux/actions/runs/44851031041)
passed. Native prompt-theme acceptance and automatic login injection remain open.

Added a path-filtered GitHub Linux shell-integration workflow with pinned actions
and mise-managed Python, using the runner's existing Bash. Actionlint passed.
[Run 44850857084](https://github.com/rmuxapp/rmux/actions/runs/44850857084)
passed on Bash 5.2.21, including PS0 command-start emission before command output,
user PS0 preservation, idempotence, prompt replacement, cwd encoding, command exit
status and opt-out. The macOS Bash 3.2 compatibility fixture also passed locally
in the prior iteration. Automatic Bash login injection, modern array-hook coverage
and native shell/prompt-theme acceptance remain open.

Bash 4.4+ integration now prepends the OSC 133 command-start marker to PS0,
preserving user content and relying on the existing source-once guard. Older
Bash leaves PS0 unchanged, and no DEBUG trap is installed. The focused shell
fixture passed on macOS Bash 3.2, covering compatibility, prompt hooks/status,
encoded cwd, opt-out and idempotence. A conditional fixture checks modern Bash
PS0 preservation and emission before command output when that shell is available;
modern-runtime acceptance is still pending. Automatic login-shell injection is
still unresolved; existing non-login/manual loading paths carry this change.

`pipe-pane --selection` now feeds the existing bounded terminal selection export
to its local consumer, with optional `--preserve-wraps`. It leaves focus and
selection unchanged and rejects `--lines` conflicts in either option order.
The same 1 MiB limit and truncation/error execution gate apply. Four focused
CLI/pipe tests passed, including scope expansion, conflicting flags, history-tail
validation and subprocess delivery/status. Help, completions and user guidance
are updated; native clipboard/selection and cross-platform acceptance remain open.

`pipe-pane --lines N` now captures only the requested tail of retained history
before starting its local consumer. It reuses the existing logical-line RPC path,
accepts 1–65536 lines and retains the 1 MiB cap and reject-truncated-before-execution
behavior. Help, completions and user documentation cover the option. Three focused
CLI/pipe tests passed, including request expansion, invalid/duplicate counts,
separation of shell command from RPC payload and local subprocess delivery/status.
Implicit target selectors, JSON output wrapping and Windows execution remain gaps.

The shaped-row cache now recognizes the identical immutable outer row snapshot
before walking per-row identities. Repaints with unchanged rows reuse the cached
paint snapshot in constant time; environment changes (font, metrics, scale or
background) invalidate identity first. A weak reference prevents pointer-address
reuse without retaining prior row data. Different snapshots still use existing
per-row reuse and shaping. Desktop compilation and four terminal snapshot tests
passed; native rendering/performance acceptance remains pending, and no measured
latency or RSS improvement is claimed.

Text and hex previews now retain Directory, View as Text/Hex and Refresh buttons
above search results, including zero-match searches. Refresh preserves the current
mode; switching modes uses the existing bounded asynchronous readers. The shared
file-navigation control guards IME composition and standard clicks. Additional
preview state stores only a path and mode, not another captured text copy. Desktop
compilation passed; native visual and interaction acceptance remain pending.

Tab-picker keyboard handling now skips ranking for non-navigation keys and IME
composition before inspecting titles/directories. Exact displayed tab-number
matching parses canonical decimal tokens without allocating a number string per
candidate. Ranking uses an in-place unstable sort with the tab index as a unique
tie-breaker, preserving deterministic order without stable-sort scratch storage.
All five palette-matching tests passed, including numeric bounds, leading-zero,
sign and non-ASCII rejection, literal title fallback and Unicode search behavior.
These remove known work; no end-to-end latency improvement is claimed without a
new measurement. Native tab-picker interaction acceptance remains pending.

Local folder browsing now has a persistent Back/Forward/Parent/Refresh navigation
bar above the search field, so filtering cannot hide navigation actions. History
availability disables Back/Forward, root directories disable Parent, and IME
composition prevents accidental navigation. Controls reuse existing asynchronous
folder loading and remain separate from searchable command rows. Desktop
compilation passed; native button, narrow-window and visual acceptance is pending.

The Settings theme picker now includes a compact configured-color preview with
sample text, cursor color and ANSI swatches for explicit palettes. It identifies
user overrides and explains that applications may change colors. Defaults are
shared with the terminal renderer, avoiding divergent preview colors; the preview
creates no terminal/session and adds no timers. Desktop compilation passed. Native
layout and screenshot acceptance remain pending: the Mac was confirmed locked.

Added Nord as a fifth built-in terminal theme, using the official Nord terminal
ANSI palette, foreground, background and cursor colors. Existing shared theme
choices expose it in Settings and the command palette without new runtime work.
Explicit color/palette overrides retain precedence. The focused preferences test
passed, including serialization, theme cycling and overrides. Native visual/live
reload acceptance and broader theme import remain pending.

Regular-file selections now expose direct hex preview through Shift+Enter and a
compact-compatible footer button. The row hint distinguishes Enter for text from
Shift+Enter for hex, avoiding a failed 64 KiB text read before an 8 KiB byte preview.
Input modes and IME composition are guarded so modified submissions cannot launch
stale file selections. Desktop compilation passed; the existing bounded hex worker
is reused. Native keyboard/button/focus and visual acceptance remain pending.

Added explicit read-only hexadecimal file preview from text previews and preview
errors. A shared bounded regular-file reader serves both modes; hex reads at most
8 KiB plus a truncation sentinel and renders 1,024 eight-byte rows with offsets,
hex bytes and printable ASCII. Rows use monospace text, remain searchable/copyable,
and support Copy Hex Dump, refresh-in-mode and switching back to text. Binary bytes
never execute or decode into terminal controls. Desktop compilation and all ten
file-browser tests passed (manual benchmark ignored), including binary formatting,
offsets, source limits, cancellation and existing text/symlink regressions. Native
preview navigation/visual acceptance remains pending; rich images/documents and
remote filesystem browsing remain separate F01 gaps.

Added bounded per-window local-folder Back/Forward navigation to the file browser,
with visible entries only when available and custom `file_back` / `file_forward`
actions. History commits only after successful reads are accepted by the focused
window, retains up to 32 paths (8 KiB each), preserves Forward on refresh and drops
the forward branch on a new destination. It adds no idle reads or persistence.
Desktop test compilation and a focused history cursor/branch/refresh/bounds test
passed. Native async navigation/focus and visual acceptance remain pending.

SSH host picker rows now offer Edit destination and Shift+Enter, opening the
existing validated destination editor with the selected alias/session/saved target
prefilled and selected. Plain Enter retains direct connection. The transition
clears list state and respects IME composition; it neither changes config/history
nor starts a client until submission. The button remains available in compact
palettes. Desktop compilation passed. A fresh desktop availability check still
reports the Mac locked; keyboard/focus/visual acceptance and screenshots remain
pending, while implementation work can continue.

Added `mise run bench-snapshot-allocations` with a scoped Rust global allocator
counter around snapshot extraction only. At 1/8/32 engines and 200 iterations per
scenario, identical ASCII rewrites recorded zero Rust allocation calls; late ASCII
edits used four calls per frame and oversized-grapheme style changes used five.
The harness retains previous frames to exercise copy-on-write and validates output.
Raw counts/traffic/timings are recorded in snapshot-allocations-debug.jsonl, with
scope in performance.md. Parsing, Ghostty internal allocation, PTYs and UI/GPU are
excluded; these counts do not establish retained-memory or rendering-latency gates.

Snapshot cell construction now reuses an existing heap-backed grapheme string when
its text is identical, including style-only row changes. Inline strings retain
their normal construction path, and changed text gets independent storage. The
Unicode regression verifies shared text pointers across a foreground-color change,
distinct storage after changed combining text, and old-frame preservation; all
four focused snapshot tests passed. Per-frame oversized-glyph scratch allocation
still exists. No measured latency/throughput or whole-process memory claim is made.

Dirty-row extraction now defers its cell-vector allocation until the first cell
that differs from the retained row. Identical rewrites retain the original row
and outer-vector identities without a temporary cell Vec; changed rows clone only
the unchanged prefix before appending new cells. Shorter rows and initial empty
rows retain correct replacement behavior. Four focused snapshot tests plus the
existing selection/style-change identity regression passed, covering Unicode,
oversized graphemes, late edits, resize and older-frame ownership. This reduces a
known allocation path, without a measured throughput or RSS improvement claim.

Terminal snapshot extraction now uses a 64-byte stack scratch buffer for ordinary
glyph text instead of allocating a Vec for every dirty frame. Oversized graphemes
allocate an overflow buffer, reused for the remaining cells in that frame and freed
after extraction; no per-terminal scratch retention is added. A focused engine
test passed mixed ASCII / 81-byte combining graphemes, subsequent shorter output
and preservation of the earlier owned snapshot. Existing hidden-tab shaped-row
release remains unchanged; coherent frame/history data is retained. This removes
one common scratch allocation, with no measured latency/RSS improvement claimed.

Pending interactive workspace startup now supports cancellation through its notice,
File menu, palette and custom `cancel_new_workspace` action. A cancellation owner
also signals when the window drops. The shared launch helper checks before spawn
and before first-frame preparation; completion checks before insertion. The busy
slot remains reserved until completion, preventing canceled retries from accumulating
workers. Canceling status replaces Starting, and only startup-owned notices clear
on completion. Desktop test compilation and the existing native child-reaping
regression passed. OS spawn already in progress is not interruptible; native cancel
button/menu/focus acceptance remains pending.

Interactive New Workspace, sidebar/group plus controls and New Workspace in
Directory now prepare PTY/colors/first frame off-thread. Startup captures directory,
placement and origin; insertion resolves that original workspace instead of the
currently selected row, preserves newer focus, and drops the prepared child if the
origin disappears or changes groups. One command/workspace launch per window bounds
pending work, with a Starting workspace notice. Settings changes are re-prepared
before insertion. Desktop compilation and the native workspace-placement fixture
passed end/top/after-current, group/pinned ordering, failed creation and selection
checks on the shared insertion path. The new interactive async focus/group-change
cases still need native UI acceptance; initial app/group-mutation startup is separate.

All RPC workspace creation now uses the background PTY/first-frame pipeline,
including ordinary local shells. Removed the synchronous workspace dispatcher and
made publication require a prepared seed. Local directory inheritance is captured
from the active pane at request time (then initial-directory fallback); explicit
cwd wins, and SSH's existing launch-directory behavior is retained. The expanded
native fixture passed local workspace actual cwd, unchanged selected workspace,
invalid-directory rollback, and all remote/split teardown and recovery cases.
Interactive New Workspace and initial app launch remain separate startup work;
latency, settings-change and native visual acceptance remain pending.

SSH RPC workspace creation now prepares its first frame/colors in the worker and
passes a Ready terminal through workspace insertion. Interactive Connect via SSH
and custom-command launches use the same ready insertion boundary, with background
re-preparation if colors change during startup and current default fonts on attach.
The native SSH/local-split fixture passed workspace creation, metadata, process
preservation, rollback, teardown and recovery after the RPC change; final desktop
compilation also covers the interactive command change. Ordinary workspace creation
and initial app launch remain separate paths. Native menu/theme/focus and startup
latency acceptance remain pending; no broad nonblocking-startup claim is made.

Added `mise run bench-rpc-splits` and recorded a native debug baseline at 1/8/32
panes, three fresh isolated app instances each. Pooled split request medians were
5.620 ms for runs growing to eight panes and 5.292 ms for runs growing to 32; app
RSS medians were 99.84/102.44/111.22 MiB. The harness checks unique live child PIDs,
uses balanced splits and quits only its own apps. Raw per-request times and RSS
samples are in `docs/benchmarks/rpc-splits-debug.jsonl`. Scope and limitations are
recorded in performance.md: CLI/IPC included, idle fixtures, immediate app-only RSS,
locked desktop, no release/render-latency or before/after improvement claim.

Ordinary RPC splits now share the background PTY/first-frame preparation pipeline
with SSH splits. Removed the synchronous fallback dispatcher and made publication
require a prepared terminal, preventing accidental UI-thread shell startup. Local
splits remain local even beside SSH sources; explicit cwd and current source font
inheritance are preserved. The native fixture now records the local child's real
cwd to a per-PID temporary file, verifies invalid-cwd rollback, and passes alongside
remote/source preservation, teardown and recovery. Initial label/output-based cwd
assertions were replaced because display metadata/wrapping do not prove process cwd.
Compilation and the focused native fixture passed; latency and visual acceptance
remain pending.

SSH RPC splits now carry a PreparedTerminal from their worker through publication:
PTY creation, initial color application and first engine snapshot all happen off
the UI thread. Settings changes during startup trigger background re-preparation
before insertion; publication inherits current source-pane font metrics and reuses
the existing ready-terminal path. Native no-network split creation, source PID and
selection preservation, failed-start rollback, teardown and recovery still pass.
This removes known synchronous first-frame work, not all UI parser access; measured
startup latency and native visual/theme-change acceptance remain pending.

Remote session identity now stays visible when shell or custom titles replace the
initial SSH tab label. Sidebar context shows SSH plus destination before OSC 7,
appends the reported remote directory when available, and switches to SSH ended /
Last path after client exit. The existing full context tooltip handles truncated
rows. Local/restored tabs retain their existing behavior; remote context returns
no local git branch. These use existing metadata/exit notifications without new
polling or filesystem work. Desktop compilation passed; native visual acceptance
and screenshots remain pending while the desktop is locked.

Added explicit `split-pane --ssh DESTINATION` / `pane.split.ssh_destination`, with
CLI help/completion support and validation before request queueing. SSH PTY startup
runs off the UI thread under the command-launch guard; completion rechecks target
ownership/capacity and publishes the prepared session with its SSH identity/label.
Default RPC splits remain local and focus/activation stay opt-in. The native
no-network fixture passed CLI split creation, distinct child PID, preservation of
the source PID and selected workspace, failed-spawn rollback, teardown of both
children, and restore of both targets as local shells. Desktop/CLI compilation and
the focused CLI split test passed. Native visual/focus and remote authentication
through this particular split path remain pending.

Added New Local Tab to the File menu, command palette and custom `new_local_tab`
action so remote panes can host an explicit local shell without another workspace.
It shares normal background tab preparation, cancellation and focus preservation,
but bypasses SSH inheritance; local cwd falls back to the app's initial directory.
The standard plus tooltip now says New SSH Tab when it inherits a session target,
using a borrowed identity check without allocating on render. No default shortcut
was added. Desktop compilation passed; native menu/tooltip/focus acceptance remains
pending while the desktop is locked.

Interactive split/new-tab creation now snapshots the source session's explicit
SSH destination and launches a separate client in the existing background startup
pipeline. Remote siblings carry SSH labels and session identity, retaining remote
metadata isolation and saved-target persistence. Saved disconnected targets are
not inherited, so restored local shells cannot silently reconnect. Existing focus,
cancellation, color/font refresh and failed-start ownership remain in the shared
startup paths. RPC pane.split retains its documented local-shell contract. Desktop
test compilation and all eight SSH tests passed; native interactive split/tab,
authentication and focus acceptance for this new path remain pending.

SSH Hosts now merges current-window session targets and restored saved targets
with discovered config aliases. Session/saved targets sort first; restored targets
are labelled Not connected, and all selections explicitly open a new workspace.
Exact deduplication preserves case-sensitive usernames. The on-demand UI snapshot
inspects at most 256 terminals and merges at most 256 config aliases; no watcher or
separate history store is added. Config-read/home-directory failures still show
available session targets and manual entry. Desktop compilation and a focused
merge/order/case/limit test passed. A fresh native-desktop check reports the Mac
locked, so picker screenshots and native visual/focus acceptance remain pending.

Extended and passed the native macOS loopback SSH fixture with a configured
ProxyCommand using SSH forwarding and a passphrase-protected temporary Ed25519
key. The encrypted-key prompt is observed through surface text, answered through
the real surface.send_text→PTY path, and checked for successful authentication
without passphrase echo. Proxy forwarding is restricted to the fixture port;
IdentityAgent is disabled and askpass bypassed so saved credentials/GUI helpers
cannot satisfy these cases. Existing host-key-change and unauthorized-key failure
cases still pass; sshd logs exactly four successful authentications (direct,
proxy transport, proxy destination, encrypted key). Temporary processes/files are
cleaned up. `mise run smoke-ssh-loopback` reproduces the result. Account-password,
agent, native visual and cross-platform acceptance remain pending; R01 stays partial.

SSH Hosts now discovers aliases through nested Include files, supporting absolute,
user-config-relative and ~/ paths plus lexical filename glob expansion. Discovery
has shared 256 KiB / 64-file / 2,048-directory-entry limits, 64 KiB per file, eight
include levels, 4 KiB include paths and 256 aliases. Canonical file identities stop
cycles; cancellation remains active throughout reads/traversal, and bad included
files retain readable results with an incomplete-discovery label. Tokens, environment
variables and named-user tilde expansion remain unsupported; conditional includes
are inventoried without evaluating Host/Match or executing commands. Focused tests
cover nested/glob paths, cycles, unreadable files and shared resource limits. Native
picker/focus acceptance remains pending. Relative Include resolution follows the
[OpenSSH configuration reference](https://man.openbsd.org/ssh_config#Include).

Added File/palette SSH Hosts… and custom action `ssh_hosts`. Concrete Host aliases
are loaded on demand into a searchable picker with Connect, Refresh and direct
entry. The background read shares the four-file-worker budget and cancellation,
caps source data at 64 KiB and results at 256, and drops a cut final line. Quoted
aliases/comments/Host= forms and case-insensitive deduplication are handled;
patterns, negations and Include expansion are excluded, and Match never executes.
Four SSH tests and desktop compilation passed. Native picker/focus acceptance and
bounded Include discovery remain pending; full connection config stays SSH-owned.

Copy Working Directory now copies the cached remote path for explicit SSH
sessions, while their context menus disable local Reveal. The shared deferred
directory lookup also checks remote identity: inspection found that it bypassed
the earlier current-directory/metadata guard. A focused PTY regression now checks
both synchronous and deferred lookup exclusion for localhost/hostless reports.
That regression and desktop compilation passed. Native clipboard/menu acceptance
remains pending; local directory-copy behavior retains its existing worker path.

Explicit SSH sessions now expose shell-reported OSC 7 paths through a separate
bounded remote-directory text channel. The sidebar context prefixes Remote, and
surface JSON includes `remote_directory`; local PathBuf/cwd/git workflows remain
unset. Remote POSIX paths decode independently of the local OS, reject controls
and malformed UTF-8, and allocate only when directory revisions change. Three
focused parser/session tests and the updated native no-network SSH fixture passed,
including /tmp remote delivery with no local directory. Native visuals remain
pending; shells must emit OSC 7, and remote filesystem browsing is not implemented.

The native SSH workspace path now passed real OpenSSH authentication against an
isolated loopback sshd using temporary Ed25519 host/client keys and private client
configuration. A config alias authenticated successfully; changing known_hosts
rejected the server; removing authorized_keys rejected the client. The server
logged exactly one accepted public key. The app/server were stopped and temporary
files removed. `mise run smoke-ssh-loopback` reproduces this using existing macOS
executables without installing tools or using saved user SSH credentials.
Interactive password/agent/proxy, cross-platform and visual acceptance remain.

Added and passed the native macOS `scripts/smoke-ssh.py` fixture (available as
`mise run smoke-ssh`). A private PATH contains only the fixture SSH executable,
so it cannot fall through to a real client. The real rmuxctl→RPC→background PTY
path passed exact IPv6/user/port argv delivery, launch identity, local-directory
isolation, background-selection preservation, missing-executable rollback,
child teardown on quit and saved-target recovery as a Local shell. Config,
recovery and sockets were isolated in a temporary directory. This validates
native lifecycle plumbing, not visual presentation or real SSH authentication.

SSH workspace creation is now available through `new-workspace --ssh DESTINATION`
and optional `workspace.create.ssh_destination`. Validation happens before queueing;
the UI reserves command startup, prepares the PTY on a background executor, then
rechecks window/capacity/modal state before publication. Success returns the local
workspace snapshot, with focus/activation opt-in; authentication remains terminal
interactive. The existing local workspace path is unchanged. Desktop/CLI checks
and focused request-validation/CLI option tests passed. Native end-to-end startup
and cmux workspace.ssh.open compatibility remain unverified/unimplemented.

Saved layout/session/recovery tabs now preserve an optional validated SSH target.
Restore still starts fresh local shells, visibly prefixed Local with a Saved SSH /
Not connected tooltip instead of implying an SSH connection. Re-saving preserves
the target, and Connect via SSH pre-fills/selects the active tab's live or saved
destination for explicit reconnection in a new workspace. Surface JSON separates
`saved_ssh_destination` from a live-launch `ssh_destination`. All 19 snapshot tests
passed, including target round trips and rejection of invalid/option-like targets.
Native restore/reconnect UI and live remote persistence acceptance remain pending.

SSH launch destinations now live on the session independently of editable tab
titles. Tab tooltips and terminal switcher search retain the destination after
renaming; surface snapshots expose `ssh_destination` (null for local sessions).
Session ownership naturally carries this identity across live tab/workspace moves.
The field describes the requested destination, not authenticated connection state,
and is not yet persisted for reconnect. The PTY metadata regression and desktop
compilation passed; native move/tooltip acceptance remains pending.

SSH destination entry now accepts ssh:// URLs with explicit ports and bracketed
IPv6, validating port range, address syntax and username-only user information.
URLs are normalized to separate -l/-p/host arguments rather than relying on the
SSH client's URL parser (the installed client rejected a bracketed IPv6 URL).
Two focused validation tests passed, and configuration-only checks confirmed
the normalized hostname/IPv6 arguments resolve the intended user and port without
network connections. Live remote acceptance remains pending.

Explicit SSH workspaces now suppress local-directory metadata before their view
is created. Both initial cwd fallback and OSC-derived local paths are excluded,
including remote reports using localhost or an empty hostname. This prevents
local git/file browsing and new-shell cwd inheritance from treating remote paths
as local. Ordinary/custom local command sessions retain their existing behavior.
A real PTY fixture verified hostless/localhost suppression; desktop compilation
passed. Remote directory identity/display remain future R01/R02 work.

R01 now has a first interactive SSH workspace path: File/palette Connect via
SSH… and custom action `connect_ssh` open a validated alias/user@host editor.
Submission launches the existing SSH client with a literal argv through the same
background PTY creation, workspace limits and teardown guards as custom commands.
OpenSSH owns configuration/authentication/host-key behavior; rmux passes no
verification bypass flags and stores no credentials. Desktop compilation and
destination/option-injection regression passed; installed SSH accepted argv in
a configuration-only dry run without connecting. Live authentication, remote
metadata, relay, reconnect and platform acceptance remain outstanding.

The task manager now offers Image storage sorting alongside screen/process
metrics. It ranks Ghostty's sampled image bytes independently of screen resident
storage, keeps missing samples last and preserves tie order. The same sort order
is used for copied reports. A focused regression passed for image-heavy versus
screen-heavy rows, zero/missing values, stable ties and report timestamp/order.
This measures engine image storage, not total GPU texture memory.

Task-manager snapshots now retain their background sampling completion time.
The panel shows a fixed UTC clock time, and copied JSON includes
`sample_completed_at_unix_ms`. Refresh keeps the prior timestamp attached to
the prior rows until replacement; closing clears it. Sorting/copying does not
change the capture time, and no periodic redraw or sampling timer was added.
Desktop compilation passed; native presentation acceptance remains pending.

The task manager's sort control and S key now open a searchable seven-mode picker,
with the current mode marked and selected. Users can jump directly to shell-tree
RSS/CPU instead of cycling every mode. Applying keeps the same terminal selected
and reveals its new row, reusing current samples without another process scan.
Existing cycle action remains available. Desktop compilation passed; native
picker focus/keyboard acceptance remains pending.

Search excerpts can now be copied with Cmd+Shift+C on macOS or Ctrl+Shift+C
elsewhere while the palette is focused. A dedicated CommandPalette key context
keeps this separate from terminal copy and ordinary text-field copy; the footer
uses the resolved shortcut hint. Button and keyboard use one guarded copy path,
which preserves filtering and refuses composition/deep-history status rows.
Desktop compilation passed; native dispatch/clipboard acceptance remains pending.

Directory browsing now keeps readable entries when individual entries disappear
or fail metadata reads. The location summary reports the omitted count and offers
Refresh as recovery. Failed entries consume the existing 2,048-entry scan budget,
so repeated filesystem errors cannot extend enumeration indefinitely. Opening an
inaccessible directory still returns the existing error view. Eight file-loader
tests passed, including mixed success/failure and an infinite-error iterator.

File previews now accept BOM-marked UTF-16 LE/BE and strip UTF-8 BOMs. Decoding
runs in the existing background loader under its 64 KiB source-read/512-line
bounds; UTF-16 may expand to at most 96 KiB of UTF-8 before line clipping.
Byte-capped trailing surrogate halves are omitted, while malformed interior
sequences and NUL-containing text remain rejected. Seven file-loader tests passed,
including both endian forms, CRLF copy preservation, emoji and cutoff handling.
The manual benchmark was not rerun; native preview presentation remains pending.

Selecting a global-search preview now exposes a compact Copy excerpt footer
button. It copies the captured displayed text (including clipping ellipses),
keeps the result palette/filter/selection open and confirms the copy. Deep-history
status rows have no copy button. This uses the bounded snapshot, without rereading
terminal history. Desktop compilation passed; native interaction remains pending.

Global-search preview rows now highlight literal query occurrences using Find's
ASCII case rules and UTF-8-safe byte ranges. Highlight work runs only for rendered
rows over the bounded excerpt. Deep-history status messages carry explicit
non-preview metadata and are not highlighted as terminal output. The focused
literal-match test filter passed nine tests, including repeated Unicode excerpts.
Native visual acceptance/screenshots remain pending: CUA confirmed the Mac locked.

Terminal search excerpts now start near the actual literal phrase instead of
using file-preview word matching. Long lines with an earlier unrelated query word
therefore show the matching phrase; ASCII case folding preserves original Unicode
byte boundaries. Excerpts remain capped at 1,024 characters plus ellipses and
sanitize control characters. All five focused scanner tests passed, including
the new repeated-word/multibyte regression.

Global terminal-output search now has a default window shortcut: Cmd+Shift+F
on macOS and Ctrl+Alt+Shift+F elsewhere, preserving Ctrl+Shift+F for terminal
Find on Linux/Windows. Its menu entry now lives beside Find in Edit. Existing
shortcut resolution supplies palette hints and custom overrides. Desktop
compilation passed; native shortcut dispatch remains pending.

Reopening global terminal-output search now restores and selects the originating
window's last accepted query, allowing Enter to rerun or typing to replace it.
The query is bounded by existing validation and retained only in window memory;
invalid submissions and attempts rejected by the worker limit do not replace it.
Desktop compilation passed. Native keyboard/presentation verification remains
pending.

Added `mise run bench-history-search` and recorded 1/8/32-engine retained-history
probe timings with five samples per match/miss case. The 32-engine debug median
was 5.048 ms for a newest-line match and 26.331 ms for a full fixture miss;
all probes returned the expected result within their per-terminal work budget.
Raw summaries and scope limits are in docs/performance.md. This measures engine
work on idle 2,000-line fixtures, not PTY contention or native UI latency.

Global search now reports preview progress, matching-terminal count and remaining
deep scans in its loading notice. Scanner progress uses a one-slot nonblocking
channel and at most one publication per 50 ms, without an idle polling task.
Cancellation clears the changing notice; completion closes the progress channel
before installing the final result palette. Existing unrelated notices are not
overwritten. Four scanner tests passed; the preview-first regression also checks
phase and completion counts. Native progress/cancel presentation remains pending.

Global search now collects bounded preview matches across the selected terminals
before starting deep-history fallbacks. A busy early terminal therefore cannot
spend the deep-scan budget before later preview hits are collected. Deferred work
retains identities and weak engine probes, not snapshot strings; result order
still follows the original terminal order. Four focused scanner tests passed,
including preview-first scheduling and stable result ordering. Results are still
published as one completed batch; progressive UI delivery remains future work.

Global terminal search now falls back to incremental retained-history probes
when a capped preview has no match or its parser was busy. Probes stop at the
first match without selecting it or exporting full history, using opportunistic
registration/query/feed steps, cancellation and 250 ms per-terminal / five-second
batch work budgets. Incomplete scans are explicit; tracked-reference cleanup
still serializes parser access. A >64 KiB engine fixture found its late Unicode
match with unchanged selection/viewport; three desktop scanner tests passed,
including fallback hits/misses, incomplete counts, cancellation and bounds.
Native interaction and performance acceptance remain pending.

Find match navigation now tries the parser lock and preserves queued movement
on contention. A canceled or superseded query is rechecked after acquisition,
before any selection/viewport mutation. Blocking direct engine callers retain
their existing behavior through a shared implementation. Twelve focused search
tests passed, including held-lock navigation, stale-selection rejection and
successful retry. Search cleanup still requires serialized parser access.

Find query replacement now uses a nonblocking parser-lock attempt. On contention,
the worker restores the query plus refresh/navigation only if that revision is
still current, allowing later edits to supersede it during the 1 ms retry backoff.
The engine API leaves the old query unchanged when busy. Ten search regressions
and a new initialized-worker contention/supersession test passed; queued navigation
lands on the latest query. Match selection and search cleanup locking remain
separate work.

Added opportunistic incremental search feed/step APIs and switched Find's worker
to them. Busy parser feeds keep pending refresh/navigation state, back off 1 ms
and recheck cancellation/supersession; idle searches still sleep on the command
channel. This supplies a nonblocking feed primitive for future full-history
multi-terminal search. Ten engine/search-worker tests passed, including held-lock
resumption with unchanged selection/viewport, superseded queries and queued
navigation. Query-setting, selection and cleanup locking remain separate work;
global output search still has its documented snapshot cap.

Search-result navigation now prepares the Find worker before selecting the
destination terminal or dismissing an existing Find session. If worker-thread
creation fails, the caller receives an error and does not activate the target
window or discard its search. Ordinary Find and result navigation share the
same installation/subscription path. Desktop compilation passed; native failure
and cross-window focus acceptance remain pending.

Extended terminal-output search to all rmux windows with the originating window
first and one shared 256-terminal cap. Results capture a window handle alongside
workspace/surface identity and identify the owning window in their labels.
Selection checks destination readiness and ownership before focusing, opening
Find and activating another window. Closed/moved results return a visible error
instead of selecting a replacement terminal. The scanner's two focused tests
passed with the cross-window payload compiled; native window activation and
result navigation still require acceptance.

Added Search Terminal Output to the menu, command palette and
`search_terminal_output` custom bindings. An explicit submit scans up to 256
terminal history prefixes (64 KiB each) in the current window on one background
worker globally. Results retain one bounded excerpt per matching terminal, show
scanned/unavailable/capped counts, support rerunning, and resolve stable surface
identity before focusing and opening Find with the query and queued navigation.
Cancellation stops between snapshots; focus changes suppress result takeover.
Two focused tests passed for matching rules, literal punctuation, Unicode,
snippet bounds, busy/closed reporting, cancellation and the terminal cap.
Native search/result/focus acceptance, full-history indexing, cross-window and
agent-session search remain pending; F05 is in progress, not complete.

Added Copy Preview Text, preserving captured CRLF/LF separators and the trailing
newline without closing the preview or rereading the file. Incomplete previews
explicitly identify the copy as captured text only. It copies the complete
captured prefix regardless of filtering. The original prefix adds at most 64 KiB
of retained text; the loader benchmark now counts it. The focused preview test
passed with exact CRLF, Unicode byte-cut and 512-line prefix assertions. Native
clipboard interaction acceptance remains pending.

Text-preview search now indexes full retained lines instead of just their
1,024-character display prefixes. Visible rows show bounded excerpts near an
exact query-word match, with Unicode lowercase expansion mapped back to original
text positions. Fuzzy-only matches keep the opening excerpt. Copy actions retain
the original line; no reread or per-keystroke filesystem work is added. The focused
regression covers a match beyond the display cap, expanding Unicode case folds,
snippet bounds, control sanitization and unchanged copy text. Native visuals
remain pending.

Added system file-manager reveal actions to directory listings, text previews
and file error recovery, plus Copy Directory Path. Labels follow the platform;
the existing GPUI reveal API targets Finder/Explorer or Linux's directory portal
with a folder fallback. It does not launch the file's associated application.
macOS paths unsupported by the framework's UTF-8 bridge produce a notice instead
of forwarding an empty path. Desktop compilation passed; native reveal behavior
and other-platform acceptance remain pending.

Added shared readiness checks for tab drags between panes, tab-to-split moves
and same-window workspace transfers. Pending split launches/divider drags or
busy participating panes now leave the tab in place and explain why. In
particular, moving a pane's last tab cannot drop that pane while its new-tab
launch is still pending. Successful same-workspace moves clear the prior move
error. Desktop compilation passed; native drag/launch race acceptance is pending.

Unified join-pane preflight across next-pane and directional actions. Joins now
reject zoom, pending splits, divider drags and busy source/destination panes
before taking tabs. Pending tab launches, close/edit prompts and terminal
overlays therefore cannot be discarded by joining their pane. A visible reason
leaves sessions/layout intact; same-pane and stale-index requests are no-ops.
Desktop compilation passed. Native action/focus acceptance remains pending.

Memory panel keyboard navigation now supports Page Up/Page Down using measured
row heights and the current list viewport. Unmeasured rows use the selected row's
height (or a fallback), and distant destinations reuse the virtual-list reveal
path. Copy and unrelated keys preserve manual wheel position instead of snapping
back to selection. Updated the panel key hint and guide. Desktop compilation
passed. Native UI access was rechecked; the Mac remains locked, so screenshots
and interactive acceptance are still pending.

Measured and reduced process-tree sampling overhead. Parent-child indexes are
built once per refresh snapshot rather than repeatedly for each terminal root;
aggregation no longer allocates a temporary sample vector per tree. Added
`mise run bench-process-trees` and recorded before/after debug summaries. The
32-shell warm median was 44.938 ms before and 29.464 ms after in five samples;
fixture and timing limitations are in docs/performance.md. Eight focused process,
ownership, live-child and port regressions passed. Native UI timing is pending.

Added shell-tree memory/CPU totals to the manual memory panel, sort modes and
JSON export. Shared process-tree ownership checks with the port scanner;
parent/start-time snapshots before/after sampling reject changed ancestors.
The metric batch is capped at 4,096 processes across 256 terminal rows; partial
totals show sampled/expected counts and withhold CPU. Host process-table discovery
is separate from that cap. Shell and foreground values remain separately labeled,
and summed RSS explicitly notes shared-page double counting. Eight focused tests
passed, including a live shell/child, ownership changes, cancellation, exited
process caches, the tree-size cap and CPU aggregation. Desktop compilation passed.
Native panel visuals and tree-sampling performance measurements remain pending.

Added Open Folder through the File menu, command palette, directory/error views
and `open_folder` custom bindings. The native chooser accepts one folder and
feeds the bounded asynchronous directory loader, independent of terminal cwd.
Folder and text-file picking share the same modal gate and cancellation/error
handling. Desktop compilation passed; native chooser/focus acceptance is pending.

Text preview now labels a final line cut by the byte limit as partial, with a
copy hint explaining that only captured text is copied. Complete newline-ended
lines and lines hidden beyond the 512-line cap are not mislabeled. The focused
preview regression passed, including a cut Unicode character and both caps.
Native visual acceptance remains pending.

Recorded a focused file-loader baseline and added `mise run bench-file-loaders`.
On this macOS arm64 debug build, 40 samples measured 2.608 ms p95 for a
2,048-entry directory and 0.095 ms p95 for a bounded preview of a 128 KiB Unicode
file. The preview retained 512 lines / 3,584 text bytes. Raw summary and scope
limitations are in docs/performance.md; this does not measure UI frames, total
memory, cold/network storage or release-build performance. Native UI access was
rechecked and remains locked, so screenshots are still unavailable.

Added Open Text Preview in File, the command palette and `open_text_preview`
custom bindings. A single-file native picker feeds the existing asynchronous,
bounded preview loader without requiring a known terminal cwd. PromptGate
prevents overlapping modal requests, picker cancellation is a no-op, and
completion reuses shutdown/busy checks before loading. Desktop cargo check
passed; native chooser, cancellation and focus acceptance remain pending.

Added New Workspace Here to local directory browsing. The action uses the
existing asynchronous workspace/PTY launch path with an explicit directory,
normal group inheritance and configured placement. Explicit cwd takes precedence
over a group's directory override. Busy/shutdown state and the 64-workspace
window limit are checked before creation. No shell `cd` text is injected.
Desktop cargo check passed; native picker-to-workspace cwd/focus acceptance
remains pending.

Local listings, previews and error recovery now retain a visible path line above
search, so filtering away the Copy Path/Back rows does not hide the current
location. It truncates with a full-path hover hint and remains present in compact
windows. Empty text previews have explicit heading and Refresh-row labels
instead of appearing as an unexplained list of actions. Desktop cargo check
passed; native narrow/short-window appearance remains pending.

Local file browsing now offers Go to Directory from listings and read-error
recovery. The existing picker transitions to a prefilled path editor with
Open/Cancel controls, Enter/Escape handling and IME protection. It validates
absolute UTF-8 paths up to 4,096 bytes without trimming names or expanding shell
syntax, then uses the bounded asynchronous loader. Invalid input remains in the
editor for correction. Focused literal-path/Unicode/relative-input tests and
desktop check/test compilation passed; native navigation acceptance is pending.

Local directory and preview reads now share a four-job process-wide permit
budget, acquired before scheduling. The worker owns its permit until filesystem
work exits, so cancel/retry cannot bypass the cap while an OS read is still
blocked. Saturation reports a retry notice without adding queued work. A focused
test passed saturation, retained permits and release/reacquisition; desktop
cargo check passed. Native slow-filesystem and multi-window measurements remain
pending.

Directory and preview loading now have cancellation ownership tied to the
window. Dropping the owner on cancel, completion or window teardown signals the
background worker. The loading notice replaces ordinary dismissal with Cancel;
cancel also drops the delivery task, preventing late UI replacement. Directory
enumeration checks per entry; preview reads use 8 KiB chunks with cancellation
checks and interrupted-read retry. In-progress OS calls cannot be preempted.
All four focused file-browser tests passed, including owner-drop cancellation
before filesystem access; desktop check/test compilation passed. Native cancel
interaction and slow-filesystem behavior remain unverified.

Local browsing/preview failures now keep a recovery picker with Back to directory,
Copy Path and Retry instead of dropping directly to the terminal with a notice.
The retry row exposes the error and its full hover text; retries use the same
bounded background reads and focus guards. This covers unsupported binary text,
removed files and directory-read errors without requiring a fresh browse from
the terminal cwd. Desktop cargo check passed; native error/retry navigation
acceptance remains pending.

Copying a local preview line now keeps the preview, query, selection and scroll
position open rather than returning to the terminal after each copy. The ordered
clipboard write happens within the picker and shows a footer confirmation,
including compact windows. Editing the query or moving selection clears that
confirmation. Desktop cargo check passed; native repeated-copy and clipboard
acceptance remain pending.

F01 local browsing now opens regular files as bounded, inert UTF-8 previews on
the background executor. The virtualized/searchable palette shows numbered
lines and Back/Refresh/Copy Path actions; selecting a line copies its retained
text. Reads cap at 64 KiB plus one detection byte and 512 lines, with truncation
labels and UTF-8-boundary handling. Display/search caps each line at 1,024 scalar
values. Binary/non-UTF-8 and nonregular files are rejected; Unix opens use
O_NOFOLLOW/O_NONBLOCK and recheck handle type. The existing locked libc version
is now a direct Unix dependency. All three focused file-browser tests passed.
Native preview UI/performance, remote browsing and richer viewer/editor parity
remain open.

Started F01 local file browsing: Browse Terminal Directory in menu/palette and
`browse_files` opens an asynchronous, searchable directory snapshot from the
focused terminal's cached cwd. Parent and Refresh navigate without shell input;
file/link rows copy literal paths. Reads retain at most 2,048 entries, include
hidden names and sort directories first within the returned subset. There is
no idle watcher or file-content read; late results do not steal changed focus.
Focused tests passed bounded listings, Unicode paths, invalid paths and dangling/
cyclic symlink non-traversal; desktop test compilation passed. This is an initial
browsing slice, not completed F01: preview/editor, remote files, persistent panel,
native interaction and performance acceptance remain open.

The tab-picker container now occludes underlying terminal hit targets and
consumes bubbled left/middle/right mouse presses/releases, movement and wheel
events. Child text-field/list handlers still run first; outside-click dismissal
is unchanged. This aligns its event isolation with other terminal overlays and
prevents picker interactions from reaching terminal mouse-reporting handlers.
Desktop cargo check passed; native mouse-reporting, wheel and click acceptance
remain pending.

Tab-picker metadata reconciliation now tracks the selected result position as
well as its terminal identity. When a title/directory change or tab reorder
moves that result, the virtual list reveals its new row. Unchanged positions
preserve manual scrolling, avoiding jumps on ordinary title updates. Query
edits, opening and keyboard navigation reset/update the tracked position.
A focused 256-row ListState test passed moved-selection reveal and preservation
of manual offset for unchanged selection; desktop check/test compilation passed.
Native live-title and wheel interaction acceptance remain pending.

Polished the named-buffer rename editor with explicit Save/Cancel controls and
an editor-specific keyboard hint, replacing the unrelated picker-navigation
footer. The controls remain available in compact windows and wrap in narrow
layouts. Palette submission now waits until IME composition has ended, including
mouse-triggered submission. Desktop cargo check passed; native appearance,
mouse and IME acceptance remain pending.

Added Copy Named Buffer through the native menu, command palette and
`copy_buffer` custom action. It reuses bounded searchable previews and verifies
the captured store revision before copying full contents through rmux's ordered
clipboard writer. The buffer remains stored; no terminal target is required
and no terminal input is sent. Stale selections leave the clipboard unchanged.
Desktop cargo check passed; native picker and clipboard acceptance remain open.

Native macOS buffer rename passed against the rebuilt desktop app using isolated
config and short-lived `/usr/bin/true` terminals. The new `smoke-buffers.py`
scenario verifies advertised capabilities, collision preservation, Unicode
content, removal of the old name, same-name no-op, invalid-name rejection,
unchanged byte accounting and notification routing without extra replies.
The app exits via normal Quit. Native UI access was rechecked and remains
locked; graphical picker/editor appearance and focus acceptance are still open.

Added graphical Rename Named Buffer through the native menu, command palette
and `rename_buffer` custom action. The searchable buffer picker transitions into
a prefilled, selected-name editor within the same palette overlay, preserving
its existing cancellation/focus lifecycle. Enter validates and renames; failures
stay visible for correction, while Escape cancels. Revision-checked submission
prevents stale editors from renaming replaced contents. A focused test passed
invalid-name correction and concurrent-change rejection; desktop check/test
compilation passed. Native visual, keyboard and focus-return acceptance remain
pending.

Added `buffer.rename` RPC and `rmuxctl rename-buffer [--name NAME] NEW_NAME`,
including capability discovery, notification routing, CLI help and completions.
Rename moves the stored String allocation without duplicating buffer contents;
byte/count budgets stay unchanged. Collisions, invalid names and missing sources
fail before mutation; same-name requests are no-ops. Changed names advance the
revision so stale pickers cannot act. Focused store tests verified allocation
identity, accounting and atomic failures; CLI parsing and live-queue routing
tests passed. Native socket acceptance and a graphical rename flow remain open.

Named-buffer paste/delete pickers now show searchable content previews alongside
names and byte counts. Preview construction reads at most 97 Unicode scalar
values per buffer and retains at most 96 plus an ellipsis, avoiding full-buffer
clones/scans for 1 MiB entries. Whitespace and controls render as spaces; empty
buffers have an explicit label. A focused test passed Unicode boundaries,
control flattening, truncation and unchanged large-buffer contents/revision;
desktop test compilation passed. Native picker appearance remains pending.

Custom keybindings now accept `context: "find"`, mapped to the Find input's
actual key context. Users can disable or replace search navigation bindings
independently of terminal bindings. A real-keymap test passed default disabling,
replacement action selection, terminal isolation and rejection of literal shell
text in this scope; desktop test compilation passed. The existing isolated zsh
integration fixture also passed hook chaining, OSC markers, exit status, encoded
directories and opt-out. Native custom Find dispatch remains pending.

Terminal Find now routes Enter, mouse navigation buttons and menu/shortcut
navigation through one method. All three record the query in bounded recent
history and skip empty/error queries; previously buttons and menu actions
bypassed history, and menu actions bypassed the error guard. Existing history
recall/draft and byte-budget tests passed with desktop test compilation. Native
button/shortcut history acceptance remains pending.

Listening Ports now supports Page Up/Down using measured listener heights and
the visible viewport. Shared notice-offset calculation keeps navigation aligned
when ownership/truncation notices precede rows; empty results no longer request
a listener scroll target. Truncated addresses expose their full text on hover.
The shared mixed-height paging test and desktop cargo check passed. Native
keyboard and tooltip acceptance remain pending.

Native macOS first-launch recovery now passes an isolated smoke scenario against
the rebuilt desktop binary: missing nested recovery folders are created with
private Unix permissions, normal Quit writes a valid single-window checkpoint,
and a second launch restores Unicode workspace metadata and checkpoints it again.
The fixture uses temporary config/state and short-lived `/usr/bin/true` terminal
processes. Run `scripts/smoke-session-restore.py --fresh-recovery` for this focused
case; it is also part of the default recovery suite. This verifies lifecycle/RPC
behavior, not OS focus, screenshots or cross-platform acceptance.

First-launch session recovery now creates missing parent directories before
opening its sibling lock file. New directories use mode 0700 on Unix; existing
directories are not chmodded, and obstructing files still fail without mutation.
A regression test first reproduced the missing-directory failure, then passed
initial checkpoint creation and file-obstruction preservation after the fix.
All three focused recovery tests passed, including exclusive lease ownership
and stale background writer rejection. Native first-launch acceptance remains
pending.

Listening Ports refresh now restores selection by socket address and owning PID
after its asynchronous scan, instead of always resetting to the first listener.
Missing listeners fall back to the first result; address or owner changes do not
inherit selection. Existing cancellation/generation guards remain in effect.
Focused selection tests and desktop test compilation passed. Native access was
rechecked and remains locked, so screenshots and refresh interaction acceptance
are still pending.

Settings now has a panel-scoped Cmd+S / Ctrl+S save binding and accepts a
`settings` custom-keybinding context. Save uses the existing guarded asynchronous
persistence action; the button displays its resolved shortcut, respecting custom
replacement/disable bindings. A focused real-keymap test passed scope isolation,
default disabling, replacement action routing and rejection of terminal text
bindings in Settings. Desktop cargo check passed. Native key dispatch and
button appearance remain pending.

Program Status Page Up/Down now uses the visible scroll viewport and measured
card strides, including wrapped messages and inter-card margins, instead of a
fixed eight-task jump. It reuses the shared paging behavior that advances at
least one task when a card exceeds the panel height. Desktop test compilation
and the focused mixed-height paging test passed; native paging acceptance
remains pending.

Program Status now has a lazily created keyboard focus/selection/scroll state.
Opening focuses the panel; arrows and Home/End navigate tasks, C or Copy copies
the selected record, and Escape closes/cancels loading and returns focus when the
panel owned it. Mouse selection and visible row highlighting share the same
task-ID selection. Removed tasks fall back to the first row; insertion/reordering
preserves the selected ID. A focused test passed selection reconciliation and
complete Unicode/multiline copy formatting; desktop check/test compilation passed.
Native keyboard, clipboard and focus-return acceptance remain pending.

Reviewed the stored rmux screenshot and visual-reference notes, then aligned
Program Status close/copy controls with the existing embedded monochrome icons
and shared 28px hit areas. The shared control helper now accepts indexed IDs for
repeated record controls. Listening Ports has explicit minimum-width/overflow
constraints and a shrinking heading with full hover text, preserving space for
refresh/close controls in narrow panes. Desktop cargo check passed. The stored
screenshot is historical evidence, not a fresh render; these changes still need
visual acceptance on the unlocked desktop.

Directional focus now switches the visible pane while zoomed, using current split
geometry and retaining zoom on the destination. This brings arrow-direction
navigation in line with numeric focus and pane cycling; outer-edge navigation is
a no-op. Directional swap/join retain explicit zoom guards so navigation does not
silently enable hidden-layout mutations. Desktop cargo check passed. The neighbor
model already has nested/resize/swap coverage; native zoom/focus acceptance is
still pending on the locked desktop.

Directional pane focus/swap/join now resolve neighbors from the current weighted
split tree rather than cached terminal pixel bounds. This makes selection follow
resize, swap and collapse immediately and keeps terminal headers/overlays from
distorting pane geometry. Normalized regions reuse the existing aligned-edge and
center ranking; zoom guards remain unchanged. A focused model test passed nested
directions, weight changes, swaps, collapse, missing targets, one-pane layouts
and large proportional weights without any render pass. Native navigation and
focus acceptance remain pending.

Workspace and workspace-group context menus now reuse the shared type-to-select
behavior, including their current Pin/Unpin labels. Tab and sidebar menus also
use GPUI occlusion and consume middle-button presses, matching the terminal
context menu's input isolation. Existing selection/activation paths remain in
use. Desktop cargo check passed; native typeahead and pointer-isolation acceptance
remain pending. No background timers or additional persistent menu resources were
introduced.

Added explicit two-pane swap via pane.swap RPC and rmuxctl swap-panes, with
capability discovery, notification support, CLI help/completion and required-ID
validation. It reuses the UI swap operation, preserves split geometry and terminal
ownership, updates pane traversal order, rejects cross-workspace/busy/zoomed
requests and treats same-pane swaps as no-ops. Explicit IDs avoid stale rendered
bounds in hidden workspaces. Native live-shell smoke passed exact identity-only
layout exchange, reverse-swap restoration, selection preservation, notification
framing, rejection paths, retained PID/Unicode shell state and child cleanup.
Desktop build and focused CLI parsing test passed; native OS focus remains
separate from the verified workspace-selection state.

Added shared type-to-select behavior to terminal and tab context menus. Plain
character prefixes select enabled labels; repeated first letters cycle matching
items, longer prefixes narrow selection, and a one-second gap starts a new query.
Navigation keys reset the bounded prefix; modifier shortcuts are excluded. No
idle timer is introduced. Enter/Space still explicitly activates the selected
item through existing enabled-state checks. Focused navigation tests pass for
cycling, prefix extension/fallback, disabled rows, expiry, reset and empty menus;
desktop test compilation passed. Native keyboard/IME acceptance remains pending.

Added Close Finished Tabs to the tab context menu with the current eligible-tab
count, disabled when no other finished tabs exist or a tab launch is pending.
The clicked tab remains the keeper; execution uses the existing identity-based,
policy-aware close path. Keyboard menu navigation skips the disabled entry and
the row shows resolved custom shortcut hints. Desktop cargo check passed. Native
UI access was checked again and the Mac remains locked, so menu/visual acceptance
is still pending.

Added --live-shell to the focused workspace/layout smoke. The native socket run
passed retained PID and Unicode shell-state probes across divider resize,
left/up split insertion and workspace equalization. Newly created split shells
were alive, existing surface identities and selection checks passed, and every
fixture-owned child was observed exiting after app shutdown. The opt-in live
mode runs only the relevant end-placement scenario and completed in under one
second here. This closes the running-process evidence gap for these RPC layout
operations; menu/shortcut focus and screenshots remain separate native UI checks.

Exposed workspace-wide pane equalization through workspace.equalize RPC and
rmuxctl equalize-panes --workspace ID, including capability discovery, notification
requests, CLI help/completion and strict target/option validation. The existing
layout operation now reports change/no-op and avoids rewriting already-normalized
weights. Automation preserves selection and rejects zoomed/busy layouts. Native
socket smoke passed nested weight normalization, idempotence, notification framing,
identity/selection preservation, zoom and invalid-target/field rejection; the CLI
validation test and desktop build passed. The fixture's immediately exiting shells
do not establish running-process or native focus behavior.

Added and ran a release focus-dispatch pressure benchmark at 1/4/8/16 flooding
panes, with application focus reporting enabled and working-task OSC metadata.
128 samples per size measured focus-in plus summary reads, focus-out and optional
status-record copying separately. Focus-in p99 was 0.015–0.175ms; largest observed
focus-in was 0.184ms. Raw JSONL and limitations are recorded in docs/benchmarks.
The benchmark waits for owned workload teardown between sizes. This sample does
not justify changing focus/input ordering; no locking change was made. GUI focus,
render latency, child consumption and worst-case contention remain unmeasured.

Polished Program Status for narrow panes: neutral panel/card colors, shrinking
title columns, bounded overflow, complete-heading/metadata hover hints and a
standard close control with primary-click handling. App/title metadata omits
empty separators. Each record now has an explicit Copy control that exports its
heading, available app/title and full message, preserving access to long text.
Message paragraphs wrap normally; oversized unbroken text stays within the card
and remains available through Copy. Desktop cargo check passed; visual layout,
hover and clipboard interaction acceptance remain pending on the locked Mac.

Program Status opening no longer waits for the terminal parser mutex. An
opportunistic records snapshot populates immediately when available; otherwise
the panel shows Loading and a single cancellable task retries every 8ms only
until the initial snapshot arrives. Closing or normal metadata delivery cancels
the pending load. Records still use inherited app names and sorted IDs. The real
PTY parser-contention fixture now checks the records API returns without waiting;
it passed, as did desktop cargo check. Native loading/cancel interaction remains
pending. This change addresses opening the panel, not every remaining blocking
engine operation such as terminal focus encoding.

Palette search now indexes each displayed, resolved shortcut alongside the action
title/subtitle. macOS modifier/key symbols receive searchable names (cmd/command,
ctrl/control, alt/option, shift and navigation keys); textual cross-platform
shortcuts and chords are searchable by words too. Indexing derives from the
existing origin-context hint, not a separate default-binding table. A focused
ranking test passes for symbols, named modifiers, arrow keys, chords and unbound
actions. Desktop test compilation passed; native typing/reload acceptance remains
pending. Search retains normal fuzzy ranking rather than becoming an exact
keybinding-conflict inspector.

Fixed stale OSC 9;4 activity indicators after terminal completion. The existing
process-exit/output-closure path now clears running, indeterminate and paused
progress while retaining error reports. Late buffered activity cannot resurrect
progress or overwrite a retained error; explicit removal and reset still clear
it. A protocol regression reproduced Running 42% surviving completion before the
fix. Both focused progress tests now pass, covering all transient states, late
reports, error retention, repeated completion and clearing. Close-warning logic
already handled exited processes correctly and needed no change.

Extended Move Tab to Split to the left and above, available alongside right/below
in menus, the palette and break_tab_left/break_tab_up custom shortcut actions.
The transfer uses the same positioned layout insertion as new splits, keeps the
existing terminal entity/PTY, and updates pane traversal order to match geometry.
The source retains at least one tab; focus follows the moved tab and zoom clears.
Desktop cargo check passed. This reuses the previously tested positioned layout
operation; native left/up transfer and live-shell continuity are still pending.

Added left/above pane creation alongside right/below through the shared layout
and shell-launch paths. Menu/palette actions and split_left/split_up custom
shortcuts expose the UI path; pane.split and rmuxctl split-pane accept all four
cardinal directions, including completions/help. Before-target insertion preserves
outer weights, existing entity ownership and pane traversal order. Focused layout
and CLI tests passed, desktop build passed, and native socket smoke verified
nested left/up ordering, equal pair weights, unchanged existing surface identities
and workspace selection. The native fixture uses immediately exiting shells;
live-process continuity and actual menu/shortcut interaction remain unverified.

Fixed keyboard/search jumps to unmeasured virtual-list rows. A regression using
GPUI's actual ListState reproduced a fresh 256-row list staying at row 0 when
asked to reveal row 255. A shared helper now uses a logical row offset for
unmeasured destinations and keeps GPUI's normal minimal-scroll behavior for
measured rows. Palette, tab picker, memory panel and copy-mode cursor/search
navigation use the helper. Two focused tests pass for distant jumps, reset,
backward movement, result-count clamping and empty lists. Desktop test compilation
passes; actual native scrolling/keyboard acceptance remains pending.

Converted the pane tab picker from eager row construction to GPUI ListState/list
virtualization with a bounded viewport and fixed 52px rows. Only viewport/overdraw
rows build their title, directory and tooltip elements; matching still scans the
bounded tab collection. Initial selection and jumps to unmeasured results use
explicit logical offsets. Metadata invalidation preserves scroll position and
clamps it when results shrink; a focused 256-item ListState test passed, and the
desktop compiles. Finished state/exit messages now participate in this picker's
search and hover details as well. Native UI access was checked again but the Mac
remains locked; visual/keyboard acceptance and measured performance remain pending.

Added Close Finished Tabs to the menu, command palette and custom shortcut actions
(`close_finished_tabs`). It selects only exited non-current tabs and reuses the
policy-aware batch-close path, preserving the current tab, live processes and
tabs arriving after confirmation began. Finished-only confirmations describe
discarding output without claiming to stop already-exited processes. Four focused
tab-order/close tests passed, including mixed running/exited tabs and reordering
with a newly arrived tab. Native menu/prompt interaction remains unverified.

Finished terminals now show a neutral Finished label in active and inactive tab
strips, with the complete exit message in the title hover hint. Terminal palette
entries across windows include the finished state and exit message for searching,
without changing custom titles or persisted names. Capturing process exit emits
the existing metadata event so parent tabs repaint immediately. Desktop cargo
check passed; native visual and hover acceptance remain pending.

Separated process exit state from terminal operational errors. Finished terminals
now retain their output with a neutral status strip, complete-message hover hint,
New shell button and Close finished terminal control. New shell routes through
existing new-tab preparation/inheritance without command replay; Close uses the
normal close action/policy. Exit state is captured once rather than overwriting
unrelated errors on subsequent output notifications. Desktop build and native
workspace smoke with immediately exiting fixture shells passed. Actual button
interaction, focus behavior and screenshot acceptance remain pending.

Unified pointer/keyboard divider clamping so a narrow restored pane does not jump
backward on drag start or when pushed against its minimum. The shared helper uses
relative weight units, preserves the smaller starting share, avoids zero neighbors
from rounding and returns unchanged pairs without arithmetic churn. Keyboard
resize now reuses it. Five resize/focus layout tests passed, including zero-delta,
both narrow sides, proportional weight scaling and prior keyboard regressions.
Native drag acceptance remains pending.

Split divider hover hints now describe dragging, pair-only double-click equalizing
and palette resize commands. Drag updates compare the computed pair with current
weights before notifying, avoiding redundant layout/render notifications while the
pointer remains at a clamped size or repeats an unchanged position. Mouse capture
and release semantics are unchanged. Desktop cargo check passed; native hover/drag
acceptance and measured render-count reduction remain pending.

Fixed pane.split JSON-RPC notifications being silently ignored by the mutation
allowlist. The native socket fixture reproduced the missing split before the fix,
then passed notification-created pane count, request-plus-notification resize
weights, unchanged identities/selection, and an invalid resize notification
followed by a successful ping with no stray response. Desktop build and the
updated workspace placement smoke passed. Notification behavior uses the same
validated UI-thread mutation path as ordinary requests.

Exposed directional resizing as pane.resize RPC and rmuxctl resize-pane, including
CLI help/completion and explicit target/direction validation. It reuses the layout
operation, reports changed/no-op, preserves selection and rejects zoomed/dragging/
starting-split workspaces. Native macOS RPC smoke passed actual weight changes,
surface identity/selection continuity, single-pane no-op, invalid direction/target
and zoom rejection, plus resumed resizing after unzoom. CLI parsing test and
desktop build passed. The fixture uses immediately exiting shells; it does not
prove live-process continuity, OS focus, saved-ratio round trips or Windows runtime.

Fixed directional keyboard resizing for pre-existing ratios below its usual 10%
floor. The minimum now respects the smaller existing share, so shrinking an
already-narrow pane is a no-op instead of snapping its divider backwards. Weight
scaling does not introduce an absolute floor, and rounding cannot create a zero
neighbor. Added regressions for narrow restored pairs, tiny equivalent weights,
and a clamped inner split leaving its same-axis ancestor unchanged. All three
resize-focused tests passed. Native shortcut/drag and saved-ratio round-trip
acceptance remain pending.

Added directional keyboard pane resize actions to the menu, palette and custom
shortcut mapping (resize_pane_left/right/up/down). The layout operation chooses
the nearest matching-axis ancestor, preferring the requested-side divider and
falling back to the opposite shared divider at an edge. It moves 5% of the pair,
clamps each neighbor to 10%, preserves pair totals/other weights and pane identity,
and does not fall through to outer splits when the local divider hits its limit.
Zoomed/starting-split workspaces ignore the action. Desktop test compilation and
focused nested-axis/identity/weight/clamp checks passed. Native shortcut, drag
interaction, saved-ratio and cross-platform acceptance remain pending.

Moved invalid-query supersession into SearchWorker: even an oversized edit advances
the revision, clears pending navigation/refresh, schedules an empty query and
returns its validation error. Removed the Find bar's duplicate reset. Search
navigation rechecks revision/cancellation after taking the parser lock, before
installing a selection. A focused regression holds the parser while rejecting a
query, verifies old results disappear, no queued selection is installed, results
clear, and a later valid query navigates successfully. All six search-worker tests
and desktop cargo check passed. Existing user/previously installed terminal
selection is not erased by this result-set reset; native visual acceptance remains
pending.

Find now allows its query and compact navigation/status group to wrap in narrow
panes instead of reserving a fixed-height single row. Truncated status text has a
full tooltip and errors use a distinct tint; navigation/close require standard
clicks. Rejected query edits clear the previous worker query, preventing stale
results/queued navigation from remaining behind an oversized draft. Oversized input
keeps its error instead of accepting a clear-query progress update; Enter/buttons
do not navigate while an error is displayed. Desktop test compilation and existing
query-history tests passed. Native narrow-pane, invalid-query and click acceptance
remain pending; history tests do not cover the new visual/error transitions.

Added per-terminal Find query history with Alt+Up/Down recall, editable drafts and
return-to-draft navigation. Submit/close remembers queries; exact duplicates move
to the front, with eight-entry/16 KiB retained-text limits and 4 KiB per query.
History moves into the open Find bar and returns on dismissal, avoiding a retained
duplicate while open. IME composition and oversized drafts bypass recall. Query
replacement uses the existing text-field undo/change path and search worker.
Desktop test build and two focused history tests passed (Unicode, deduplication,
byte/entry eviction, edit reset and draft restoration). Native Alt-key, IME and
find-bar interaction acceptance remain pending.

Polished the zoomed-pane restore banner with a hidden-pane count, split icon,
explicit Restore layout label and the current Toggle Pane Zoom binding resolved
in the zoomed terminal context. Long status/shortcut text truncates and a tooltip
retains the restore hint. Activation now requires a standard click. Desktop build
and native preset/session restore smoke passed, including restored zoom state;
this does not verify banner clicks, glyphs or narrow-window visual layout.

Settings row labels now select their control and focus the panel without changing
the value, so mouse-to-keyboard editing continues on the clicked setting. Theme
row labels use the same selection path. Long setting button values truncate with
hover hints; labels and control groups may shrink/wrap within a narrow panel.
Desktop compilation passed. Native screenshots are still unavailable (Mac locked);
click/arrow interaction, hover details and narrow-window layout remain pending.

Fixed After current placement from a pinned workspace/group. New unpinned rows
now begin the unpinned tier instead of inheriting a potentially unrelated storage
position from the pinned anchor. Extended the native placement smoke with a group
whose anchor differs from the selected member and whose storage order differs
from sidebar order. The fixture reproduced the pinned-placement failure before
the fix, then passed unpinned-group/pinned-group order and selection continuity,
along with all three placement modes and creation/close failure coverage.
Desktop build passed. Native mouse/keyboard and screenshot acceptance remain pending.

Added persistent ungrouped workspace placement (First / After current / Last) in
Settings and new_workspace_placement config; Last preserves previous behavior.
New Workspace, configured commands and RPC creation use the insertion policy.
Creation returns the actual index, background insertion preserves selection, and
failed RPC creation removes the inserted entry rather than assuming the last row.
Group-member placement stays separate; generated group anchors retain append.
Native RPC smoke passed all modes, selection continuity, failed creation and close;
existing group-directory smoke and saved-settings persistence test passed.

The live run also exposed and fixed a startup panic introduced by dynamic toolbar
hints: GPUI forbids registering two tooltips on one element. Static and action
controls now share an unadorned button builder and each installs exactly one hint.
Native startup now passes; visual/OS-focus acceptance remains pending.

Added Bash completed-command status reports (OSC 133 D) before subsequent prompts.
A leading status capture preserves $? for the first user prompt hook; the existing
trailing integration emits the captured value even if user hooks change status.
Scalar multiline/comment hooks and modern array hook ordering are retained, and
first-prompt completion is suppressed. Existing Bash integration checks now verify
statuses 0/1/7 and status delivery to user hooks; automatic non-login startup test
also passed on macOS Bash. Modern array-hook runtime, command-start tracking and
login-shell autoloading remain pending. Bash ignores --rcfile for login shells;
no startup-file semantics were replaced to work around that limitation.

Exposed macOS Option-key behavior in Settings: Terminal Alt or Native text,
with guidance about accented/dead-key composition and Control+Option/navigation.
The row participates in keyboard navigation and appears only on macOS. Added the
existing preference to saved-settings comparison/persistence so changes apply
immediately and survive Save Settings. Focused settings navigation/toggle and
file-merge persistence tests passed. This reuses existing input routing; native
Option/dead-key interaction and visual acceptance remain pending.

Added a same-pane keyboard/output contention mode to the PTY benchmark and
mise bench-keyboard task. It records press, release and native-text dispatch call
timings into the output-flooding target at 1/4/8/16 panes, with per-size child
cleanup. Local release run passed all sizes; key-press p99 ranged 0.016–0.216 ms,
maximum observed 0.244 ms. Raw results and limits are recorded in the PTY baseline.
The existing raw-input responder benchmark did not exercise this parser lock.
No mutex rewrite is justified by this sample; UI latency, Kitty-mode transitions
and sustained worst-case acceptance remain unproven.

Bound shared control tooltips to the window's available width/height, with a
380px width ceiling, 120px height ceiling, five-line wrapping/ellipsis and clipped
overflow. Explicit line spacing and a subtle border distinguish hints from dark
panels. This covers tab titles, palette details, shortcuts and sidebar hints.
Sidebar grouping guidance now resolves the current binding on hover and points
to the palette when no binding applies, rather than advertising a fixed shortcut.
Desktop cargo check passed; native clipping/wrapping and visual acceptance remain
pending while the Mac is locked.

Added Run Configured Command… to the File menu, general palette and
custom_command_picker shortcut action. The dedicated picker reuses fuzzy search,
virtualized rows, keyboard navigation, current binding hints and focus return.
Shared command rows now include cwd and quoted empty/space-containing argument
previews, bounded to 768 characters; environment values are omitted. Empty config
and no-match states provide distinct guidance. Execution resolves the current
command definition and uses the existing background launch path. Desktop check
passed. Native screenshots remain unavailable because the Mac is locked; picker
appearance, focus and interaction acceptance remain pending.

Added rmux --command NAME for configured custom commands. It uses the existing
PTY command launcher with literal argv, child-only environment overrides, saved
cwd or launcher-directory fallback, and the configured terminal label. Explicit
command launches bypass automatic recovery ownership; unknown names and mixed
launch modes fail before a terminal starts. Focused parser tests and desktop
build passed. A native real-PTY smoke test passed Unicode/empty/literal arguments,
configured and inherited cwd, environment overrides and missing-name failure.
Native window appearance and cross-platform runtime acceptance remain pending.

Replaced fixed shortcut labels on the terminal tab picker, new/cancel tab,
split-right/down and sidebar new-workspace controls with the shared binding
resolver. Tooltips resolve on hover against the pane's active terminal (sidebar
uses current window focus), so overrides, disabled bindings and chords reflect
the current keymap after settings reload. No binding lookup is added to the
terminal paint loop. Desktop cargo check passed; native tooltip appearance and
focus-context acceptance remain pending.

Polished shortcut labels across palette/context-menu hints and pending chords.
macOS uses compact modifier/key symbols; other platforms use Ctrl/Alt/Shift with
named special keys. Binding hints respect GPUI's display key/modifiers (including
Windows keyboard mapping) without changing parsing, precedence or dispatch.
Palette shortcut columns are capped at 40% width with ellipsis/full-label tooltips
so long bindings cannot crowd out command names. Desktop test compilation and
focused platform notation/key-identity checks passed. Native glyph, narrow-window
layout and keyboard-layout acceptance remain pending.

Added a dedicated Open Layout Preset… picker in the File menu, general palette
and layout_preset_picker custom action. It reuses fuzzy matching, virtualized rows,
keyboard navigation, focus return and current-context shortcut hints. Shared preset
rows now include layout/session type, optional project cwd and source path, making
project directories searchable without reading files. Empty configuration and
no-match states have distinct guidance. Invocation still resolves current config
and uses the existing import path. Desktop compilation passed; native picker,
focus return, narrow-window layout and screenshot acceptance remain pending.

Added rmux --preset NAME startup, sharing configured layout/session decoding,
default-cwd handling and fresh-shell preparation with GUI presets. Exact Unicode
names and live stdio RPC are supported; conflicting launch/inspection options and
benchmark mode are rejected. Explicit preset launch bypasses normal recovery-file
ownership. CLI parsing test and desktop build passed. Extended the isolated native
session harness with --preset; its run passed two-window/four-shell restoration,
metadata/weighted splits/tab selection, actual fixture cwd (including a null saved
path filled by preset cwd), unchanged source JSON and child cleanup. Corrected an
outdated harness group expectation to include the existing cwd field. Native
palette invocation, OS activation and visual focus remain unverified.

Added optional absolute cwd defaults to named layout/session presets, allowing
one reusable template to serve several projects. Background import fills only
unset tab directories across every nested pane/window, preserving authored
per-tab paths and leaving the source JSON unchanged. Existing preflight validates
the resulting directories before any shell starts. Updated example and user-guide
workflows. Desktop test compilation and focused preset validation/nested-directory
precedence tests passed. Native shell-cwd/preset invocation acceptance remains.

Added reusable development and services layout files plus a practical setup
guide. Development has a wide editor pane and stacked run/output panes; services
has equal API/worker/log columns. Files contain no executable commands, geometry,
font overrides or machine-specific paths. The guide explains null versus explicit
absolute cwd, fresh-shell semantics, preset registration, Windows JSON paths and
saving customized arrangements. Both files passed the current rmux --check-layout
validator after a desktop build. This verifies schema/layout structure only;
native appearance, repeated-use behavior and cross-platform runtime remain pending.

Added named layout/session presets to configuration, command palette and custom
shortcuts. Up to 32 validated name/absolute-path entries appear as Open Preset:
name; an optional session flag selects the multi-window format. Invocation resolves
the current config, skips only the file picker, then uses the existing background
import/progress/cancel/rollback path. Files are read only when invoked; settings
loading and palette display do not touch them. Documented a repeatable save,
configure, reload and apply workflow with explicit fresh-shell/cwd semantics.
Desktop compilation and focused name/path/count/shortcut validation tests passed.
Native preset invocation, reusable example bundles and full upstream template
semantics remain pending.

Added preparation counts to interactive layout/session imports. The worker emits
bounded progress events (one pending update, no timer); UI updates require the
same uncanceled job token, preventing late status overwrites. Added Cancel Layout
/ Session Restore to the palette and cancel_restore custom shortcuts, sharing
the existing status-button cancellation path and generic cancellation wording.
Desktop compilation and a focused layout/multi-window progress-total plus
pre-spawn cancellation test passed. Native status/keyboard/cancel acceptance and
interrupting an in-progress OS spawn are not claimed.

Added Open Session in New Windows… to the File menu, command palette and
open_session custom shortcut. The shared layout/session picker prepares shells
on the background executor using existing schema/directory validation and
cancellation. Publication preserves saved order/geometry, activates the saved
window and rolls back only newly created windows if native creation fails.
Existing windows remain intact; empty imports fail explicitly. Desktop compilation
and all three restore preflight/cancellation/partial-spawn cleanup tests passed.
These tests cover preparation, not the new native picker/publication/rollback UI.
Native import, selected-window focus and cross-platform acceptance remain pending.

Removed eager payload copying from borrowed paste calls. Ordinary borrowed,
owned clipboard/image-path and automation paste now share the same admission and
queue path via Cow: parser-busy rejection drops its permit before any borrowed
payload allocation, while owned strings move unchanged. The parser guard ends
before copying; the existing bounded writer, one-paste permit, cancellation,
bracketing and viewport-follow differences remain. All nine paste-focused tests
passed, including real-PTY large input under concurrent output, ordering,
automation viewport preservation and cancellation. No native clipboard or measured
RSS/latency claim is inferred from this allocation-path change.

Added a real-PTY concurrent output/paste regression: a raw receiver records a
896 KiB Unicode multiline paste and following input while a bounded background
producer emits more than 250 KB of terminal output. The focused macOS run passed
in 2.33 seconds, verifying exact bracketed-paste bytes, following-input ordering,
released paste permit and no writer error. The fixture retries only explicit
pre-enqueue parser-busy rejection, never an accepted request. Producer cleanup is
owned by the fixture shell. This is delivery evidence, not native clipboard, UI
latency, saturation throughput or other-platform acceptance; T07 stays in progress.

Replaced generic link tooltips with a dedicated window-bounded preview: a compact
modifier/action label and an ellipsized target line prevent long unbroken URLs
from overflowing narrow windows. Added Copy Link Address and Open Link terminal
context-menu rows. They capture the address under the menu's opening coordinates,
keep it stable if output changes, disable missing targets and apply the same
HTTP(S) opening rules as modifier-click; copy retains the original target. Link
lookup is nonblocking and a busy parser leaves the rows disabled. Existing menu
keyboard navigation includes the new rows. Desktop compilation passed; native
hover/menu/clipboard/narrow-window acceptance remains pending.

Added modifier-hover link feedback: supported HTTP(S) targets show a pointer
cursor and target tooltip; unsupported OSC 8 targets show an explanatory tooltip.
Hover and click share the same URL allow rules. Lookups occur only while holding
the platform link modifier, cache the displayed revision/cell, and use the existing
nonblocking terminal lookup. Parser contention clears feedback without an error
banner and permits retry on motion. Pointer exit/modifier release, click, window
transfer and changed terminal frames clear stale targets. Desktop compilation
passed; native cursor/tooltip/modifier and output-pressure acceptance remain.
Local/remote file path routing is still unfinished.

Filled custom-shortcut action coverage for clipboard and lifecycle commands
(copy/paste/select-all/clear-scrollback/close-tab/close-window/quit), layout/session
save/open, settings-file/appearance save, and memory/git/program-status/notification
diagnostics. These map to existing typed menu actions, retaining their dialogs,
close policies and focus handling; no default keys or parallel action handlers
were added. Documented names, terminal-vs-window scope and examples. Existing
real-keymap override/context/disable test passed with desktop test compilation.
Native dispatch/confirmation acceptance and full Ghostty action parity remain.

Added terminal-scoped shortcut text targets alongside existing actions and
configured commands. Single/chord bindings send exact UTF-8 input through the
normal bounded terminal writer; they add neither Return nor bracketed-paste
markers. Validation enforces one target, terminal-only scope and 1–16,384 bytes.
The handler requires terminal focus and rejects active copy/find/ports/prompt/quit
states. Focused real-keymap tests passed for Unicode/control-byte preservation,
settings-context exclusion, target ambiguity and byte bounds. Native dispatch to
a PTY remains unverified. This extends rmux's JSON keybindings; Ghostty syntax,
named key tables and complete action compatibility remain pending.

Extended manual terminal memory diagnostics with Unix foreground process-group
leader PID, RSS and CPU, plus foreground RSS/CPU sorting and JSON export. The
sampler batches up to 256 shells, 256 foreground leaders and the rmux process;
no polling or command-line/environment reads were added. Foreground identity is
rechecked on completion, discarding samples after job changes or terminal moves
out of the window. CPU retains existing process-generation/baseline checks.
Desktop compilation and the real-PTY idle/running/interrupted foreground identity
test passed. These are leader-only snapshots, not pipeline/descendant aggregates;
Windows foreground discovery, native panel acceptance and performance measurements
remain pending.

Removed blocking terminal-parser and exit-state lock acquisition from the
on-demand active-work close check. Contention conservatively requires the normal
confirmation; it never silently treats unknown state as idle. Idle/exited sessions,
Unix foreground job detection and direct-command lifetime handling remain intact.
The focused real-PTY lock-contention test passed with the parser deliberately held,
and the foreground-job/direct-command close-detection test passed. Corrected the
stale W11 ledger item that still listed foreground job detection as missing.
Native UI inspection was attempted again, but the Mac remains locked; no visual
or native close-dialog acceptance is claimed.

Added dedicated keyboard focus and visible selection to the terminal memory
panel: arrows/Home/End select, Enter focuses the addressed terminal, R refreshes,
S sorts, C copies the report and Escape closes. Sort and completed refresh resolve
the selection by stable surface identity; clicks update the same selection.
Selected rows scroll into view. Closing restores previous focus only when the
panel owns focus, and existing cancellation/generation cleanup remains shared by
button and keyboard paths. Desktop compilation passed; native keyboard, focus
restoration and narrow-window layout acceptance remain pending.

Changed the terminal memory panel from eagerly constructing every sampled row to
GPUI's virtualized list, matching the command palette's rendering approach.
Only requested rows construct labels, formatting, tooltips and click handlers;
sorting retains a lightweight index vector and does not clone sample/name data.
Refresh, sort and close reset list measurements; terminal focus still uses stable
workspace/surface IDs. Desktop compilation passed. Native scrolling, sizing and
render-cost measurements remain unverified; no latency or RSS improvement is
claimed from compilation alone. Also corrected the stale F08 ledger entry to
reflect existing manual shell/app process sampling and its remaining scope.

Extended the live RPC harness to exercise workspace merging on the actual macOS
app with seven running fixture shells. Same-window and cross-window merges passed:
both nested layout subtrees, all terminal IDs/PIDs and Unicode shell variables
survive; destination selection/ownership changes correctly; stale source IDs fail
without mutation; merging the final source workspace closes its window. The test
also confirms every tracked child exits after terminating its own isolated app.
Ran `mise exec -- cargo build -p rmux-desktop` and
`mise exec -- python scripts/smoke-live-rpc.py --live-shell` successfully.
This establishes live stdio/PTY continuity, not visual polish, native OS focus,
unread/group/tab-order behavior, socket transport, persistence or other platforms.

Extended workspace merge picker and RPC routing across app windows. Picker rows
identify the window and workspace; stable destination handles/IDs are rechecked.
Both window interaction guards, both split states/depths and the destination's
256-terminal budget are validated before moving ownership. Moved terminal entities
retain sessions and image-file ownership while rebinding native focus/input
subscriptions and resetting old-window pointer/editor state. Destination selection
is shared with same-window merging; OS activation stays opt-in for RPC. Source
entry/group/last-window cleanup runs only after destination ownership succeeds.
Desktop test compilation and focused RPC validation/routing passed. These are not
live cross-window tests: PID/state/unread/order, source-close focus, groups and
restore/native acceptance remain pending.

Added rmux workspace.merge RPC and merge-workspace CLI/help/completion with
explicit source/destination IDs and optional OS activation. The UI handler
resolves the source window, requires the destination in that same window and
calls the existing guarded merge implementation. Distinct-ID/unknown-field/type
validation occurs before queueing; notification mutation routing and capability
listing include the method. Focused RPC invalid-request/no-queue and valid-route
checks passed, as did the CLI argument check. These tests do not establish live
PTY continuity or native focus behavior. The pinned CLI/RPC inventory contains
no merge command, so this is explicitly an rmux extension. Live socket, groups,
focus, persistence, cross-window and full upstream behavioral acceptance remain.

Started W15 with a same-window Merge Workspace into… picker, menu/palette action
and merge_workspace custom shortcut. Source/destination IDs are captured and
revalidated after picker dismissal. Existing interaction guards plus split-launch,
drag and combined-depth checks run before ownership mutation. The synchronous
transfer retains both pane trees and terminal entities, appends source panes after
destination panes, updates drag ownership/subscriptions, clears zoom and removes
the empty source through existing group-aware entry cleanup. No new PTY is started
by this path. Desktop test compilation and a focused depth-boundary test passed.
Live PID/state/order/unread, focus/group/persistence acceptance, cross-window merge,
CLI/RPC and exact upstream behavior remain pending; W15 is not accepted as complete.

Added a macOS Finder copied-file clipboard adapter using per-item AppKit file-URL
metadata, preceding GPUI's text/image fallback. It reads only on Paste, never
changes the system clipboard, bounds item/path data, rejects non-local URLs and
checks pasteboard changeCount before queueing shared POSIX-quoted paths. The
native NSURL fixture passed escaped spaces/apostrophes/hash/Unicode and rejection
of remote/non-file/query/fragment URLs without reading file contents or touching
the clipboard. Desktop test compilation passed. See Apple's
[pasteboard item API](https://developer.apple.com/documentation/appkit/nspasteboard/pasteboarditems)
and [item string API](https://developer.apple.com/documentation/appkit/nspasteboarditem/string%28fortype%3A%29).
Native Finder paste/focus acceptance, Linux copied-file MIME handling and Windows
quoting remain pending; AppKit provider reads may still block on the provider.

Started T04 with live default-terminal-background opacity and optional compositor
blur. Settings provides 0–100% opacity in 5% steps plus blur, with config validation,
change tracking and Save Settings persistence. The terminal base fill uses alpha;
text and explicit cell backgrounds remain opaque. Removed the redundant workspace
background and made the window root fill conditional so parent layers cannot mask
transparency. Window appearance changes are cached by mode (opaque/transparent/
blurred), and preference updates notify all window roots. Desktop compilation and
a focused defaults/bounds/stepping/mode/save-reload test passed. Native main-app
visual parity, compositor availability/cost, cross-platform acceptance, background
images and shaders remain pending; no screenshot or performance claim is made.

Added crash-orphan cleanup for newly pasted images in a private per-user cache
directory. A stable directory lock serializes file publication and bounded cleanup;
content writes run after releasing it, while a per-image lock remains held for
the owning terminal lifetime. Each background paste examines at most 512 entries,
removing only unlocked regular rmux-image files and skipping symlinks/unrelated
names. Contended publication returns a retryable busy error instead of blocking.
Focused storage tests passed permissions/bytes, live-file retention, orphan
removal, publication exclusion, unrelated/symlink preservation and quota release.
Native forced-termination/cross-process acceptance remains pending. No idle timer,
external temp-directory sweep or deletion of legacy image files was introduced.

Image preparation now participates in the existing paste status/cancel UI. The
badge distinguishes Preparing image paste from queued delivery, and both its
button and Cancel Queued Paste cancel the pending image task plus queued PTY
paste. Terminal key/text input cancels preparation before sending new input;
automation mutations use the existing busy guard while preparation is pending.
Completion notifies even when focus/overlay changes suppress delivery, clearing
the badge. Cancellation drops delivery immediately; an already-running bounded
file write may finish before its unused temporary file/reservation are released.
Desktop compilation passed. Native status/cancel/focus-race acceptance remains
pending; no claim is made that an in-progress PTY write can be interrupted.

Extended T11 to image-only clipboard paste on macOS/Linux. The pinned GPUI API
has no copied-file clipboard representation, so Finder copied-file support was
not claimed. Supported clipboard images are moved into a background writer,
retaining original format/bytes without decoding or another image clone. Private
temporary files are retained by the target terminal, with 16 MiB per-image and
app-wide 32-file/64 MiB reservations, and removed on terminal drop. Focus/overlay
state is rechecked before queueing the quoted path; newer text paste/file drop
cancels pending image delivery. Desktop test compilation and storage tests passed
for contents, mode 0600, unlink-on-drop and released/rejected quotas. Native image
clipboard/focus acceptance, crash-orphan cleanup, copied-file clipboard support,
Windows quoting and remote upload remain pending. GPUI's initial clipboard read
can allocate before rmux applies its image limit; no RSS bound is claimed.

Started T11 with local external file-drop insertion on macOS/Linux. Focused
terminals accept absolute paths as POSIX-quoted text through the existing bounded
paste queue, preserving bracketed paste and adding no Enter. Overlays, unfocused
terminals and active quit/prompt guards reject delivery. Files are never opened;
256-path/64 KiB caps and UTF-8/control checks apply before queueing. Desktop test
compilation and a focused real-/bin/sh round-trip passed for spaces, apostrophes,
Unicode and literal shell syntax, plus invalid/oversized input rejection. Native
Finder/file-manager acceptance, Windows quoting, image paste and remote upload
remain pending; T11 is partial, not verified. No native UI claim is made while
the Mac remains locked.

Added an explicit Open as HTTP external-link control and O shortcut to the local
Listening Ports panel. Wildcard IPv4/IPv6 addresses map to their loopback address;
explicit addresses and ports are preserved. Scoped IPv6 and port-zero entries
remain copy-only; HTTP is named explicitly rather than guessing a service's
protocol. Uses the default-browser API only on user action, with no automatic
network request or port polling. Desktop test compilation and focused URL-mapping
coverage passed (wildcards, explicit IPv4/IPv6, unchanged 443 scheme, unsupported
scope and zero port). Native browser activation, icon/layout and narrow-panel
keyboard acceptance remain pending while the Mac is locked. T09 and the guide
now describe the new control without claiming automatic sidebar port metadata.

Added a dedicated Move Workspace to Window… picker to the native menu and command
palette, plus custom shortcut action move_workspace_window_picker. It uses the
existing fuzzy/virtualized palette with active workspace names, workspace counts,
busy-state hints and distinct no-other-windows/no-match messages. Source identity
and destination window handles are captured, then the existing guarded transfer
path revalidates them on selection. General palette destination commands already
existed; corrected the stale W06 ledger rather than claiming a new transfer
backend. Desktop compilation passed. Native access was rechecked: the Mac remains
locked and automatic unlock failed, so screenshots, focus, stale-window and
visual acceptance remain pending. No synthetic preview was substituted.

Measured tail capture with an explicit ignored release fixture and removed a
duplicate Ghostty grid lookup in each backward-walk iteration. The same three-case
fixture observed median 20-line capture from 1.1 MB history at 11 → 7 µs, a 1 MiB
wrapped line at 2.341 → 1.649 ms, and blank screen at 16 → 15 µs. Each case has
11 samples; these sequential runs are observations, not fixed speedup guarantees.
Output remains capped at 64 KiB in the fixture. Seven release text-export tests
passed, including edited-wrap differential cases and UTF-8 truncation. See
[measurement details](benchmarks/history-tail.json). Counting still scans the
candidate range; giant-line parser contention, native input-to-pixel latency,
whole-process memory and concurrent multi-pane acceptance remain unmeasured here.

Corrected tail capture to operate on formatted text lines, following the pinned
[cmux tail tests](https://github.com/manaflow-ai/cmux/blob/a76816e4f0788003758b9434ab702068a640fb5d/cmuxTests/TerminalControllerTerminalTextTests.swift).
Unused trailing rows no longer consume the requested line count. Explicit spaces
that preserve a newline remain significant. Row metadata bounds the candidate
range; a counting formatter pass handles extra separators after erased/wrapped
rows before the bounded output pass. Neither pass copies preceding history;
the counting pass scans the candidate text, so long-line latency remains to be
measured. Seven focused export tests passed, including differential comparisons
against full-text suffixes after 200 deterministic Unicode/wrap/erase/tab edits,
with UTF-8 byte caps. Native/live socket acceptance and upstream snapshot merging
remain unverified; this supersedes the trailing-blank limitation recorded below.

Connected the logical tail exporter through Session, background surface.read_text
RPC and capture-pane --lines N. CLI help/completion include capture-pane, explicit
surface targeting, --scrollback and --lines; the CLI cap is 1 MiB. Counts are
validated as 1–65536 and require history scope in RPC. The session closure keeps
only a weak terminal reference and uses try_lock, preserving the existing busy
error instead of waiting on the parser. Desktop/CLI compilation and focused CLI
routing/count checks passed. The real-PTY busy-parser test also passed with a
tail-export assertion, confirming no wait for the held parser lock. Trailing blank screen rows currently count toward
the requested range; upstream trailing-blank equivalence, implicit target aliases
and live socket/native acceptance remain pending. No full capture parity claimed.

Added a terminal-engine tail-range exporter for capture-pane work. It walks
backwards through Ghostty row metadata to locate logical line boundaries before
formatting, without copying preceding history or replacing the user's selection.
The existing bounded UTF-8 formatter limits output allocation to 16 MiB maximum;
truncation returns a prefix of the requested range and is explicitly reported.
Five focused text-export tests passed, covering wrapped Unicode, byte caps,
selection/viewport preservation, oversized line counts and alternate-screen
isolation. Blank trailing screen rows count toward this low-level range (the
formatter trims their text). Session/RPC/CLI wiring and the capture command's
trailing-blank semantics remain pending; no end-to-end capture parity or latency
measurement is claimed.

Confirmed [pinned pipe-pane behavior](https://github.com/manaflow-ai/cmux/blob/a76816e4f0788003758b9434ab702068a640fb5d/CLI/cmux.swift):
one captured history snapshot feeds a client-side shell command. Implemented
pipe-pane --surface ID with --command or quoted positional command using the
existing history RPC, capped at 1 MiB. Truncated/invalid responses reject before
spawning; command text is stripped from RPC params. The CLI forwards stdout/stderr
without collecting them, waits/reaps the child and returns its status; early
stdin closure is supported. Command runtime follows normal shell semantics, with
no automatic timeout. No PTY reader or UI thread waits on the consumer. Two
focused request-routing/process tests passed, including literal stdin, nonzero
status, early close and size limits. Window video recording, selector aliases,
JSON command-output mode and Windows execution remain pending.

Reviewed the [pinned CLI contract](https://github.com/manaflow-ai/cmux/blob/a76816e4f0788003758b9434ab702068a640fb5d/docs/cli-contract.md)
before recording work: window video capture and terminal text piping are distinct;
copy-mode is a placeholder there. Corrected T15 scope without claiming implementation.
Implemented the concrete set-buffer stdin gap: omitted text or dash reads UTF-8
exactly, with a 64 KiB read ceiling and the existing serialized-request cap.
Literal arguments never consume stdin; malformed/oversized input fails locally.
Three focused CLI tests passed, covering newlines, empty input, invalid UTF-8,
bounded reads and argument handling. Capture backends and native acceptance remain
pending; no PTY logging substitute was introduced for video recording.

Started T17 with local PTY input-mode sampling and an Input hidden badge for a
focused terminal in canonical mode with echo disabled. Raw-mode TUIs are excluded;
macOS/Linux query termios through portable-pty without taking the terminal parser
lock. Output, focus and input events refresh the sample; there is no idle timer
and silent termios changes may remain stale until another event. Remote tty and
ConPTY modes are not inferred, and no typed content is inspected. A real PTY
fixture passed hidden-canonical → raw-no-echo → echoed-canonical transitions;
desktop compilation passed. Native badge layout, broader prompt coverage,
Windows/remote detection and optional prompt-editing gestures remain pending.

Named-buffer lifecycle now includes Delete Named Buffer… in the command palette,
native menu and delete_buffer custom shortcut action. The searchable picker
lists names/sizes without cloning stored text and works without a focused terminal.
Deletion checks the captured store revision, so a replaced buffer cannot be
removed through stale picker state, and releases the stored text and byte quota.
Five focused buffer tests passed, including stale-delete rejection and storage
release, with desktop test compilation. Native picker/input acceptance remains
pending; full T14 parity is not claimed.

Added Paste Named Buffer… to the palette/menu and paste_buffer custom binding.
The searchable picker lists bounded name/size metadata, captures its originating
surface, and checks store revision before terminal-aware paste. Changed buffers,
closed/moved targets and existing input guards fail explicitly without fallback.
While wiring it, found and fixed a mismatch: the named-buffer API permitted 1 MiB
but try_paste_without_scroll still capped 16 KiB. That function now accepts 1 MiB
and queues paste encoding on the writer thread; surface.send_text JSON validation
retains its independent 16 KiB limit. A real PTY test passed with Unicode input
above 64 KiB, bracketed markers, subsequent input ordering, unchanged viewport,
busy-parser handling and >1 MiB rejection. Four buffer tests and the desktop test
build passed. Picker visual/input acceptance remains pending.

Copy mode now saves selections to named buffers through Save Buffer… or b. A
name field supports Enter/save and Escape/cancel; existing names are explicitly
replaced. Line/character/rectangle formatting runs on the background executor,
checks the 1 MiB per-buffer cap, and transfers owned text into storage without a
second UI-thread text clone. Clipboard state is untouched. Store revisions reject
stale completions if any buffer mutation happened during formatting; closing or
superseding the save cancels delivery. Four buffer tests passed, including stale
save rejection and name validation, with desktop test compilation. Native input,
layout and broader upstream copy-mode contract acceptance remain pending.

Named buffers now support paste-buffer --surface ID [--name NAME] [--bracketed]
and buffer.paste RPC. Explicit opaque targets resolve through the same input
routing as surface.send_text, retaining existing modal/busy/exited/backpressure
checks and preserving focus/viewport. Raw delivery is capped at 64 KiB;
--bracketed uses terminal-aware paste encoding up to the 1 MiB buffer limit.
Buffers are retained, mutations never auto-retry, and invalid targets/options or
missing buffers return errors. Three buffer tests and one CLI test passed with
both test builds; native delivery and UI save-to-buffer acceptance remain pending.

Added process-local named copy buffers, beginning with the inventoried set-buffer
and list-buffers CLI shapes. The live RPC queue supports buffer.set/list plus
rmux-specific buffer.get/delete; capabilities advertise them and shell completion
includes both CLI commands. Default name is default; no persistence or OS clipboard
mutation occurs. Storage caps 32 names, 1 MiB each and 8 MiB total text; failed
replacements preserve existing entries and deletes release accounted bytes.
The 64 KiB transport request cap remains in force. Two storage/protocol tests,
a CLI parsing test and a live-queue routing test passed with desktop/CLI test
compilation. Full upstream semantics, paste-buffer targeting, UI selection saves
and native acceptance remain pending.

Copy mode now has rectangular selection via r, alongside character and line
modes. Anchored keyboard ranges use terminal-cell columns, highlight each row,
and include complete graphemes touched by either edge. Copy joins retained row
slices with newlines without synthetic right padding; short/empty rows are
supported. Formatting uses the existing background copy path and checks the
clipboard generation between rows. Seven focused behavior tests passed (the
explicit performance sample stayed ignored), including reversed rectangles,
CJK/combined emoji at edges, empty rows and cancellation. Desktop test compilation
passed. Native UI acceptance and named buffer/CLI/RPC parity remain pending.

Copy-mode clipboard preparation now runs on the background executor against the
shared snapshot. The requested selection is captured at activation; subsequent
navigation does not change what is copied. Each request uses the same global
clipboard generation as terminal/field copies, checking before formatting and
again before delivery so a later clipboard action wins. The view shows copying,
empty-selection, completion or superseded feedback. One pending delivery task
is retained per view; closing drops it. An allocation already executing remains
bounded by the snapshot and may finish before release. Desktop test compilation
and the focused clipboard-ordering test passed; native clipboard/UI acceptance
remains pending.

Measured and reduced copy-mode snapshot costs at the 8 MiB input limit.
Printable ASCII rows now map byte offsets directly to cell columns without
Unicode checkpoint construction; controls stay on the Unicode path. Truncated
history releases string capacity when it exceeds twice retained text size.
Single debug samples changed ASCII build time from 987.097 to 118.092 ms and
10,000 column lookups from 159.331 to 0.333 ms. A 65,536-line truncated snapshot
reduced tracked vector/string allocations from 11,010,048 to 2,228,224 bytes.
Unicode sample timing remained roughly unchanged. Raw data, scope and rerun
command are in docs/benchmarks/copy-mode-snapshot.json; these exclude PTY export,
UI rendering, allocator overhead and RSS and are not release latency claims.
Six behavior tests plus the explicit performance sample passed. No broad test
suite or tool installations were needed. Native acceptance remains pending.

Copy-mode paging now uses the actual viewport height and measured row heights
(where available), reusing the existing variable-height navigation helper instead
of jumping a fixed 20 logical lines. The header shows current/total lines; copy
reports byte count and empty selections leave the clipboard untouched. Feedback
updates on user actions without a timer or polling. Desktop compilation passed;
native layout/input validation remains pending while the Mac is locked.

Copy-mode vertical navigation now uses Unicode terminal-cell widths instead of
UTF-8 byte offsets and retains the preferred column across short lines. Sparse
byte/cell checkpoints are built during background snapshot loading; lookups
binary-search those checkpoints rather than rescan whole long-line prefixes.
Indices use compact u32 byte/cell pairs within the existing 8 MiB snapshot cap,
with at most one checkpoint per 256 bytes plus one per retained line. This is
not a measured native latency claim; pathological giant graphemes can still
require scanning their full context. Five focused copy-mode tests passed,
including CJK, combined emoji, short-line transitions and sparse long-line
lookups. The desktop test build passed. Native rendering/input validation,
rectangle selection and named buffers remain pending.

Copy mode now supports character ranges alongside line selection. c toggles the
mode; horizontal arrows or h/l enter character navigation, and Shift or Space/v
anchors ranges across lines. GraphemeCursor keeps combined emoji, accents and
CJK UTF-8 intact; reversing an anchored selection preserves the same text.
Character spans are highlighted and the bounded 4 KiB display window follows
the cursor on long lines. Copy remains based on full retained text. All four
focused copy-mode tests and desktop test compilation passed. A fresh desktop
check still reports the Mac locked, so no screenshot or native interaction claim
is made. Rectangle selection, visual-column vertical navigation refinement,
named buffers and native acceptance remain pending.

Copy mode now searches its retained history snapshot. / or the platform Find
shortcut opens the input; Enter runs a case-sensitive literal search, Shift-Enter
runs backward, and n / Shift-N wrap between matching lines. Search shares the
immutable buffer, runs on the background executor only when submitted, and
checks cancellation between lines when replaced or closed. Queries are capped
at 4 KiB; match indices cannot exceed the existing 65,536-line snapshot bound.
Input focus stays separate from terminal typing, and the view keeps the live
terminal's viewport unchanged. Three focused copy-mode tests passed, including
Unicode matching, cancellation and forward/reverse wraparound; desktop test
compilation passed. Native interaction/layout verification remains pending.

T14 now has a keyboard copy-mode view with a stable, bounded history snapshot.
Opening formats history off the UI thread, then renders only visible rows. The
snapshot caps text at 8 MiB and indices at 65,536 logical lines; truncation and
per-line display clipping are explicit. Navigation, anchored line ranges,
Unicode copy, Select All and mouse/Shift-click selection use that snapshot,
independent of subsequent terminal output. Its focus target traps navigation
and Paste; closing releases the buffer and restores terminal focus. Busy parser
exports report retry rather than blocking the UI. Native layout/input acceptance,
character/rectangle selection, search and named buffer semantics remain pending.
Two focused snapshot selection/bounds tests and the desktop test build passed.

Fish shell startup now automatically loads the existing directory/prompt hooks
through --init-command after normal user configuration, in login and non-login
sessions. The private integration directory follows session lifetime, the path
is passed as data via an environment variable, and startup clears that variable.
Both parent and config.fish integration opt-outs are honored. Extended the fish
fixture for config ordering, prompt preservation, login modes and config-level
opt-out. rmux-terminal compilation passed; the fish runtime fixture remains
unrun because fish is not installed (no unmanaged installer was used). Native
shell acceptance and PowerShell support remain pending.

Default font selection now uses the installed-font picker from Settings, the
command palette, and the native menu. Save Settings persists the family and
tracks family changes during saves; existing inheritance preserves explicit
per-terminal font overrides independently of text size. The custom action is
choose_default_font. The desktop test build and all 13 focused preference tests
passed, including persistence and inheritance. Native picker interaction and
visual polish remain unverified while the desktop is unavailable.

Choose Terminal Font now opens a searchable installed-font picker from the
palette/menu or custom choose_font action. Font discovery runs on the background
executor with one pending job per window, and the picker opens only while its
captured focus remains current and the window is available. Results reuse the
palette's virtual list, ranking and keyboard paths; retained entries are capped
at 2048 with an explicit limited-catalog heading. Selection changes only that
terminal's family and preserves its current size; Use Configured Font restores
configured defaults. No font is installed and no idle font scan is added.
Desktop compilation passed. Native discovery latency, font rendering, selection
and Escape behavior remain unverified; default-family editing stays in config.

The listening-port panel now owns keyboard focus and a scroll handle. Arrow keys
wrap through results, Home/End jump, Enter or the platform Copy binding copies
the selected address, R refreshes and Escape closes/restores terminal focus.
Pointer selection shares the same selected row, with focused/unfocused styling
and a compact key-help footer. Keyboard scrolling accounts for partial/churn
notices before the data rows. Moving a terminal to another window dismisses this
window-bound panel and cancels any scan. Desktop compilation passed; actual
focus routing, clipping and clipboard behavior remain native acceptance work.

Listening-port scans now accept cancellation from panel close and terminal-state
destruction. Checkpoints run before process discovery, before socket backend
creation, before each iterator read, and around the final ownership snapshot.
Close clears loading/results and advances the generation so an older completion
cannot overwrite a reopened panel. The global scan permit stays held until the
worker actually returns, preventing overlapping scans while an OS call finishes.
Four focused port tests passed, including pre-canceled entry without a live PID
and existing live loopback discovery. Mid-syscall interruption and native
close/reopen behavior are not verified or promised; cancellation is cooperative.

Port ownership now compares fresh process snapshots before and after socket
enumeration. Listener rows survive only when the PID's start time and parent,
and its entire ancestry back to the terminal root, remain unchanged. Reused,
reparented or vanished ancestors exclude their descendants; root changes fail
the scan. A notice explains when displayed candidates were discarded. Three
focused tests passed, including synthetic ownership changes and live macOS
loopback discovery. This adds a second manual process snapshot, not polling.
Process times have second resolution and socket observations are not atomic;
this reduces stale attribution without claiming to eliminate every reuse race.

T09 now includes Show Listening Ports: an explicit local TCP listener snapshot
for the terminal shell and its descendant process tree, displayed in a compact
panel with refresh, address copy and close actions. Palette/custom bindings also
provide keyboard entry/exit. Scans run off the UI thread behind one app-wide
permit; no idle polling or subprocess is introduced. Display retention is capped
at 128 address/PID rows, traversal at 100,000 socket records and the owned tree at
4096 processes. OS/backend enumeration allocations and syscall duration are not
claimed to obey those application-level caps. Closing releases displayed rows;
an in-flight scan can finish but does not publish into a closed panel.

A macOS live loopback-listener ownership test and a descendant/cycle test passed;
ARM64 Linux and macOS desktop compilation passed. Root disappearance/reuse is
checked, but process/socket snapshots are not atomic and child ownership can
change during enumeration. Native panel interaction, permission failures,
process churn and Linux/Windows runtime behavior remain pending. Automatic
metadata updates, remote discovery and PR metadata are still gaps. The adapter
is pinned netstat2 0.11.2: https://docs.rs/netstat2/0.11.2/netstat2/ .

Terminal, tab, workspace and group menus now bound their content height to the
window and keep a per-open-menu scroll handle. Arrow/Home/End navigation scrolls
the selected action into view; the initial enabled terminal action is revealed
as well. An inner scroll container handles menu scrolling while the outer menu
stops wheel events from reaching terminal contents. This complements the width
limits and prevents tall menus from simply extending beyond short windows.
Desktop compilation passed. Desktop access was rechecked and the Mac is still
locked; actual clipping, wheel routing and keyboard visibility remain unverified.

Workspace and group context menus now use the same active-keymap shortcut column,
window-bounded width and long-hint tooltips as terminal/tab menus. Their binding
lookup uses the captured return focus, not the menu's temporary key context.
Custom bindings now also expose group_current_workspace, edit_group_color,
edit_group_icon, toggle_group_pin, ungroup_workspaces, close_group,
remove_workspace_from_group, set_group_anchor and close_workspace. These reuse
existing actions and confirmations; no new default shortcuts were assigned.
Desktop compilation passed; rendered menu and native key-dispatch acceptance
remain pending.

Terminal and tab context menus now show right-aligned shortcut hints resolved
against the targeted terminal's active keymap. They share the command palette's
highest-precedence binding lookup instead of hardcoding platform defaults;
custom chords and disabled/overridden bindings follow the same lookup path.
Menu widths are capped against the window, labels can truncate, and long shortcut
hints have a bounded column and full-text tooltip. Desktop compilation passed.
Rendered spacing, small-window clipping and live shortcut-label behavior remain
native acceptance work while the Mac is locked.

Linux/XDG notifications now retain async action handles and share macOS's
click-to-current-terminal routing and 16-listener admission cap. The notification
advertises the default Open Terminal action. Waiting races a two-minute deadline;
on expiry a CloseNotification request gets its own two-second bound before the
handle is dropped. This avoids a thread per banner and does not trust servers to
honor expiry hints. The pinned zbus backend supplies asynchronous signal waits.
ARM64 Linux cross-compilation and macOS desktop compilation passed. No Linux
notification server or display session was exercised; banner actions, dismissal,
D-Bus disconnects and compositor focus permissions remain runtime acceptance work.
Windows click handling remains pending as native Windows validation is deferred.

macOS banners now have asynchronous click-to-terminal routing. Clicks resolve the
surface's current workspace/window, honor modal/busy interactions, and ignore
closed terminals, dismissed history records or a disabled notification setting.
At most 16 response listeners are retained; further OS deliveries are skipped
until a listener finishes. Each listener has a two-minute timeout that removes
its banner; no per-banner blocked thread or recurring polling is used.

Before enabling this path, inspection found mac-usernotifications 0.3.1 consumes
PendingGuard via ManuallyDrop, leaking its request-id String and abandoning
registration cleanup while waiting. A vendored patch keeps the guard alive and
borrows the receiver instead. Both polled-cancellation and unpolled-drop tests
passed without touching Notification Center; desktop compilation passed. Current
fire-and-forget delivery did not exercise that flawed response path. Vendored
provenance and test scope are in vendor/mac-usernotifications/RMUX-PATCHES.md.
Actual macOS banner clicks, timeout removal and moved-window routing remain
unverified; Linux/Windows click handling remains pending.

Native notification Settings now distinguish Terminal unfocused from App in
background. The persisted notification_focus policy is checked both before
queue admission and before OS dispatch; focused terminals remain suppressed in
either mode. Window activation events track app focus across live windows, with
closed windows excluded, rather than relying on GPUI macOS mainWindow lookup.
No polling or additional OS backend initialization is introduced. Three focused
notification tests and the file-backed settings merge/persistence test passed.
Native multi-window activation ordering, permission and banner delivery still
need platform verification; already-submitted OS requests are not recalled.

N03 now has an opt-in native notification dispatcher and persisted Settings
control. The default remains off and does not initialize an OS backend. Delivery
uses one lazy consumer, at most eight queued weak terminal references and a
one-per-second admission limit; excess native banners are dropped while in-app
history remains independent. Before dispatch it rechecks the setting, focus,
read/dismissal state and terminal lifetime. Backend calls run off the UI thread;
errors appear in the originating terminal. Titles/bodies are bounded and XDG
markup is escaped. The pinned notify-rust 4.18.2 adapter uses the modern macOS
UserNotifications backend; its bundle check precedes permission requests.
Windows uses rmux's own AppUserModelID, whose installer registration remains
pending. Source: https://docs.rs/notify-rust/4.18.2/notify_rust/ . Native delivery,
permission denial, click-to-focus, app-wide focus policy, OS resource costs and
Linux/Windows acceptance remain unverified. No OS notifications were enabled or
sent during development. Desktop compilation and three focused tests passed:
UTF-8/control-character payload bounds, flood admission, and persisted-control
comparison. These tests do not establish native delivery or permission behavior.

Added a native-rendered terminal context menu for Copy, Copy with Line Breaks,
Paste, Select All, Find, working-directory copy/reveal, and Clear Scrollback.
Right-click opens it when the terminal application does not consume mouse input;
Shift-right-click overrides application mouse reporting. Palette Terminal Actions
and custom terminal_menu bindings provide keyboard entry. Arrow/Home/End keys
navigate enabled entries, Enter/Space activate, and Escape/outside click dismiss.
The menu uses existing action paths, restores terminal focus on dismissal, clears
on window rebinding, and blocks conflicting automation while open. Desktop
compilation passed. The Mac remains locked, so native visuals, hit testing and
TUI mouse interaction are not yet verified.

Modifier-click hyperlink lookup now uses a nonblocking parser lock. Under output
pressure it reports a retry message instead of waiting on the UI thread. Existing
engine URL detection (including fully visible soft-wrapped URLs and OSC 8
precedence) remains authoritative. The held-parser test verifies prompt return;
the existing hyperlink tests cover detection and viewport behavior. Native click
latency and hover feedback remain unverified/incomplete.

Terminals now show a compact out-of-flow Paste pending indicator with a Cancel
Queued Paste control. Accepted clipboard input requests a repaint; writer
completion/cancellation wakes the existing update channel after releasing the
paste permit, so quiet children do not leave stale status. No polling timer or
terminal resize is introduced. The blocked-writer test now verifies that its
completion callback sees the released permit. All seven focused paste tests and
desktop compilation passed. Native appearance/hit testing remains unverified;
cancellation still cannot interrupt a write already in progress.

The production PTY writer loop is now shared with a deterministic blocked-write
regression test. It holds an earlier write, queues a bracketed paste and later
ordinary input, cancels the paste, then releases the writer. Observed output is
exactly BEFOREAFTER and the paste permit is released. All seven focused paste
tests passed, including the existing real-PTY large-paste ordering test. This
proves queue behavior with a controlled Write implementation; OS-specific PTY
blocking/cancellation timing and native UI interaction remain unverified.

Cancel Queued Paste now exposes a per-terminal cancellation generation through
Edit, palette and custom bindings. The writer checks it before/after encoding
and skips whole canceled paste messages while retaining ordinary queued input.
Once write_all begins it finishes the payload, preserving bracketed delimiters.
The paste permit remains held until the canceled queue slot is processed. Six
focused paste tests and desktop compilation passed, including cancellation
before encoding and subsequent slot reuse. Native cancellation timing and a
blocked real-PTY cancellation scenario remain pending; in-progress writes are
explicitly not interruptible by this action.

Native clipboard paste now queues its owned text plus captured bracketed mode
before encoding on the sole PTY writer thread. Later keystrokes stay behind that
queue slot, and the paste permit remains held through encoding/write completion.
UI dispatch retains bounded size/mode checks without payload normalization or a
second clipboard-text copy. A bounded input-error channel reports asynchronous
encoding/write failures. Desktop compilation and five focused paste tests passed,
including large-paste ordering, sanitation/limits, automation viewport and permit
lifecycle. Native latency/output-pressure measurements and active-write cancel
semantics remain pending; cancellation during write_all is not implemented.

Native paste no longer waits on the terminal parser mutex to read bracketed
paste mode. Busy-parser rejection queues no input, releases its paste permit
and displays an explicit retry message; successful retry clears the error.
Oversized clipboard input is rejected before parser access. Desktop compilation,
the held-parser nonblocking test and large bracketed-paste ordering test passed.
Encoding remains on the UI thread within the existing 1 MiB input bound, and
already-writing paste cancellation is not implemented; native clipboard and
output-pressure timing acceptance remain pending.

Repeated Copy/Reveal Working Directory requests now supersede earlier requests
instead of being ignored while a lookup is pending. Workers use a weak request
generation plus clipboard ordering, check cancellation before/after parser
access, and poll only while a pending parser lock is busy. Closed terminals and
obsolete requests stop waiting; there is no idle timer. Desktop compilation and
the focused session test passed, including cancellation while the parser lock
remains held. Native repeated clipboard/reveal interaction remains unverified.

Tab context menus now expose Copy Working Directory and Reveal Working Directory
for the clicked terminal identity, reusing the existing asynchronous lookup,
clipboard ordering and missing-local-directory error handling. Both actions are
also available to custom shortcuts. Desktop compilation passed; native menu
layout, clipboard and file-manager activation remain pending acceptance.

Custom shortcuts now expose numbered tab, pane and workspace navigation actions
for 1–9, dispatching the existing typed commands without changing their ordering
or target semantics. Canonical one-digit suffix validation rejects zero, leading
zeros, negatives and values above nine. A focused test passed for all 27 action/
number combinations and invalid cases; the desktop test target compiled. Native
chord navigation/focus acceptance remains pending.

Pending shortcut chords now show their keystrokes and Waiting for next key in
a small out-of-flow window overlay. It observes GPUI pending-input changes,
adds no polling timer, and neither moves keyboard focus nor participates in
terminal layout. Existing GPUI completion/timeout/replay handling is unchanged.
Desktop compilation passed; native visibility, clearing on focus changes and
timeout, occlusion and shortcut routing remain pending visual acceptance.

Custom two-stroke shortcuts now allow an unmodified suffix, such as `ctrl-k t`,
while retaining the modified/function-key leader requirement and key/prefix
validation. Three focused shortcut tests passed using GPUI's real keymap:
leader pending state, completed action dispatch, bare suffix non-binding and
context isolation, plus existing validation/override checks. Installed GPUI
source confirms a one-second incomplete-chord timer and replay path; native
timeout, mismatch replay, focus changes and terminal input acceptance remain
pending. This does not implement Ghostty key-table/config import parity.

Closing Terminal Memory now releases the window's cached process sampler and
signals cancellation to an in-flight sample. Workers check before OS sampling,
after acquiring the sampler and between terminal samples; existing generation
checks suppress stale results. Cache release never waits on the worker's mutex.
Reopening starts a fresh CPU baseline. Desktop compilation passed. An OS query
already in progress may finish before its worker releases the last sampler Arc;
native close/reopen behavior and measured RSS reclamation remain unverified.

Open Settings panels now observe shared preferences and settings-file activity
across windows, including restored and detached-window construction paths.
Save/Open show a busy state during file work instead of appearing actionable
while the existing operation guard ignores requests. Controls remain editable,
and stale-save notices still protect newer edits. Unchanged bounded adjustments
no longer republish global preferences and invalidate terminal views. Desktop
compilation passed. A fresh native check still reports a locked Mac; cross-window
visual updates, busy-state contrast and screenshots remain pending acceptance.

Settings keyboard scrolling now keeps history help inside its control row,
restoring the correct child positions for Save and Open all settings. Page
Up/Down moves by the measured viewport and control strides, including wrapped
rows and footer spacing. Existing arrows/Tab/Home/End retain their behavior.
Three shared navigation tests passed and the desktop test target compiled;
native short-window scrolling and focus acceptance remain pending.

Settings now exposes the per-new-terminal scrollback budget as New terminal
history, with 1 MiB steps bounded to 0–256 MiB and an explanation that existing
history is preserved. Save these settings now persists this control and includes
it in unsaved-change detection while retaining unrelated configuration fields.
Three focused persistence tests and the settings navigation/bounds test passed.
Native control layout and new-terminal retention acceptance remain pending;
this setting is not a total app/process memory cap.

Focused cursor shape can now be set to Application (default), Block, Bar or
Underline through configuration and Settings, with explicit Save persistence.
The renderer combines this preference with the existing unfocused hide/hollow
policy and IME bar caret; engine-hidden cursors remain hidden. No blink timer
was added. The focused cursor test passed for default application behavior,
explicit shapes, composition/unfocused precedence, serialization and change
detection; desktop test compilation passed. Native cursor rendering and settings
keyboard/layout acceptance remain pending.

Settings now exposes all four terminal themes as direct, wrapping choices with
a selected-state border, rather than requiring repeated clicks through a cycle.
Keyboard Left/Right cycling remains available. Named Terminal Theme palette
commands apply the same typed action, preserve explicit overrides and skip
global preference publication when the selected theme is already active. Save
these settings remains the explicit persistence action. Desktop compilation
passed; native wrapping, contrast and keyboard-focus acceptance remain pending.

Terminal themes now include Solarized Dark/Light using the author's canonical
ANSI mapping. Settings cycles through Dark, Light and both Solarized presets;
configuration accepts `solarized_dark`/`solarized_light`. Explicit colors and
palette overrides retain precedence, and existing live settings propagation
applies the presets without recreating terminals. The focused theme test passed
for decoding, serialization, cycle direction and overrides; the desktop test
target compiled. Native color/contrast and Settings layout acceptance remain
pending. This adds two built-in themes, not a complete theme catalog/importer.

Asynchronous splits now capture source font size/family at request time instead
of inheriting whichever tab happens to be active at completion. The ready seed
carries this appearance through attachment. Inherited defaults reconcile with
settings changed during launch; custom attributes remain preserved. The focused
font-inheritance test passed and the desktop test target compiled. Native tab
switching/settings changes during a pending split remain unverified.

The pending-split status/cancel control is now an out-of-flow overlay rather
than a row in the workspace layout. This removes the indicator's contribution
to terminal sizing, avoiding resize/reflow solely for pending/canceling status.
It uses a bounded width, truncated label, and pointer-event occlusion so clicks
and scrolling on the control do not reach terminal content beneath it. Desktop
compilation passed; native narrow-window placement, hit testing and actual PTY
resize counts remain pending verification.

Pending splits now have a Cancel Split Terminal control, File/palette action,
and `cancel_split` custom binding. Cancellation suppresses late attachment/errors
and retains the launch slot until work completes, bounding cancel/retry jobs.
Workspace destruction and window rebinding also signal cancellation. Splits and
tabs share the same pre-/post-spawn cancellation boundary; its two focused tests
passed, including reaping an unattached real PTY child. Native cancel/focus and
in-flight frame-preparation behavior remain unverified. Already-running OS spawn
or engine preparation cannot be forcibly interrupted.

Interactive split startup now prepares initial engine colors/palette and the
first frame on the background executor before attaching the terminal view.
Settings changes during launch trigger background re-preparation. Ready and
raw-session inputs share the existing split validation/attachment path, and
raw-session preparation failures now use its error handling rather than panic
in TerminalView::new. Desktop compilation and the live socket smoke passed,
including targeted nested splits, creation and cleanup. That smoke exercises
the shared attachment path, not native asynchronous Split actions; their timing,
focus, cancellation and visual acceptance remain pending. Layout/resize work
still runs on the UI thread.

Interactive split commands now launch the PTY shell on the background executor,
with one pending launch per workspace and a Starting split terminal indicator.
Completion resolves the original pane identity and rechecks split eligibility;
focus is acquired only if the original focus remains current. Missing targets,
shutdown, workspace destruction and window rebinding discard pending attachment.
Launch failures preserve existing panes. Desktop compilation passed. Native
focus/process cleanup and latency evidence remain pending; initial frame/color
preparation still runs on the UI thread, and workspace/RPC creation paths are
unchanged. This is not evidence that all split-startup stalls are eliminated.

Group-header context menus now act on the displayed group rather than silently
editing its anchor workspace. They expose group name/color/icon, pin/unpin,
Ungroup Workspaces (preserving terminals), and the existing Close Group prompt.
Activation verifies captured group ownership before applying an action, avoiding
retargeting if membership changes. The palette's Show Workspace Actions remains
workspace-specific for editing anchors individually. Desktop compilation passed;
native grouped-row interaction and visual acceptance remain pending.

Workspace sidebar rows now offer a context menu for name, description, color,
icon, pin/unpin and policy-aware close. Opening selects the captured workspace
identity and clears multi-selection. Pointer and keyboard navigation share one
highlight; Escape restores prior terminal focus, and outside click/focus loss
dismisses. Show Workspace Actions in the palette and `workspace_menu` custom
bindings provide keyboard access. Automation mutations report busy while the
menu is open. Desktop compilation passed; native placement, focus, grouped-row
interaction and screenshots remain pending acceptance.

The tab context menu now exposes Move Tab Left/Right using the existing
entity-preserving reorder path, with disabled end-of-strip actions. Pointer
movement selects an enabled row for subsequent keyboard activation, avoiding
separate hover and keyboard highlights. Desktop compilation passed. Native
interaction/screenshot acceptance remains pending: a fresh computer-use check
reported that the Mac is locked and automatic unlock failed.

Tab actions can now be opened with Shift+F10, Show Tab Actions in the palette,
or a `tab_menu` custom binding. Keyboard opening anchors below the tab strip.
The context menu dims batch actions without targets or while a tab launch is
pending; arrow/Home/End navigation skips unavailable choices and activation
rechecks availability. Desktop compilation passed. Native anchoring, disabled
contrast and keyboard/focus behavior remain pending visual acceptance.

The tab strip now has a right-click menu for Rename, Close, Close Other Tabs
and Close Tabs to the Right. Opening selects the target tab, then gives the
menu keyboard focus; arrows/Home/End navigate, Enter/Space activates and Escape
returns focus. The menu anchors at the pointer, fits within window bounds and
dismisses on outside click/focus loss. Actions resolve the captured tab identity.
Close-icon clicks now require a standard click, preventing a right-click from
closing the tab. Desktop compilation passed; native placement, hit testing,
keyboard routing and visual acceptance remain pending.

Close Tabs to the Right now shares the policy-aware batch-close implementation,
with menu/palette access and a configurable `close_tabs_to_right` action. It
captures the original right-side identities, preserving left-side and newly
added tabs even if ordering changes before confirmation. Three focused tab-order
tests passed, including that reordered-target scenario. Native prompt, focus
and process lifecycle acceptance remain pending.

Close Other Tabs now removes its captured targets as one pane update: selection
is resolved once, scrolling/focus repair runs once, and one metadata event is
emitted. A newer selection is retained if it survives; otherwise the kept
terminal becomes active. Focus changes only when a removed terminal contained
focus. The two focused tab-order tests passed and the desktop test target
compiled. Native batch focus/process lifecycle and latency remain unverified.

Close Other Tabs is now available in the menu, command palette and configurable
shortcuts. It preserves the selected terminal, applies the shared active/always/
never close policy once for the batch, and captures target identities before
confirmation. Reordered targets still resolve correctly; newly added tabs are
excluded, and a missing keeper aborts the operation. Focus-preserving removal
reuses the existing tab-close path. Two close-policy tests and two tab-order
tests passed; native prompt, process cleanup and focus acceptance remain pending.

Asynchronous tab completion now reconciles inherited font size/family against
settings changes made during startup. Values matching the launch-time defaults
adopt current defaults; custom size/family choices remain intact. This avoids
installing an old default as an accidental permanent override. The existing
focused font-inheritance test passed for both inherited and custom attributes,
and the desktop test target compiled. Native in-flight settings interaction
remains unverified.

The asynchronous tab launcher now shares its cancellation/prepare boundary with
two focused lifecycle tests. Pre-canceled work never invokes the launcher;
cancellation immediately after starting a real PTY child returns an error and
reaps that unattached child. Both tests passed on macOS. This provides process
cleanup evidence for the worker boundary, while native button/focus behavior,
mid-preparation cancellation and window-transfer interaction remain pending.

Pending new-tab launches now have a cancel control, menu/palette action and
`cancel_new_tab` shortcut action. Cancellation signals the background launch,
skips preparation when observed after process startup, and suppresses late tab
attachment/errors without touching existing tabs or focus. The launch slot
remains occupied until completion, preventing cancel/retry job accumulation.
Pane destruction and window rebinding also signal cancellation. Desktop
compilation passed; native cancel/cleanup behavior remains unverified. An OS
spawn or frame preparation already in progress is not forcibly interrupted.

New-tab startup now also prepares engine colors/palette and the initial frame
in its background task. The UI attaches a prepared session instead of immediately
reacquiring the parser for those operations. A color/palette change during
startup triggers background re-preparation, and frame-preparation failures use
the existing tab launch error path. Desktop compilation and the live socket
smoke passed after extracting shared initialization; that smoke exercises normal
window/split startup, not the new-tab action itself. Native asynchronous tab
focus/cancellation acceptance and latency measurement remain pending.

New Terminal Tab now launches its PTY in a background task using the cached
working directory, inherited font settings and configured scrollback budget.
Each pane permits one pending launch and shows Starting…; existing terminals
remain usable. Completion selects the new tab only if the original focus is
still focused. Launch failure preserves existing tabs. Pane destruction/window
rebinding cancels pending UI work, and shutdown rejects late completions.
Desktop compilation passed. Native pending-state/focus/failure acceptance and
startup latency measurement remain pending. Initial frame/color setup still
runs in TerminalView::new on the UI thread; splits/workspaces have separate
startup paths that are not made asynchronous by this change.

Repeated terminal Copy actions now replace pending copies rather than being
ignored. The new generation invalidates earlier background work before its UI
task is canceled, including transitions between ordinary Copy and Copy with
Line Breaks. Only the newest request can publish clipboard content; the existing
worker cancellation checks handle stale queued/contended work. The clipboard
generation test passed and the desktop test target compiled. Native rapid-copy
interaction remains unverified; already-running formatting is not interrupted.

Search workers now discard queued navigation when a scan completes without
matches, rather than consuming up to 64 no-op steps or carrying those steps
into later output. Five focused worker tests passed. The new regression queues
64 moves before an empty scan completes, adds matching output afterward, checks
that no selection appears, then verifies a fresh navigation request selects it.
This is worker-level behavior evidence, not native interaction acceptance.

Find navigation with zero matches now returns search-owned progress without
acquiring the parser lock or invalidating terminal snapshots. A no-value
navigation result also no longer increments the terminal revision. Three
focused engine search tests passed, including a regression asserting unchanged
revision, selection and viewport for both empty and unmatched queries. Valid
history/Unicode/alternate-screen navigation remains covered; no GUI timing
claim is made.

Fresh macOS debug binaries passed `mise run smoke-socket` with selected-text
wrap routing coverage. The real CLI reached the background selection exporter
and returned empty/untruncated text for a surface without a selection. Invalid
wrap scopes, string/null booleans preserved request IDs and returned -32602; a
second client still completed ordinary selection reads. Existing split, history,
notification and cleanup scenarios passed too. This verifies live routing and
empty-selection semantics; nonempty wrapped selections remain engine-tested,
and native selection/clipboard interaction is still pending.

Selected-text automation now exposes the wrap choice through selection-scope
`surface.read_text` with `preserve_wraps:true`, and `read-screen --selection
--preserve-wraps` with help/completion support. Other scopes reject the enabled
option. Exports retain bounded background execution and UTF-8-safe truncation.
All twelve CLI tests and the focused engine wrap/truncation regression passed;
desktop and CLI compilation passed. Live RPC acceptance of this option remains
pending.

T19 now includes an explicit Copy with Line Breaks action in the Edit menu,
command palette and configurable terminal shortcuts. It preserves soft-wrap
boundaries while ordinary Copy continues to unwrap them. Both modes share
the 16 MiB background streaming exporter and cancellation guards. A focused
engine regression passed for both modes, exact/over-limit output and unchanged
selection/viewport; desktop compilation passed. Native clipboard acceptance,
full reflow parity and per-session shell history remain open.

Tab-picker selection now reconciles against title/directory metadata changes.
If the chosen tab stops matching, the first remaining result becomes the stored
selection and is revealed; the former tab cannot reclaim selection merely by
matching again. Empty results clear the stored identity. Metadata updates leave
scroll position unchanged when the selected tab still matches. All five terminal
metadata subscription paths use the reconciliation. Desktop compilation passed;
native live-update interaction remains pending.

The tab picker now pages by its measured viewport and row heights, accounting
for directory subtitles instead of jumping eight results. Empty/whitespace-only
queries return original tab order directly, avoiding per-tab lowercase title/path
allocations, scoring and sorting. Three focused navigation tests passed and the
desktop test target compiled. Native small-pane paging remains unverified; no
latency or memory reduction has been measured for the empty-query fast path.

Clipboard cancellation now also stops waiting for a busy parser. Pending copies
use opportunistic lock attempts with 1 ms backoff and recheck their generation
on contention; there is no idle polling. A focused regression held the parser
lock, superseded the copy and observed completion before releasing the lock.
The session test passed. Cancellation still does not interrupt formatting once
it has started; no scheduler-latency guarantee is implied by the backoff.

Background clipboard exports now check their generation before engine access
and again after acquiring the parser lock. A newer rmux clipboard operation
therefore skips obsolete selection formatting, including work superseded while
waiting for output parsing. The existing completion guard still prevents stale
clipboard writes. A focused session test passed for valid copies, cancellation
at both checks, and unchanged selection; desktop compilation passed. This does
not interrupt an already-running formatter or an in-progress lock wait.

An environment-controlled close policy now displays its effective value in
Settings with a read-only Environment indicator instead of a no-op button. Its
tooltip explains RMUX_CONFIRM_CLOSE; keyboard attempts show the same actionable
explanation without republishing unchanged preferences. Desktop compilation
passed. Native tooltip, wrapping and notice presentation remain unverified.

Settings rows now wrap labels and grouped controls when horizontal space is
limited. Numeric stepper values are plain centered labels with a stable minimum
width instead of clickable no-op buttons. Desktop compilation passed. A fresh
native inspection attempt found the Mac locked and automatic unlock unavailable;
small-window layout and screenshots remain pending, not accepted by compilation.

A reproducible `mise run bench-selection` fixture now measures bounded rejection
and complete selection formatting at 4/64 MiB retention. A same-host comparison
against the previous two-pass implementation recorded complete-export medians
about 47–51% lower; the large-selection 4 KB rejection dropped from 11,200 to
23 µs. [Raw results and limitations](benchmarks/selection-export.md) are recorded.
These are single-run engine observations, not clipboard/UI or multi-pane gates.

Complete selection export now shares the streaming formatter instead of probing
the full formatted size and then formatting a second time. The owned byte
buffer becomes the returned String without another content copy. Oversized
selections still return an error, exact-limit Unicode selections still succeed,
and the default clipboard limit remains 16 MiB. Seven focused selection-related
tests passed, covering copy limits, Unicode, alternate screens and preservation
of viewport/selection. End-to-end clipboard timing remains unmeasured.

Find selection seeding now uses the bounded streaming selection exporter. The
previous size-limited API still formatted the whole selection once to measure
it before rejecting oversized content. The new path stops output at 4 KB and
rejects truncated seeds, retaining the existing remembered-query fallback,
nonblocking lock attempt and single-line filtering. Four focused text-export
tests passed, covering early writer termination, UTF-8 boundaries, selection
and viewport preservation. No end-to-end latency improvement is claimed yet.

Notification Page Up/Down now uses the visible scroll viewport and measured card
strides (including gaps), sharing the palette's bounded paging algorithm instead
of jumping eight messages. Expanded cards participate at their actual height;
page movement advances at least one message and clamps at list edges. All three
focused navigation tests passed, and the desktop test target compiled. Native
expanded-card scrolling and layout measurements remain to be visually accepted.

Group header notification previews now choose the newest unread message across
all members, including collapsed members, instead of the first member with an
unread message. The sidebar retains the existing arrival sequence alongside its
shared message cache and compares cached values; this adds no terminal parser
locks or history scans. Desktop compilation passed. Native grouped-sidebar
visual acceptance remains pending.

Notification mutation routing now has a focused contract test for both
`notification.mark_read` and `notification.dismiss`: missing/null, negative,
fractional and string revisions, unexpected content/internal fields, and
noncanonical surface IDs are rejected before queueing UI work. Valid requests
retain `u64::MAX` revision precision, preserve the request ID, and route to the
correct operation. The test passed after correcting its fixture to use a
generation-bearing surface ID; no production precision defect was found.

Fresh macOS debug binaries passed `mise run smoke-socket` with the new
`read-notification` CLI scenario. The live app retained both messages, cleared only
the targeted unread flag, preserved Unicode title content, returned false for
repeat/missing revisions, and kept workspace selection unchanged. The existing
socket lifecycle, nested split, OSC history, process cleanup and reconnect checks
also passed in that run. This is native-process RPC evidence, not screenshot or
panel interaction acceptance. N02's implementation record now reflects the newer
history/panel behavior and acknowledgement evidence.

Individual acknowledgement is now available through `notification.mark_read`
and `rmuxctl read-notification --surface ID --revision NUMBER`, including local
help and shell completion. It reuses the panel's read-state/selection update path,
retains history and does not focus the terminal. Missing/already-read revisions
return marked_read:false; unknown surfaces remain errors. All eleven CLI tests
passed and the desktop compiled. Live RPC acknowledgement and native panel
synchronization still need acceptance.

Notifications now support per-message Mark read through a card button or R.
History is retained, other unread messages remain unread, and bell/status attention
is independent. In Unread mode the shared selection-repair path chooses the next
surviving message; All mode keeps selection on the retained record. Five focused
notification tests passed, including individual read-state/cache behavior. Native
panel focus, selection and badge updates remain unverified; RPC read-state
mutation is not part of this slice.

Notification cards now clamp previews to two title lines and four body lines.
More/Less or E expands the selected message in place, with one expanded identity
per open panel. Copy still exports complete content; expansion does not acknowledge
alerts or focus the terminal. Card controls wrap in narrow panels. Desktop
compilation passed; actual clamping, expanded scrolling and narrow layouts still
need native visual acceptance.

Notification cards now offer Copy, and the focused panel handles Command-C on
macOS / Control-C elsewhere for the selected message. Copy formats title and body
without extra blank lines for empty fields, uses the shared clipboard ordering
path, and does not acknowledge or open the terminal. Four notification tests
passed, including Unicode/multiline copy formatting; desktop compilation passed.
Native clipboard, focus and card-layout verification remain pending.

Notification panel construction now filters read records before cloning message
handles or building labels in Unread mode. Visible messages from one terminal
share a single formatted workspace/tab label instead of copying both titles and
formatting them for every card. All/Unread counts are collected during the same
history walk. Desktop compilation passed. This reduces repeated allocations by
construction; no frame-time or memory improvement has yet been measured.

The notification panel and its keyboard target list now sort by descending
arrival sequence across the window. Both All and Unread filters share that order;
selection remains identified by terminal/revision rather than a row index.
Moving workspaces or panes no longer reorders the feed. Three focused history
and arrival-order tests passed and the desktop compiled. Native selection/scroll
behavior during incoming alerts still needs acceptance; RPC listing order is
unchanged.

Workspace notification previews now choose the newest unread message across
all panes/tabs using a process-wide arrival sequence, instead of the first pane
with unread history. Per-terminal cached previews include that sequence, so the
sidebar does not scan each 32-message ring. Moving panes therefore does not change
preview priority. Three notification tests passed, covering interleaved terminal
arrivals, dismissal, read state and eviction. Native sidebar verification remains
pending; notification panel ordering is unchanged.

Tab drop targets now use a blue leading border with a subdued background to
indicate insertion before the target; the trailing append area uses the same cue.
Border space is reserved outside a drag so the cue does not change layout width.
The active tab also retains that leading accent alongside its existing top accent.
Self-drops are excluded from accepted targets and hover cues. Desktop compilation
passed; native drag screenshots and visual parity acceptance remain pending.

Cross-pane tab drops now resolve an explicit destination tab before detaching
the source. A stale target leaves both panes untouched instead of silently
appending. Explicit end-of-bar drops still append. The validated insertion index
is passed through the synchronous transfer, removing the ambiguous fallback from
the insertion method. Desktop compilation passed; native drag/close race
acceptance remains pending.

Tab drops now consistently insert before the target tab within and across panes.
A trailing 20 px drop area appends to the end, represented explicitly as an absent
target rather than a fabricated tab identity. The moved tab remains selected;
keyboard and drag reordering emit metadata updates for dependent window UI.
The focused tab-order test passed for both directions, adjacency, self-drops and
append. Native drag hit targets, overflow and metadata refresh remain unverified.

Closing or moving out the active workspace now chooses its successor using
pinned/grouped navigation order, falling back to the previous row at the end.
The chosen identity survives storage removal and group-slot adjustment. Closing
a group anchor still prefers its promoted member, and closing an inactive
workspace retains the active identity. Thirteen group tests passed, including
logical successor selection and existing anchor/order tests. Native close/move
focus behavior remains pending.

Saving settings now inspects configuration symlinks without following them
first. A dangling link produces a resolution error and remains intact instead of
being replaced by a regular file; valid relative links still update their target.
Two focused save tests passed on macOS, covering both symlink cases, preserving
invalid configuration contents and merging unrelated settings. Windows symlink
acceptance remains deferred with native Windows validation.

Background configuration reload now compares its starting settings-panel values
with the current session before applying the file. If the user edited controls
while the read ran, it discards that reload result and explains that the newer
edits were preserved; a later explicit reload can replace them. This covers both
keyboard and mouse changes across windows, since preferences are global. Desktop
compilation passed; native concurrent reload/edit acceptance remains pending.

Settings save completion now compares the saved snapshot with the current
session controls. If changes were made during the background write, the notice
explains that the earlier values were saved and asks the user to Save again for
the newer ones. The comparison covers exactly the persisted appearance/behavior
controls and excludes an environment-overridden close policy. All twelve focused
preferences tests passed, including file preservation and comparison coverage.
Native concurrent-adjustment/save feedback remains unverified.

Single-line editors now show a compact “Too long” indicator when a paste or IME
update exceeds the existing 4096-byte limit. Its tooltip explains that existing
text was retained; successful edits and undo/redo clear the indicator. Paste size
is checked before allocating the newline-normalized copy, with subtraction-based
bounds checks in both input paths. Desktop compilation passed. Native narrow-field
layout, tooltip and oversized IME interaction still need visual verification.

Single-line text fields on macOS now support Command-Backspace to delete to the
start and Command-Forward Delete to delete to the end. Selected text takes
precedence, Unicode grapheme boundaries remain intact, and edits use the existing
undo/event path. Ten composition tests passed, including edge deletion within an
emoji sequence, selected spans and field boundaries. Native shortcut delivery
and IME interaction remain pending.

Copy/Reveal Working Directory now resolve the terminal's reported path on a
background executor instead of taking the parser mutex on the UI thread. One
lookup per terminal is allowed in flight; queued lookups hold only a weak engine
reference. Directory copies participate in the shared clipboard generation guard,
so newer rmux copy requests win over a delayed lookup. Missing/remote paths retain
the existing error behavior. Desktop compilation and the focused live-PTY session
test passed, including lookup result parity and queued ownership checks. Native
clipboard/Finder interaction and end-to-end latency remain unverified.

Search workers now check query revision after initialization and again before
queued navigation. A query superseded while registration waited for the parser
is skipped instead of scanning and installing an obsolete selection. Four focused
worker tests passed, including a lock-contention regression that queues an old
match/navigation, replaces it with a no-match query, then verifies the terminal
has no stale selection. Navigation already in progress when an edit arrives is
not made atomic with that edit by this change.

Search-worker initialization now checks cancellation while waiting for the
parser lock, using a 1 ms backoff only during registration contention. Closing
Find can terminate an unregistered worker without waiting for the parser to
release its lock. A deterministic test holds that lock, waits for the worker to
consume its query, then verifies cancellation closes the result channel without
publishing. All five search tests passed, including Unicode/alternate-screen
search and worker navigation/refresh. Existing searches still require the parser
lock to release tracked references safely; this change concerns initialization,
not a guarantee of immediate cancellation during every search operation.

Opening Find no longer registers the Ghostty search on the UI thread. The
on-demand search worker now initializes it when processing its first command,
keeping parser-lock waits in that worker. Selection-prefill uses a nonblocking
lock attempt and falls back to the remembered query while the parser is busy.
Two focused worker tests passed: opening/queuing under a deliberately held parser
lock, plus latest-query replacement, navigation, refresh and shutdown. This
removes an identified UI wait; end-to-end input latency is not yet measured.

Notification dismissal now repairs selection through one shared path for mouse,
keyboard and RPC/CLI requests. Removing the selected message selects the next
surviving row (or previous at the end), reveals it, and clears selection when the
list becomes empty. Dismissing an unselected row leaves the current selection and
scroll intent alone. The first-row fallback also handles a stale selection after
history eviction. Desktop compilation passed; native interaction remains pending.
Native UI access was retried this cycle: the Mac is still locked, so no screenshot
or visual acceptance was claimed.

Command palette Page Up/Down now uses the virtual list's actual viewport height
and each result's title/subtitle row height instead of a fixed eight-result jump.
The destination stays within a page including the starting row, with a minimum
one-row advance in very small windows and clamping at list boundaries. Three
focused palette tests passed, including mixed-height rows, tiny viewports, empty
results and out-of-range selection. Native keyboard/resize interaction remains
pending; the separate tab picker and notification list still use fixed paging.

Tab overflow browsing no longer jumps back to the active tab on every terminal
metadata update. All five subscription paths (new, restored, added, inserted and
batch-transferred tabs) now repaint without changing the scroll target; explicit
selection, reordering and removal still reveal the active tab. Tab rendering also
reuses one shared title for display, drag preview and tooltip rather than creating
the title twice. `cargo check -p rmux-desktop` passed. Native overflow browsing
under progress/title updates remains to be checked; no measured performance gain
is claimed for the allocation reduction.

Directional joins now use the shared geometric neighbor selection, available
from the View menu, palette and custom shortcuts. Both next-pane and directional
joins share the existing batch transfer: destination tabs remain first, source
tab order is retained, and only the source active tab is selected after transfer.
The desktop compiled and two focused shortcut tests passed. Code inspection
confirmed the batch path avoids selecting intermediate unread tabs; native
multi-tab order, unread-state and directional interaction acceptance remain open.

Directional pane swapping is now available from the View menu, command palette
and four custom shortcut actions. Swaps share directional focus's geometric
neighbor selection and the existing identity-preserving tree swap; pane order
stays aligned with the tree for cycling and persistence. Edge/zoomed operations
are no-ops. The desktop compiled and all seven focused layout tests passed,
covering nested geometry, swaps and saved indices. Native interaction and
cross-platform acceptance remain pending.

Fifteen existing layout/workspace actions now accept custom shortcut bindings:
equalize, swap, break in either direction, join, tab/workspace movement and
detachment, rename, pin and metadata editors. They dispatch existing typed menu
handlers with terminal-context defaults; no new default shortcuts were assigned.
The guide includes a shared-prefix chord example. Focused shortcut tests passed;
native chord dispatch for these actions remains pending.

Notification history now maintains a shared pointer to its latest unread message.
Sidebar preview retrieval and unread checks no longer scan the 32-record ring on
each render; only removal of the cached message searches for its predecessor.
Mark-read skips already-read histories, and clear/eviction release the cache.
Message text is not duplicated. Two focused history tests passed, comparing the
cached result against the ring through mixed arrival/dismiss/read/eviction cycles.
No native frame-time improvement is claimed without measurement.

Added a standalone optimized palette-search benchmark using the real matcher.
On Apple M4 Max, the synthetic edit sequence measured median 2.152 vs 4.190 µs
at 128 candidates, 14.417 vs 25.758 µs at 1,024, and 106.073 vs 190.640 µs at
8,192 for incremental versus full rescans. Three alternating-order samples
validated identical ranked results; the harness runs through
`mise run bench-palette-search`. [Method and raw samples](benchmarks/palette-search.md).
These are computation timings, not native input/render latency or memory gates.

Palette filtering now keeps an immutable search snapshot and reusable ranking
buffer for the palette's lifetime. Appending to the normalized query rescans only
previous matches; deletion/replacement rescans all candidates, and an empty query
restores source order without sorting. Five focused matcher tests passed, including
a differential full-ranking comparison across typing, backspace, whitespace,
Unicode and empty-result transitions. Buffers are released with the palette.
Native typing latency and allocation measurements remain pending; no quantified
performance gain is claimed from the structural reduction alone.

Palette virtual-list sizing now uses explicit 36px title rows and 56px subtitle
rows with 20px/16px line heights. The results area sums only enough rows to reach
its 320px cap, avoiding first-frame height inference from a partially measured
list and preserving short-result shrinkage. Both focused palette sizing and
navigation tests passed. Native clipping, small-window behavior and measured
render performance remain pending; this is not visual parity acceptance.

Command-palette results now use GPUI's variable-height virtual list rather than
building elements for every match on each render. Existing title/subtitle rows,
shortcuts, stable command click targets and empty state remain; filtering resets
the list and arrow/page navigation reveals the selected item. Visible-item
measurement uses 96px overdraw and the existing 320px result-height cap. Desktop
compilation and the focused keyboard-navigation test passed. Native access was
rechecked and remains blocked by the locked Mac; sizing/scroll/click acceptance
and measured render-time improvement remain unverified.

Added `rmuxctl dismiss-notification --surface ID --revision NUMBER`, with local
help and completion flags. Revision parsing preserves unsigned 64-bit precision
and rejects fractional, negative, overflowed, missing and duplicate values.
Eleven CLI tests passed. The new client command also passed the native socket
smoke against the existing app build, removing an older retained message while
preserving the newer one; repeated dismissal returns false via the shared RPC.

Font increase/decrease, reset size and Use Configured Font now have configurable
shortcut action names and route to the same terminal handlers as menus/palette.
The guide now correctly distinguishes resetting to the configured size (rather
than hard-coded 14), preserving a custom font family, and restoring both family
and size. Focused shortcut tests passed; native custom-binding acceptance remains
pending. Existing per-terminal inheritance and persistence behavior is unchanged.

Terminal, pane and workspace errors now share a compact banner with copy and
dismiss icon controls. Text scrolls within a 96px cap, controls retain 28px hit
areas, and banner pointer events are isolated from terminal input/selection.
Copy participates in the existing clipboard generation ordering; dismissal only
clears the owning view's error and does not retry an operation. Desktop compilation
passed. Native clipping, scrolling, tooltip and click acceptance remain pending.

The split-tree mutation itself now enforces the restore depth limit, closing the
remaining break-tab path around the creation guard. Break Tab checks before
constructing a destination or removing its live source tab and reports the same
limit message. Rejected tree mutations preserve all geometry; successful tab
breaks clear stale errors. Seven focused layout tests passed, including rejection
without mutation and existing swap/remove/resize behavior. Native break-tab limit
interaction remains unverified.

Interactive/RPC split creation now checks the target's depth before starting a
shell, sharing the 32-level constant with saved-layout validation. This prevents
creation through these paths of a split tree that the restore validator rejects.
Shallow sibling branches remain splittable even when another branch is at the
limit. Successful creation clears stale startup errors. Focused tests passed for
target-depth decisions and actual save/decode at depth 32 with rejection at 33.
Other layout mutation paths and native limit-message acceptance remain to audit.

Added `rmuxctl split-pane --pane ID --direction right|down` with optional cwd,
focus and activation flags, command help and direction/boolean completions.
Required targets/directions and duplicate/unsupported options are rejected locally.
Ten CLI tests passed. The native socket smoke now creates its first split through
the friendly command and verifies nested RPC splitting, PID continuity, inherited
shell cwd and failed-start zoom preservation end to end.

Added `pane.split` RPC with explicit pane identity, right/down direction, optional
absolute cwd and opt-in focus/activation. Split construction now accepts a target
independent of keyboard focus, inherits its cached directory/font settings, and
changes zoom/drag state only after shell startup succeeds. RPC has a 64-pane limit
and existing busy-window guards. Native socket smoke passed with nested targeted
splits, original shell PID continuity, unchanged workspace selection, actual shell
cwd inheritance, and failed-start preservation of zoom/pane count. This path still
starts PTYs synchronously like existing workspace creation; background startup and
native visual/keyboard split acceptance remain open.

Added friendly CLI forms for live layout operations: `move-panel`,
`move-workspace`, and explicit `set-pane-zoom`. They map to existing RPC transfer
and zoom handlers, require source/destination IDs, reject missing/duplicate
arguments, preserve boolean JSON types, and keep OS activation opt-in. Offline
help and shell completions include all three. Nine CLI tests passed, including
exact request mappings and malformed destination/zoom cases. Shared transfer
paths have prior live checks; this iteration did not rerun native acceptance.

Added offline per-command help via `rmuxctl help COMMAND` and `COMMAND --help/-h`,
including a leading socket option without opening it. `--version` reports the
client build version. Help describes supported arguments, explicit target IDs,
focus/activation behavior and exit codes; completion suggests the new help forms.
Eight CLI tests passed, including literal `--help` payload preservation and unknown
help topics. Raw RPC schema help and full upstream command coverage remain open.

Added offline `rmuxctl completions bash|zsh|fish` generation and a shared local
candidate generator for friendly commands, supported flags and boolean values.
Completion respects leading socket paths (including spaces), option values,
literal `--`, used options and conflicting read-screen scopes; it never connects
to the app. Seven CLI tests passed, and installed Bash/Zsh syntax plus wrapper
argument handling passed with no socket environment. Invalid shells exit 2.
Fish scripts generate but runtime verification is pending because Fish is absent;
no tools were installed. Live-ID/path/JSON completion and full CLI parity remain open.

The tab picker now searches displayed tab numbers as exact tokens, fixing a gap
between the documented behavior and the title/directory-only implementation.
A numeric query prioritizes the matching tab number while preserving ordinary
numeric title/path matches; mixed number/metadata queries are supported. Four
focused matcher tests passed, including tab 3 versus 13, leading-zero rejection,
metadata combinations and numeric title fallback. Native interaction remains pending.

Tab-picker selection now retains a terminal entity ID instead of a ranked row
number. Live title/directory changes can reorder results without changing the
Enter target; drawing, navigation and submission resolve the same identity, with
a first-result fallback if it disappears. Query edits deliberately reset to the
first result. Desktop compilation passed. Native live-output/picker interaction
remains pending; this does not establish complete keyboard acceptance.

Tab-picker search now uses ranked fuzzy words across titles and cached directories,
sharing the palette matcher. Each word stays within one field; contiguous matches
beat subsequences, title matches win equal-quality directory matches, and empty
queries preserve tab order. Three focused matching tests passed, including mixed
fields, Unicode and forbidden cross-field matches. Moving a pane between windows
also clears the picker and its old-window blur subscription through the existing
editor reset path. Native search and cross-window popup acceptance remain pending.

Custom shortcuts now expose the existing Last Terminal/Workspace, pane cycling,
tab/workspace reorder, previous/next/last prompt, top/bottom scroll and find-match
navigation actions. They dispatch the same typed actions as menus/palette and
retain the default terminal-only scope. User documentation includes focus-history
and prompt-jump examples. Focused shortcut compilation/keymap tests passed; native
shortcut interaction and complete upstream binding coverage remain pending.

Paste encoding now allocates one output buffer instead of separate mutable-input
and output buffers. Ghostty's documented null-output size query sanitizes the
payload in place; rmux adds bracketed delimiters in the same allocation and checks
the reported length. This removes a payload-sized temporary allocation and copy
(up to 1 MiB per accepted interactive paste), without duplicating sanitization
rules. Six focused paste tests passed, including differential comparison with
Ghostty's regular output API for every ASCII byte, Unicode, empty input, newlines
and embedded delimiters in both modes, plus existing live PTY ordering checks.
No end-to-end latency improvement is claimed without measurement.

Paste captures bracketed-paste mode under the terminal mutex, then allocates and
encodes its bounded payload after releasing that mutex. Both interactive and
nonblocking automation paths use the same standalone Ghostty encoder; the single
writer and paste permit still preserve complete-payload ordering and backpressure.
Five focused paste tests passed, covering 1 MiB boundaries, delimiter sanitization,
real PTY ordering, automation viewport preservation, and permit recovery. The
interactive mode lookup still waits for parser access and encoding still occurs
on the caller thread; this change does not establish a UI latency bound.

Find now uses the shared monochrome chevron/close icons and neutral chrome with a
thin separator. Empty queries retain “Find in terminal”; completed zero-result
searches show “No matches” instead of an ambiguous 0 / 0. In-flight searches retain
their searching indicator, and status text does not shrink beneath the controls.
Desktop compilation passed; rendered narrow-pane and keyboard acceptance is pending.

Notification history now has All/Unread controls with live counts and a panel-local
U shortcut. Mouse and keyboard navigation share the filtered records; switching
filters preserves the selected message when visible and scrolls it into view.
Clear all still clears window history even when the Unread filter is empty.
Desktop compilation passed; native visual/keyboard acceptance remains pending.

Added bounded notification history end-to-end: the engine retains 32 notifications
between UI observations, and each live terminal retains 32 messages with independent
read state. Mixed OSC 9/99/777 bursts no longer collapse to the latest message.
The panel lists newest first per terminal with unique terminal/revision row IDs and
keyboard selection; focus/Mark All Read acknowledges history, targeted dismiss
removes only one message, and Dismiss All batches clearing once per terminal while
preserving bell/status alerts. RPC lists history; `surface.list` retains its latest
message field and avoids formatting historical text. Engine unchanged-revision
reads avoid scanning the ring. History/burst tests and an isolated native socket
smoke passed, including old-message dismissal and mixed OSC bursts. Native panel
appearance/keyboard acceptance remains pending; history ends with terminal lifetime.

Completed streaming OSC 99 Base64 text assembly: encoded groups and multibyte
Unicode can span chunks, title/body maintain independent state, and independently
padded chunks still concatenate. A final unpadded group is flushed at message
completion; malformed or incomplete final text is discarded. Only up to three
encoded characters and three undecoded UTF-8 bytes persist per text field, besides
the bounded title/body. Plain text avoids a payload-sized scratch allocation.
Four focused tests passed, including every split of multilingual encoded text,
one-character chunks, interleaved title/body completion, padding, and malformed
endings. This finishes the previously documented encoding-group gap; notification
history, lifecycle actions and native UI acceptance remain outstanding.

OSC 99 capability discovery now replies to empty `p=?` queries with supported
text payloads and visibility policies, preserving safe identifiers and BEL/ST
termination. Unsupported action/close/sound/expiry features are not advertised.
Replies use the existing ordered PTY response path and are capped at 64 KiB per
drain; complete responses are omitted when capacity is exhausted. Response payloads
are never answered, preventing feedback loops. Identifier validation now follows
the protocol alphabet, and complete unpadded Base64 text chunks are accepted.
Focused query framing/order/bounds and text assembly tests passed. Cross-chunk
Base64 encoding groups and remaining notification lifecycle features are pending.

OSC 99 now retains `o=always|unfocused|invisible` across notification chunks and
applies the policy before storing messages or marking unread. Invisible uses the
active OS window, selected workspace/tab and pane zoom structure without acquiring
parser locks; unfocused uses the existing focus state. Suppression advances the
engine revision, preventing stale alerts on later focus changes. Legacy OSC/RPC
notifications retain their existing always-accept behavior. Three focused protocol
and truth-table tests passed and the desktop compiled. Native activation, hidden
pane/tab and multi-window interaction acceptance remain pending.

Added a conservative OSC 99 prefix fast path after the boundary benchmark exposed
OSC-heavy overhead. Chunks without `9`, without a potentially split introducer,
and without an active capture use Ghostty's original bulk writer. Differential
framing tests compare every split of mixed control streams with the boundary-only
path. In the same one/eight-pane benchmark, the no-digit-9 shell-metadata fixture
moved from 25–28% slower to within roughly 1% of the prior raw path; ANSI/Unicode
was within 1%, plain text 2–4% slower, and engine-resident bytes matched. Chunks
containing `9` conservatively retain the boundary path, so remaining overhead is
workload-dependent. See [measurements](benchmarks/notification-boundaries.md).
Validation: 77 terminal tests passed in the combined run; one existing shell
resize test hit a transient `openpty` allocation error and passed on its focused
retry. The manual throughput benchmark is ignored during ordinary test runs.

Measured OSC 99 stream-boundary overhead against the prior raw writer in one
release executable, alternating three samples at one/eight panes. ANSI/Unicode
throughput stayed within 1% in the original comparison; plain text varied by
1–5%, and synthetic two-OSC-per-line output was 25–28% slower. Engine-resident
memory matched each pair. Two speculative fast paths failed to improve results
consistently and were removed. The reproducible ignored benchmark and raw samples
are in [the measurement report](benchmarks/notification-boundaries.md). This leaves
OSC-heavy throughput as an explicit performance gap; GUI latency is not measured.

Added bounded OSC 99 text notifications (title/body, interleaved chunk IDs and
Base64) using Ghostty's stream-boundary API. Ghostty still owns UTF-8/control-string
framing and cancellation; non-99 OSC payloads are rejected before copying. Limits
are 8 KiB per captured sequence, 16 pending IDs, 256 title bytes and 1024 body bytes;
reset clears pending chunks. Completed messages use the existing notification
revision/unread path alongside OSC 9/777. All 77 terminal tests passed after stream
integration, including byte-split/cancelled strings and legacy coexistence. Desktop
compilation passed. Native UI and throughput acceptance remain pending. Protocol
queries, actions/close reports, post-delivery ID replacement, visibility filtering,
expiry and graphical payloads remain incomplete. Protocol reference:
https://sw.kovidgoyal.net/kitty/desktop-notifications/

Terminal selection copying now formats on the background executor instead of
blocking UI dispatch on the parser and formatter. The existing 16 MiB limit and
no-selection behavior are preserved. One pending copy per terminal coalesces
repeated requests; selection gestures, Select All and Clear Scrollback cancel
pending delivery. A shared generation guard orders results against newer terminal,
text-field, directory, memory-report and notice copies within rmux. Clipboard
export retains no engine until execution. Focused engine export and generation
tests passed, as did desktop compilation. Native clipboard interaction under
output pressure remains pending; other applications' clipboard writes are not
tracked by the internal generation guard.

Removed the blocking parser acquisition after queued text input. Follow-output
scrolling now tries once and otherwise coalesces into one atomic flag, consumed
under the next frame lock. Explicit viewport navigation cancels the deferred
follow; automation input continues preserving scrollback. No thread, timer or
unbounded queue was added. A real-shell regression deliberately holds the parser
lock and verifies input returns promptly, deferred follow reaches bottom, and
later explicit navigation wins. The automation bracketed-paste/viewport test also
passes. Key encoding, mouse handling, focus and other synchronous engine operations
remain performance work; this is not a claim of complete input-latency acceptance.

Settings now exposes new group workspace placement (After current / First / Last)
with keyboard cycling and saved preferences. Group directory capture/reset actions
also appear in the File menu; group creation, selection, rename and directory
actions can be bound to custom shortcuts. Five focused settings tests passed,
covering cycling, navigation and persistence. Native visual acceptance remains
pending: the desktop was checked again and is locked, preventing screenshots.

Workspace groups now retain an optional starting directory independently of anchor
shell navigation. Palette actions capture the focused terminal directory or reset
to following the anchor. Group create/update RPC accepts an absolute cwd, update
supports clear_cwd, and snapshots/layouts/sessions preserve the override. New group
members prefer the override while existing terminals are untouched. Fifteen focused
group tests passed, including mapping/restore and invalid/conflicting RPC input.
Native macOS spawned-shell acceptance now passes via `mise run smoke-group-directory`:
actual shell startup directories remain fixed after anchor navigation, follow an
updated override, then follow the anchor after reset. The anchor retains its PID
and directory throughout. The isolated harness owns its app, shells and temporary
files and checks startup paths independently of UI metadata. Native palette
interaction and other-platform acceptance remain pending.

Added close confirmation to Settings with Active tasks / Always / Never choices,
forward/backward keyboard cycling and immediate application across close scopes.
Saving persists the choice while preserving unrelated config. An environment
policy override stays authoritative and is not written into the saved preference.
Settings navigation now derives its control count from the settings list. Focused
settings tests pass; native panel interaction and appearance remain pending.

Git branch badges now subscribe to native HEAD changes, including atomic checkout
replacement. One watcher serves the application with shared repository watches,
capacity-one event channels and background discovery; idle terminals add no polling
or Git subprocesses. Dropping a terminal queues unsubscription without blocking the
UI. Shell reports and manual refresh remain available when native events are
unavailable. Three focused tests passed, including real native event delivery to
multiple subscribers and continued delivery after one unsubscribes. Native sidebar
visual acceptance remains pending; these changes are for the next release.

Added Mark All Read in This Window in the notification panel, View menu, palette,
and custom shortcut actions. It acknowledges all alert sources across hidden
and visible tabs in this window without focusing terminals, removing messages,
or touching parser locks. Focus acknowledgement shares the same helper. Other
windows remain untouched. The desktop compiled and the existing attention-source
transition test passed. Native interaction acceptance remains pending.

Follow-up after v0.1.0: moved Settings into the application menu and added native
macOS About, Hide, Hide Others and Show All commands. Cmd-H and Cmd-Option-H use
GPUI's platform operations; defaults are registered before custom shortcuts so
reload retains the usual override behavior. About uses AppKit's standard panel
and bundled version metadata. The focused desktop check passed; native panel/menu
interaction remains unverified. The published v0.1.0 archive remains unchanged.
Mouse-focus investigation found an existing focus-first handler; the earlier
capture alone does not establish a defect, so no speculative mouse change was made.

Prepared the first public macOS preview with a user-facing README and real app
screenshot. Added Sparkle's native updater/controller and Check for Updates menu,
public feed/key metadata, separate distribution bundle identity, Developer ID
signing, notarization/stapling and signed update archives. Local packaging passed
Apple notarization and Gatekeeper, and the native updater menu appeared. Published
binaries will be built on GitHub-hosted ARM64 runners through the new release
workflow; its signing credentials are encrypted Actions secrets and imported into
a temporary keychain for Apple signing and a temporary file for Sparkle signing. Release v0.1.0 is now published from GitHub Actions run 44755158627. Its public
ZIP passed checksum, pinned-public-key Sparkle signature, strict code signature,
stapled-ticket and Gatekeeper verification. The raw-main update feed serves that
exact archive. Final native update-dialog confirmation was blocked when the Mac
locked; no end-to-end version upgrade has yet been exercised.

Fixed invisible toolbar SVGs: GPUI requires explicit SVG text color instead of
inherited parent text color. Real-window capture now shows sidebar, notification,
add, close and split controls. Full visual/interaction parity remains incomplete.

Separated notification, bell and task-status unread sources. Dismissing the
current message clears only its notification alert; a bell received while a
message was already unread is retained. Focusing a terminal acknowledges all
sources. Existing tab/sidebar/RPC attention values remain booleans. A focused
state-transition test covers every bell/status combination, both arrival orders,
repeat dismissal and a new notification after focus acknowledgement. The focused
desktop test passed (1 passed); native notification-panel interaction remains
to be exercised with the updated running build.

Actual app screenshots are now available again: the earlier desktop-lock blocker
has cleared. Captures showed the older debug process and newer release process;
neither should be treated as proof that all latest source changes are running.

Tightened shortcut validation around GPUI's permissive key parser. Unknown named
keys and noncanonical function-key spellings now fail instead of creating inert
bindings. Canonical duplicates (including Ctrl-X versus Ctrl-Shift-x) and
single-stroke/chord prefix conflicts within the same custom scope report clear
errors; distinct two-stroke chords sharing a prefix remain valid. Focused tests
passed both insertion orders, aliases, valid shared prefixes and existing real
keymap precedence/disable behavior. Default-versus-custom conflict explanations
and graphical editing remain future work.


Added validated configuration keybindings for common actions and custom commands,
with terminal/default or global window scope and one/two-stroke sequences. Initial
built-in bindings are retained once; reload clears the keymap and reinstalls only
defaults plus current overrides, so removed bindings recover and reloads do not
accumulate handlers. Duplicate keys in a context, unknown targets and plain-text
strokes fail validation. GPUI keymap tests passed override precedence, editor
context isolation, disabled bindings, restoration and invalid/chord definitions.
Desktop build passed. Native key dispatch, full conflict UI and complete shortcut
contract coverage remain pending.


Added notification-panel focus ownership and keyboard navigation using stable
terminal IDs. Arrow/page/Tab movement scrolls the selected message into view;
Enter/Space opens its terminal and closes the panel, Delete/Backspace dismisses,
and Escape restores prior focus. Settings and notifications now share consistent
mutually exclusive open/close behavior. Cached message collection requires no
parser access; card styling uses neutral dark backgrounds with a selected-row
highlight. Desktop build passed. Native event routing, visual quality and
screen-reader acceptance remain pending; desktop capture was rechecked and is
still blocked by the locked Mac.


Refreshed ARM64 Linux checks after recent desktop/CLI/session changes. Desktop and
CLI cross-compilation passed. Sixteen PTY/session runtime tests passed in 1.16 s
inside the existing local Linux image with networking disabled, no pull/install,
read-only code/root and temporary writable /tmp. Coverage includes new nonblocking
metadata/resize paths, paste ordering/viewport preservation, direct-command env,
retention, process cleanup and interactive resize. Saved the exact binary hash
and output in linux-arm64-session-tests-2026-11-20.txt. Linux GUI/IME/presentation
and x86_64 execution remain pending; Windows validation remains deferred.


Extended custom commands with bounded per-child env overrides (64 portable names,
16 KiB total, no NULs; case-insensitive duplicate rejection on Windows). Existing
command entry points retain their default environment. A per-window pending flag
prevents concurrent custom launch bursts and clears on completion/failure. Desktop
build, configuration validation and actual PTY environment fixture passed; Unicode,
empty values and literal shell-looking content remain intact without mutating the
parent environment. Native repeat-activation interaction remains unverified.


Added bounded custom command definitions (name/argv/optional absolute cwd) to the
configuration model and searchable Run entries in the palette. Commands start a
new ungrouped workspace using a prepared PTY session; process startup is off the
UI thread and arguments are passed literally. Cached selected-terminal cwd is
used when no override is supplied. Busy/closing/limit state is rechecked before
inserting the prepared session; unavailable destinations drop the owned session.
Palette argument previews are sanitized and bounded. Desktop build passed;
configuration validation and direct-command PTY tests cover the shared launch
contract. Native palette activation and full upstream command/shortcut/environment
parity remain pending.


Made font defaults observable live: terminal preference observers track the prior
size/family defaults and apply changes independently only where current values
still match them. Custom zoom/family values remain; Reset Font Size and Use
Configured Font can rejoin defaults. The existing font invalidation path updates
cell measurement/cache behavior and persistent metadata without replacing shells.
Updated Settings hints, reload notice and documentation. Focused test passed all
four inherited/custom combinations. Matching is value-based, not an explicit
inheritance flag; native rendering and user interaction acceptance remain pending.


Removed blocking parser acquisition from output metadata handling. A single
try-lock reads title/directory/notification/progress/status summary, optional
status details and render-delay state. The existing per-terminal async update
task retains pending work and retries after 8 ms only on contention, preserving
revision cursors and stopping after success. Synchronized-output timer callbacks
also use nonblocking delay queries. Added metadata_busy diagnostics. Extended
contention regression passed immediate busy results and eventual title delivery;
native socket fixture passed OSC/RPC notification, input, creation and cleanup.
Metadata copies still execute on the UI thread after acquisition; this change
removes waiting, not all engine work or the need for presentation measurements.


Prepaint now uses try_resize before snapshot acquisition. A busy parser leaves
both PTY/engine dimensions untouched and schedules a display-frame retry using
the current layout rather than queueing obsolete drag sizes. Unchanged sizes
avoid engine locking, and Find refresh occurs only after an applied resize.
Added opt-in resize_busy counters. Focused Unix PTY test passed busy/no-op behavior
and latest-size consistency across PTY and engine; desktop build passed. Actual
reflow and PTY resize still run synchronously after access, so this does not prove
resize latency or complete removal of UI-thread engine work. Native drag/output
stress and presentation measurements remain pending.


Refreshed the release PTY pressure baseline: 768 requests across 1/4/8/16 panes
and two additional 16-pane runs completed. Eight-pane response-snapshot p99 was
1.331 ms; sixteen-pane p99 ranged 4.249–5.662 ms, closely matching snapshot-batch
tails. Raw results and limitations are recorded in pty-baseline.md. Inspection
found blocking prepaint snapshot acquisition; switched it to try_frame with cached
frame reuse and next-display-frame retry on contention, plus opt-in snapshot_busy
counter. Desktop build and existing parser-contention regression passed. This
change has not yet been measured in GUI presentation; resize/metadata/key locks
and end-to-end latency acceptance remain open.


Added workspace.close and close-workspace --workspace ID. The operation uses
existing busy/confirmation policy checks and the normal owned-terminal cleanup
path; confirmation-required calls return an error without prompting or mutating.
Fixed shared background-workspace close behavior to retain active focus instead
of reselecting/acknowledging its terminal. Native socket smoke passed background
selection preservation, active-workspace fallback, sole-workspace window removal
and owned fixture-shell reaping. RPC regression tests passed. Forced close and
full cmux close contract remain pending.


Added surface.send_key and send-key CLI with named keys, modifier prefixes,
common aliases and single Unicode characters. Encoding follows current Ghostty
modes and bundles press/release into one writer message, preserving viewport and
focus without waiting on parser access. Focused engine checks passed ASCII/Unicode,
Ctrl-C, application arrows and exact Kitty press/release bytes. Six CLI tests and
native Enter-to-fixture-shell smoke passed. Held keys, sequences and --force are
not yet supported; native Windows validation remains deferred.


Added send --paste and surface.send_text paste:true using a bounded nonblocking
session paste path. Ghostty encodes current bracketed-paste behavior; the writer
queues one complete payload with the existing single-paste permit. No viewport
scroll or focus change. Parser contention returns a retryable error before any
input is queued; automation remains capped at 16 KiB. Focused PTY test passed
exact Unicode/multiline framing, ordering, busy-permit release and viewport
preservation. Native CLI-to-shell smoke also passed paste delivery, and replaced
a timing-based notification assertion with waiting for actual shell output.


Refreshed the optimized macOS app bundle including rmuxctl, notification and
workspace automation, and the Settings panel. Focused Settings navigation/bounds
test passed after resolving a test-macro import collision. The socket integration
fixture passed against both actual release-bundle executables, covering clients,
filters, workspace creation/failure rollback, RPC/OSC notification coexistence and
normal quit cleanup. Existing user app processes were not terminated; an already
running old instance does not update until relaunched. Pixel/interaction review
remains pending and is not implied by this release smoke.


Added Settings keyboard focus ownership, remembered return focus, seven-control
Tab/Shift-Tab/up/down navigation, left/right adjustment, Enter/Space activation
and Escape close. Selected rows/buttons show a highlight and navigation scrolls
them into view. Plain typing is contained in the panel; switching to notifications
closes Settings and restores focus. Removed duplicate open-state storage in favor
of the optional focus/navigation state. Desktop build passed; focused navigation
and value-bound checks cover the pure behavior. Native event routing, focus
restoration and screen-reader acceptance still require an unlocked desktop.


Added an in-app Settings side panel through Cmd+, (Ctrl+, elsewhere), app menu
and palette. Controls cover default text size, padding, terminal theme, unfocused
cursor and copy-on-select, with session changes and explicit Save. Saving merges
only these fields into the latest validated config, preserving other options and
following existing symlinks; invalid edited config is left untouched. Writes use
an atomic temporary-file replacement on a background worker. Font default
semantics are stated in the UI; custom color overrides remain effective. Desktop
build passed; native appearance, keyboard navigation and full settings parity
remain pending on visual/interaction review.


Added notification.send/list/dismiss plus notify/list-notifications CLI commands.
Messages share existing in-app panel/unread state and never focus a terminal.
Latest-per-surface storage stays bounded; dismissal is revision-checked. Separated
shell notification cursor from UI message revision so RPC messages cannot cause
old OSC metadata to replay over new content. Native fixture-shell socket smoke
passed send/replace/list/stale-dismiss, OSC after RPC, and RPC remaining current
after ordinary shell output. Native OS delivery, reply actions and full cmux
clear/history behavior remain pending; screenshot acceptance is still blocked by
the locked desktop.


Added workspace.create and CLI new-workspace/new-window commands. Workspace
creation reuses the normal shell/workspace path with a new no-focus mode, accepts
name/description/absolute cwd, and defaults to preserving the selected workspace.
Explicit focus/activate selects the new workspace; failed shell startup discards
only the empty entry and restores numbering. Busy-state and 64-workspace guards
apply. Native socket smoke passed background/focused creation and failed-cwd
rollback with unchanged discovery state. OS activation still needs manual
acceptance. Command/env/layout overrides and full cmux creation semantics remain.


Added window filtering to workspace discovery and combined window/workspace
filtering to pane/surface discovery, exposed through CLI --window/--workspace.
Filters reject malformed IDs/unknown fields and intersect without changing focus;
valid missing/mismatched targets return empty lists. Unrelated windows and
workspace children are skipped before snapshot construction. CLI tests and native
two-window socket smoke passed, including combined filters, mismatched ownership
and malformed-ID errors. Caller permissions and ref/index resolution remain open.


Added nine common CLI commands based on the checked-in cmux command inventory:
window/workspace/pane/panel lists, workspace select/rename, panel focus, text send,
and screen/history/selection reads. Explicit opaque targets, literal text after
--, strict duplicate/conflicting/unsupported option rejection and optional OS
activation preserve predictable behavior. Raw RPC remains available. Four CLI
tests passed; native socket smoke now renames and lists via command syntax.
Full cmux targeting/output compatibility and remaining options are still pending.


The macOS bundle now builds and carries rmuxctl alongside rmux, with an explicit
ad-hoc signature for the helper before sealing the app. No global install or
PATH change. The socket smoke accepts explicit app/client paths; the new
`mise run smoke-bundle-socket` exercised both copied debug-bundle executables
successfully, including discovery/error exit codes, shared mutations, reconnect
and endpoint cleanup. Trying-rmux now documents the packaged client and reflects
implemented socket/CLI support rather than listing them as entirely missing.


Added the standalone Rust `rmuxctl` client without GPUI/Ghostty dependencies.
It connects to explicit Unix socket paths or RMUX_SOCKET_DIR, validates JSON
object parameters, bounds request/reply allocation, preserves RPC replies and
returns distinct success/RPC/transport exit codes. No mutation retries. Two
focused tests passed, plus native socket smoke using the actual client for
capability discovery and unknown-method error handling. cmux command aliases,
ref/index targeting, caller scope and Windows transport remain pending.


Added opt-in Unix socket RPC using the existing bounded live dispatcher. Private
endpoint creation refuses existing paths; identity-checked cleanup preserves
replacements. Eight-client admission and read/write timeouts bound transport
resources. A focused endpoint unit test and native two-client smoke passed,
including shared Unicode rename, reconnect and normal-quit cleanup. Windows
transport, caller scope, subscriptions and cmux compatibility remain pending.


User requested progress screenshots from the main app rather than a preview
workspace. Switched the visual review workflow accordingly. Found an older debug
bundle still running, so refreshing the release bundle alone could not update
that visible process. Preserve existing terminal processes and open the current
release separately for review. Native screenshot capture still depends on the
Mac being unlocked; synthetic fixtures are not visual acceptance evidence.

Integrated the macOS native traffic-light controls with the 36px app header,
removing the separate stacked titlebar across new, detached and restored windows.
Reserved control space in the sidebar header or workspace header when the sidebar
is hidden. Title labels support double-click zoom and native AppKit drag; GPUI's
generic drag method is a no-op on macOS, so a scoped borrowed-window bridge uses
the current mouse event. Existing pinned objc/raw-window-handle dependencies are
reused; no tools were installed. Native session-restore smoke passed after the
titlebar change and compilation passed for the AppKit bridge. Actual traffic-light
placement, fullscreen behavior and mouse dragging still require visual/native
interaction acceptance. Linux/Windows retain their normal titlebar configuration.

Lightened the workspace frame: split separators now draw a centered 1px line
inside the existing 5px drag target, preserving resize geometry and keeping the
hover affordance. Removed the duplicate bottom New Workspace control; the header
add button retains its platform shortcut tooltip, menu and keyboard routes.
Terminal padding already defaults to 8px and was left configurable. Desktop
compilation passed via mise. Rendered divider alignment, hover and drag acceptance
remain pending.

Added an isolated native visual-preview harness and `mise run preview-ui` task.
The generated scene combines grouped workspaces, nested splits, long tab names,
synthetic branch metadata and unread notification content; overflow/compact
variants exercise crowded tabs and smaller windows. It requests fixed logical
geometry and uses temporary configuration rather than user recovery/session
files. Headless layout validation passed; native visual execution and screenshot
comparison remain pending. This supplies a repeatable acceptance scene, not
evidence that the current layout is polished.

Refined sidebar context layout: branch and directory now truncate independently,
with branch width capped at 45% of the row and the full values in a tooltip.
Added consistent branch/pin SVGs and moved pin status from an extra text line
into the workspace title row, reducing vertical noise. Context remains cached
and non-interactive so it does not steal workspace clicks. Desktop compilation
passed via mise; the layout still needs rendered narrow-sidebar acceptance.

Completed the prompt-driven Git refresh path with an explicit palette refresh
action and cached `git_branch` in surface discovery. The focused engine test
proves repeated identical OSC 7 reports remain observable. Extended native session
smoke changes a fixture HEAD to a Unicode branch without changing directory,
sends another prompt directory report and waits for the background cache to
update. Both checks passed through mise, including the existing recovery and
cleanup scenarios. Native visual placement and manual action invocation remain
pending; no file watcher or idle timer was added.

Started T09 sidebar Git context: branch and detached-HEAD labels appear beside
the active/remembered terminal's directory. Discovery runs off the UI thread on
directory reports, reads at most 4096 bytes per metadata file and traverses at
most 64 ancestor directories. `.git` indirection supports linked worktrees.
One request per terminal runs at a time with one coalesced follow-up; stale-path
results are discarded. No Git subprocess or polling timer is added. Focused
tests passed for nested repositories, Unicode branch names, worktree indirection,
detached HEAD and oversized metadata. PR/port information, refresh outside shell
directory reports and native visual acceptance remain pending.

Added the reference sidebar's notification context: clear blue/white unread
badges and two-line message previews, including hidden group members. Previews
use cached unread, non-dismissed messages in pane/member order and are gathered
only while the sidebar is visible. They neither query terminal engines nor
acknowledge notifications. Adjusted selected group-summary contrast. Desktop
compilation passed via mise; native screenshot access was rechecked and remains
locked. Rendered clipping/spacing and visual acceptance are still pending.

Continued the screenshot-led visual pass with a coherent embedded SVG icon set
for add, close, disclosure, copy, refresh, split, sidebar and notification controls.
Replaced inconsistent font glyphs and CSS-drawn split icons, keeping established
hit areas and tooltips. The workspace header now has direct sidebar/notification
controls and a compact semibold title. Icons are built into the executable via
GPUI's asset source; no runtime disk reads or icon fonts are needed. Native
screenshot comparison remains blocked by the locked Mac; no visual parity is
claimed from compilation alone.

Prioritized the user's explicit cmux UI/UX parity request over socket work.
Inspected the official landing screenshot and recorded the visual hierarchy in
`docs/reference/visual-parity.md`. Applied a coordinated first pass to neutral
chrome, selected workspace contrast, typography, tab height, sidebar separator
and header add control. Native rendered comparison is still required; a locked
Mac prevents claiming visual acceptance. The intended follow-up is screenshot-led
layout/interaction refinement, not another unrelated diagnostics feature.

Produced a refreshed optimized macOS app at `target/release/rmux.app` with all
current workspace/UI work. The local bundle's strict signature verification and
live-shell RPC smoke against its actual executable passed, covering identity,
Unicode I/O, cross-window transfer/state retention, zoom, bounded reads and owned
process cleanup. Executable size is 9.1 MiB (not runtime RSS); SHA-256:
`e235bc9339f5406cd820248830802e5ab60cbb72671626a06437b0ec69dccdcb`.
Added `mise run open-app`, `mise run smoke-release` and a concise trying-rmux guide
separating implemented capabilities from acceptance gaps and missing P2 work.
No visual, OS activation, latency or full phase acceptance is claimed. Next work
should deliver coherent daily-workspace/automation batches rather than additional
diagnostic polish.

Added notification count and Clear all to the notification-panel header, plus a
Dismiss All Notifications palette action. The operation snapshots each current
message revision and uses existing per-terminal guarded dismissal to clear its
unread marker without focusing terminals or suppressing future revisions. It
walks terminals once and retains entity handles for updates, avoiding repeated
workspace searches. Desktop compilation passed through mise; native dismissal,
new-message and layout acceptance remain pending.

Aligned Next Unread Terminal and notification-list ordering with logical sidebar
order, including pinned groups and collapsed members. Unread navigation checks
the current workspace first and wraps once, then uses the shared direct terminal
focus path for zoom, remembered pane and group visibility without acknowledging
an unrelated previously active tab. Removed the duplicate focus implementation.
Desktop compilation passed through mise; native unread ordering/acknowledgement
acceptance remains pending.

Fixed tab transfer selection to reveal collapsed destination groups for both
same-window and cross-window moves. Move Tab to Next Workspace now follows the
logical sidebar ordering (including group members) instead of backing-vector
order. Extended the native session fixture with a transfer into a collapsed
child workspace and back, checking expansion and retained process identity.
`mise run smoke-session-restore` passed. The new collapsed cross-window transfer
case and keyboard invocation of next-workspace movement still need native
acceptance; rendered focus remains unverified.

Fixed direct terminal navigation leaving its workspace hidden in a collapsed
group. Terminal focus now shares workspace selection's group-reveal behavior,
before scrolling the sidebar, while preserving an anchor's intentional collapsed
state. The extended native session fixture focuses a child workspace's terminal
and verifies group expansion, selected workspace and zoom following the target
pane. `mise run smoke-session-restore` passed, including existing restore,
recovery and owned-process cleanup scenarios. OS activation and rendered focus
remain unverified; the fixture proves application state through live RPC.

Added direct cross-window workspace switching to the command palette alongside
terminal switching. Window-labelled results resolve stable workspace IDs,
validate the destination's availability, restore its normal remembered-pane
focus and activate its window. Closed/moved/busy targets produce a notice before
changing destination selection. Desktop compilation passed through mise. Native
focus restoration and OS activation acceptance remain pending.

Added cross-window terminal switching to the command palette. Results include
other windows' workspace/pane/tab labels and cached directories; choosing one
uses stable window/workspace/terminal identities to focus the existing process
and activate its window. Destination availability and membership are checked
before focus changes, with a visible notice for closed/moved/busy targets.
Desktop compilation passed via mise. Native multi-window focus, OS activation
and stale-target interaction acceptance remain pending.

Extended directory-based terminal switching to the command palette across all
workspaces in the current window. Terminal results now have a muted directory
subtitle, full title/path tooltip and directory-inclusive fuzzy search. Other
commands retain single-line rows and shortcut badges. Paths are copied from the
existing cache when opening the palette; switching still uses stable workspace
and terminal identities. Desktop compilation passed through mise. Native layout
and interaction acceptance remain pending.

Added cached working-directory subtitles to the tab picker so identical shell
titles can be distinguished. Queries now match words across title and directory;
full paths remain available in tooltips when the subtitle is truncated. Rendering
and filtering use cached shell-reported/initial paths, with no parser lock,
filesystem access or polling. Desktop compilation passed via mise. The command
palette already had an explicit empty state and guarded no-result activation;
no changes were needed there. Native visual acceptance remains pending.

Added title search to the pane tab picker using the existing Unicode composition
text field. Case-insensitive query words filter titles while original tab numbers
and unread markers remain visible. Matching counts and an empty-result state are
shown; Enter on no results is a no-op. Arrow/page navigation shares the palette's
bounded selection helper and leaves active composition alone. Numbers now enter
the query, and Home/End edit it; direct pane-level tab shortcuts are unchanged.
Desktop compilation passed via mise. Native UI access was rechecked and the Mac
remains locked, so visual/IME/keyboard acceptance is still pending.

Improved the crowded-pane tab picker with visible tab numbers, a tab count and
keyboard hints. Plain 1–9 opens the corresponding tab through the existing
selection path; missing numbers are no-ops. Page Up/Down moves the highlight by
six rows with bounds clamping and scroll-into-view, preserving Enter-to-open and
Escape-to-cancel. Desktop compilation passed via mise. Native picker layout,
keyboard routing and visual comparison remain pending acceptance.

Optimized process sampling to refresh the requested PID set in one batch instead
of enumerating the host process table for each PID. Incomplete batches fall back
to individual memory probes and reset CPU baselines, preserving exited-process
handling without computing CPU twice over an invalid interval. The focused
sampler checks and benchmark passed; observed debug warm medians fell to
0.783/0.963/2.722 ms at 1/8/32 shells plus the app. The benchmark document records
both runs and limits; this is not an input-latency or release acceptance claim.

Measured process sampling with an opt-in Unix fixture and mise task at 1/8/32
blocked shells plus the application PID. Debug-build warm medians were
1.298/4.837/12.052 ms; five spaced samples per case returned finite nonnegative
CPU after unavailable first samples. Fixture processes are reaped even on panic.
See `docs/benchmarks/process-sampling.md` for scope and limits. This establishes
sampler cost, not release or input-latency acceptance; full-cap and descendant
sampling still need work.

Added descending screen-storage, shell-RSS and shell-CPU sorting to the memory
panel, available through an inline control and command palette. Missing samples
sort last, and copied JSON follows the displayed order. Sorting uses references
to at most 256 existing rows, without process resampling or copying terminal
data. The selected ordering survives refreshes, including an in-flight refresh.
Desktop compilation passed through mise; native interaction/visual acceptance
remains pending.

Added application-process RSS/CPU to the memory panel summary and copied reports.
The app PID is sampled alongside up to 256 shell PIDs on the existing manual
background refresh; the summary explicitly covers all windows. Closing clears
the displayed sample, and existing generation checks prevent late refreshes
from restoring it. Documentation distinguishes OS process RSS from GPU allocation
totals and warns against adding screen storage to RSS. Desktop compilation passed
via mise. Native visual acceptance and sampling performance remain unverified.

Added root-shell RSS and CPU to the manual terminal-memory panel and copied JSON
reports using the already pinned sysinfo dependency. Sampling stays on the
background executor and is bounded to 256 requested PIDs, with no polling timer.
PID-set changes reset the cache and CPU baseline; first/too-close refreshes leave
CPU unavailable. Each PID refresh must succeed before cached data is reported,
preventing exited shells from appearing live. Two focused desktop tests passed
through mise, covering positive RSS, unavailable first CPU, cache reset and a
sampled child that exits. Descendant/helper attribution, CPU accuracy/performance
measurements and native panel visual acceptance remain pending.

Improved the memory panel with descending resident-storage sorting (unavailable
samples last), click-to-focus rows using stable workspace/terminal IDs, and a
Copy JSON report control. Reports contain raw byte counts, scope, sample mode,
identity and availability rather than turning failed samples into zeros. Clicking
a stale row cannot target a replacement terminal. Sorting runs with sampling on
the background executor; report serialization occurs only on explicit Copy.
Desktop compilation passed via mise. Native row navigation, clipboard and visual
acceptance remain pending.

Added a terminal-memory panel through View and the command palette, with manual
refresh and close controls. It samples up to 256 terminals in the current window
on the background executor, showing workspace/tab names, PIDs, resident,
compressed, image and virtual storage plus explicit busy/closed states. The
summary counts successful samples rather than presenting busy entries as zero.
No polling timer was added, and generation checks discard results after close or
reopen. Desktop compilation passed via mise; native panel layout/interaction is
unverified while the Mac is locked. CPU/process RSS and richer task-manager
controls remain pending.

Started task-manager memory diagnostics with on-demand `surface.memory_usage`
RPC. UI lookup creates a weakly owned one-shot sample; Ghostty memory inspection
runs on the background executor and returns busy rather than waiting on parsing.
The response explicitly scopes primary/alternate resident, virtual, compressed
and image bytes to Ghostty screen storage, not application RSS or child memory.
Focused session contention test and extended native live-session smoke passed
through mise. No idle sampler or timer was added. UI, process CPU/RSS attribution
and refresh controls remain pending.

Verified selection export under parser contention and across the session/thread
boundary. The focused session test returns busy without waiting on a held parser
lock, confirms creating the queued exporter adds no strong engine owner, and
reads selected text on another thread without changing the selection. It passed
in 0.02s. Extended native live-session smoke also passed through mise: no-selection
RPC returns empty text with correct scope/truncation, alongside existing history,
viewport, identity, shell-state and cleanup checks. Native nonempty selections,
terminal-close timing and graphical screenshots remain separate acceptance work.

Started terminal selection-read parity: `surface.read_text` now accepts
`scope: "selection"`. Selection formatting uses the same bounded streaming writer
as history export, preserves UTF-8, caps output at the requested limit and runs
off the UI thread with a nonblocking engine-lock attempt. No selection returns
empty text with `truncated: false`; busy terminals retain the retry error. Four
focused export tests passed via mise, including selection/viewport preservation
and truncation in a multibyte character; desktop compilation passed. Native
selected-text RPC interaction and screenshot capture remain pending.

Added Close Workspace Group to the File menu and command palette. Multi-member
groups use one explicit confirmation naming the group and workspace count;
single-member groups use the existing workspace close policy. Confirmation
captures member IDs and closes only those still in the same group, with the
anchor last, through normal workspace/process cleanup. Cancel preserves the
group, and the dialog points to Ungroup for keeping terminals. Desktop
compilation passed via mise; native dialog/confirmation acceptance is pending
while the Mac remains locked.

Retried native UI access: the Mac is still locked and automatic unlock failed,
so visual review remains unavailable. Continued with group-create failure
rollback: when no shell/pane starts, discard the empty entry directly and restore
selection, previous-workspace history, title and numbering instead of running
normal close (which could focus a neighbor or expand its group). Extended native
smoke passed via mise: an invalid cwd with two existing workspaces returned an
error and left complete window/group/workspace discovery unchanged. Existing
creation, placement, directory, recovery and child cleanup checks still pass.

Added opt-in generated-anchor cleanup to ungroup RPC. Eligibility requires
explicit generated provenance and sole membership; user-selected/promoted
anchors and groups with children are rejected before mutation. Existing close
policy is honored, returning a confirmation-required error instead of bypassing
it. Native smoke passed for ordinary user-anchor rejection, memberful-group
rejection and cleanup of a disposable generated anchor while all eight other
terminals remained present; tracked children were reaped. A focused provenance
test passed via mise, including promotion losing cleanup eligibility. Native
confirmation behavior remains separate acceptance work.

Group-create RPC now accepts an absolute `cwd` for fresh anchors and rejects
combining it with existing workspace IDs. The explicit path flows directly to
shell construction without changing global startup directory state. Native smoke
passed via mise: a generated anchor and three members actually started in a
separate Unicode project directory, while original terminals remained in their
own directory. External-ID retry still starts only one anchor. Focused parser
checks reject relative cwd and empty/duplicate workspace lists before UI work.

Ungroup now materializes logical sidebar/member order before removing the group
container, preserving active workspace identity and individual workspace pins.
Both palette/File actions and RPC use the same path. Extended native smoke
passed via mise: a group created in reverse storage order was flattened in its
displayed order, selection stayed on the same workspace, and terminal IDs/PIDs
remained unchanged. Existing placement, cwd inheritance, restore and cleanup
checks also passed. Native visual appearance remains unverified.

Group headers now aggregate unread state across their workspaces, including
collapsed members, and show a compact workspace/unread count. The existing
attention dot lights for any unread member. Rendering reads each workspace's
cached attention state once and uses a set for group membership checks; it does
not acquire terminal parser locks or acknowledge notifications. Existing metadata
subscriptions drive redraws. Desktop compilation passed via mise; native visual
notification/collapse acceptance remains unverified.

Added sidebar multi-selection using Cmd-click on macOS / Ctrl-click elsewhere,
plus Shift-click ranges over visible rows. The header shows the selection count
and a shortcut hint. Group Selected Workspaces is available through File/menu
palette and Cmd-Shift-G (Ctrl-Alt-Shift-G elsewhere); it uses logical sidebar
order, creates no terminal, and promotes source-group anchors as needed. The
shortcut propagates when fewer than two workspaces are explicitly selected.
Closed workspaces are pruned from selection; plain clicks reset it. Desktop
compilation passed via mise; native modifier-click and shortcut interaction
remain unverified.

Added `workspace.group.new_workspace` RPC with explicit placement overrides and
configured fallback. It uses the group's anchor as the placement/cwd reference,
selects the new workspace, enforces the 64-workspace window limit and reports
shell-start failure after removing an empty failed workspace. Extended native
smoke passed for end/top/after-current order, fresh shell counts and actual
working-directory inheritance recorded by every fixture shell. The first cwd
assertion compared lexical paths and failed on macOS path aliases; resolving
both paths fixed the assertion, and rerunning the smoke passed including child
cleanup and recovery. Native visual interaction remains unverified.

Added the reloadable `group_new_workspace_placement` preference with
`after_current` (default), `top` and `end`. Cmd-N and header `+` now route through
one placement function; absent/stale after-current references fall back below
the anchor. Focused placement and configuration tests passed via mise, covering
all three modes, anchor preservation, backward defaults and unknown-value
rejection. Per-directory/per-group overrides and native placement UI acceptance
remain pending.

Added group-create RPC for explicit existing workspace lists and fresh generated
anchors. All supplied members are resolved before regrouping; duplicate/empty
lists are rejected. Optional external identity is persisted and window-scoped;
retries resolve it before spawning or modifying groups. Native smoke passed for
existing-member creation/retry, fresh-anchor creation/retry (exactly one new
terminal), and persisted external identity through automatic recovery. Existing
terminal IDs/PIDs survived regrouping and cleanup passed. Nine focused group
tests passed, including external identity mapping, uniqueness and validation.
Explicit cwd override, cleanup of generated anchors and native visual acceptance
remain pending.

Added live group add/remove/set-anchor RPC with same-window checks, membership
validation, busy guards and stable selection/anchor-slot handling. Removing the
last member returns a null group; adding an existing member is idempotent.
Extended native smoke passed: moved the second fixture workspace into the first
window, added it to the group, made it anchor, removed it and verified promotion
of the original anchor, then ungrouped. All four terminal IDs/PIDs remained
unchanged. Automatic restore/capture/forced-stop checks also pass. Focused
parameter rejection tests passed via mise. Native visual focus and group-create
RPC remain pending.

Added `workspace.group.update` (name/collapsed/pinned) and
`workspace.group.ungroup` to live JSON-RPC with strict parameter validation,
window-scoped target lookup, busy-state guards and notification routing. Ungroup
returns member IDs and keeps terminals alive. Native session smoke passed via
mise: updates reflected in discovery, ungroup removed membership, and terminal
IDs/PIDs were unchanged. Automatic group recovery checks still pass. A focused
parser test passed for malformed IDs, empty updates/names, invalid booleans and
unexpected ungroup options without queueing UI work. Creation and membership
RPC remain pending.

Exposed groups through live RPC discovery. `window.list` includes ordered group
records with window-scoped handles, names, style, pin/collapse/provenance,
anchor IDs and ordered member IDs. `workspace.list` reports nullable group IDs,
anchor flags and indices in logical sidebar order (including collapsed members).
Extended native session smoke passed through mise for explicit restore and
forced-stop relaunch: discovery matched live workspace IDs, Unicode group style
and persisted state. Group mutation RPC methods remain pending; visual UI
acceptance is still separate.

Unified sidebar ordering and keyboard navigation through a tested group-order
projection. Next/previous workspace, numbered workspace actions, sidebar shortcut
labels and palette workspace numbering now follow pinned/group/member order.
Collapsed children retain their numbers and remain reachable; selecting one
expands its group. This avoids shortcut renumbering when a header is collapsed.
Eight focused group tests passed via mise, covering mixed pins, member pins,
collapsed versus navigation order, and reordered top-level groups. Native
keyboard/visual interaction remains pending.

Preserved the sidebar slot of a group when its anchor closes, detaches, is
removed/regrouped, or is explicitly replaced. A live previous anchor trades
entry positions with its replacement; closing an anchor moves its replacement
into the vacated slot while retaining surrounding standalone row order. Active
workspace identity is restored after those changes, and closing the active
anchor selects its promoted member. Seven focused group tests passed through
mise, including replacement and closure with member storage on either side of
the header. Desktop compilation passed; native close/focus interaction remains
unverified.

Top-level workspace/group keyboard ordering now follows visible header rows and
uses each group's independent pin state. Header drag moves the group at its
anchor position, including relative to other group headers, without merging
members. Pinning projects rows into pinned/unpinned tiers without rewriting
underlying order; restore retains saved entry order instead of sorting by
individual workspace pins. Cross-tier drops return without changing membership.
Desktop compilation passed via mise. Native mixed-row ordering, anchor-promotion
position stability and shortcut-number order still need acceptance/refinement.

Workspace Up/Down now reorders group children within their member list, clamping
at the first child so ordinary movement cannot replace the anchor. Sidebar
workspace drops onto grouped rows place a child after the target, including
cross-group and standalone-to-group moves; dropping a child onto a standalone
row removes its membership before normal ordering. Anchor drops into groups
produce a notice instead of implicitly merging groups. Six focused model tests
passed through mise, including forward/backward placement, cross-group transfer
and anchor preservation. Native drag hit targets and full top-level group/pin
ordering still need acceptance and implementation work.

Added independent group colors and icons to saved state and sidebar headers.
File-menu/palette editors reuse workspace color swatches, icon presets, custom
entry and validation, with Enter/escape behavior retained. Group styling belongs
to the container and survives anchor changes; headers no longer borrow the
anchor workspace's style. Older saved groups default to no custom style. Desktop
compilation and six focused group/model-format tests passed via mise, including
custom-style round-trip, member-index mapping, backward defaults and invalid
color/icon rejection. Native visual/editor interaction remains unverified.

Added palette destinations for moving the active workspace into an existing
group, plus File-menu/palette actions to remove only the active member or make
it the anchor. Destinations include stable IDs to distinguish duplicate names;
the current group is omitted. Membership changes use existing entities and do
not spawn, move or terminate PTYs. Moving an anchor promotes a source member;
moving the last member removes its empty source group. A new focused test covers
destination identity after source removal, invalid destinations, same-group
idempotency and explicit anchor provenance. Four group-model tests and desktop
compilation passed through mise. Native palette interaction remains unverified.

New group members now inherit the anchor workspace's cached working directory;
new empty groups inherit the active workspace's directory without temporarily
joining or expanding its group. Ordinary new workspaces also inherit the active
workspace's cached directory. Cached OSC/launch metadata avoids acquiring the
terminal parser lock. Explicit-directory spawning shares the existing workspace
pane constructor. Restoring now reapplies saved collapse flags after focus setup,
and scrolling a hidden member targets its anchor header. Desktop check and the
extended native session smoke passed: a collapsed, pinned anchor-only group with
a Unicode name and stable identity survives automatic checkpoint, forced-stop
relaunch and normal Quit. Native cwd inheritance and multi-member collapsed
restore interaction still need direct acceptance coverage.

Added the first workspace-group sidebar UI: anchor headers replace the anchor's
ordinary row, members are indented, chevrons collapse/expand without changing
terminal focus, and header `+` creates a member after the anchor. Cmd-N inside a
group inserts after the active member. New Empty Workspace Group is available
with Ctrl-Cmd-G on macOS / Ctrl-Alt-G elsewhere; palette and File-menu actions
also create a group from the current workspace, rename, pin and ungroup. Header
double-click edits the group name. Group pins determine the top-level tier and
visible-row mapping drives workspace scrolling. Desktop cargo check passed;
native appearance/interaction remains unverified. Anchor cwd inheritance,
multi-selection, membership drag/drop, configurable placement and automation
still need implementation; current group creation uses normal new-shell cwd.

Connected group ownership to window state and layout/session persistence. Saved
groups carry stable group IDs, ordered workspace indices (anchor first), names,
collapse/pin state and generated-anchor provenance. Restore resolves indices
before workspace pin sorting; capture resolves current live identities back to
indices. Older files default to no groups. Validation rejects missing, duplicate
or multiply owned members, empty groups and invalid identities before shell
preparation. Closing/detaching a workspace now updates its source group and
promotes an anchor as needed. Existing 16 snapshot tests and four focused group
tests passed through mise. Sidebar rendering/actions and native group recovery
acceptance remain pending; groups are not yet exposed in the UI.

Started W08 workspace groups with a window-local ownership model in
`workspace_groups.rs`, based on the pinned cmux group contract. Ordered members
define the anchor; removing it promotes the next member and clears generated
provenance, while preserving group identity and collapse/pin state. Regrouping
keeps ownership exclusive, validates before mutation, and removes empty source
groups. Ungrouping returns members without touching terminals. Two focused tests
passed through mise, covering promotion, invalid operations, regrouping and
same-name groups with distinct identities. This is model groundwork only:
sidebar integration, saved sessions and automation remain next steps; no group
UI or full W08 completion is claimed.

Added optional zoomed-pane persistence using depth-first layout indices. Older
files default to unzoomed; invalid zoom indices are rejected. Restore focuses
the visible zoomed pane while retaining the full split tree and weights. The
focused format test and native session smoke passed: explicit and automatic
restore retained zoom, forced-stop/relaunch retained it again, and unzooming
returned the unchanged 1:3 split tree. Actual visual focus remains unverified.

Verified partial restore rollback with real PTYs. Preparation now accepts an
internal shell factory, allowing deterministic later-spawn failures without
changing environment variables. A focused test fails in the first and second
windows and cancels after the first spawn; each case confirms every previously
prepared shell is reaped before the error returns. The production path still
uses the existing shell constructor. The test passed in 0.24s. OS/window-opening
failures and native cancellation dialogs remain separate acceptance work.

Constrained the tab picker to the pane's actual available height using an inset
flex container, retaining its 240px cap and scrolling for overflow. This avoids
relying on stale terminal bounds during split resizing. Added full-title tooltips
for truncated picker rows. Desktop cargo check passed for the containment change;
native preview was retried but the Mac remains locked, so visual acceptance is
still pending.

Improved window notices with a bounded scrolling text area, consistent spacing,
background/border contrast, Copy and Dismiss controls. Active restore notices
retain Cancel instead of Dismiss, and action controls do not shrink behind long
messages. Desktop cargo check passed; native narrow-window/clipboard/scroll
appearance and interaction acceptance remain pending.

Made recovery failures visible in the fresh window and disabled automatic writes
after failed restoration, preserving the original file for repair instead of
overwriting it with fallback state. Lock failures also produce a notice. Native
smoke passed for malformed JSON and unsupported session versions: both launched
one fresh window, exited normally, and retained the original file bytes. Existing
restore/checkpoint/lease/forced-stop checks passed. Notice appearance remains
visually unverified while native UI access is unavailable.

Added optional window geometry to saved layouts/sessions, including windowed,
maximized and fullscreen modes. Older files retain default placement. Restore
prefers the display with greatest overlap and clamps bounds to an available
display, keeping windows reachable after monitor removal. Window bounds changes
now trigger coalesced recovery checkpoints. Focused fitting and format-validation
tests passed; native restore/crash-relaunch smoke passed and confirmed valid
windowed geometry in captured files. Exact visual placement, display changes,
maximized/fullscreen runtime behavior and platform acceptance remain pending.

Added an exclusive OS-backed lease for configured recovery files. A sibling
.lock file survives atomic checkpoint replacement; startup holds the lease
before restore, and background writers retain it through writes. A competing
launch falls back to a fresh window with recovery disabled. Focused tests passed
for exclusion and last-owner release. Native smoke confirmed a second process
does not restore/overwrite the owner's two-window checkpoint, then SIGKILL/relaunch
reacquires the lease and restores successfully. All fixture shells exited.
Cross-platform lock/runtime acceptance remains pending; Windows validation stays
deferred per user direction.

Fixed a checkpoint notification gap: font size/family changes now emit terminal
metadata events, reaching the observed pane/workspace path. Previously they only
notified TerminalView, which recovery deliberately does not observe per frame.
Desktop cargo check passed; native font-change/relaunch acceptance remains pending.

Verified recovery without normal Quit. The automatic native smoke waits for a
completed rename/directory checkpoint, SIGKILLs only its isolated test rmux,
confirms checkpoint bytes remain unchanged, and launches a new process from the
same configured file. Two windows, workspace names, split weights and tab labels
return with four distinct fresh shell PIDs; the Unicode reported directory
survives the subsequent normal-quit checkpoint. Cleanup verifies both generations
of fixture shells exit. The extended smoke passed against the existing binary.
Power-loss and interrupted-replacement behavior, uncheckpointed recent changes,
visual/OS focus and high-output capture latency remain unverified.

Removed terminal parser locking from layout/checkpoint directory capture. Each
TerminalView now retains the last delivered directory metadata as a PathBuf;
snapshots clone that cache instead of calling Session::current_directory. An
explicit spawn directory seeds the cache until the first OSC directory report,
so shells without integration no longer lose their requested directory on save.
Subsequent remote/cleared reports clear the fallback. The native session smoke
passed with assertions for all four initial directories and a later OSC 7 report
containing Unicode/spaces reaching the automatic checkpoint before Quit. Capture
uses delivered metadata rather than forcing a synchronous engine read. Large
workspace capture timings and remote-directory clearing acceptance remain pending.

Added opt-in event-driven checkpoints observing window/workspace/pane changes,
coalesced through a one-slot channel and a 750 ms timer that runs only after a
notification. No terminal-frame observer or idle polling is installed. A shared
background writer serializes automatic and quit saves, skips identical encoded
state, and uses generations to reject stale jobs; deduplication retains at most
one 8 MiB encoded session. Busy capture waits for another notification rather
than polling. Native restore smoke now renames a workspace and verifies the file
changes before Quit, then verifies the quit checkpoint/cleanup; both explicit and
automatic cases passed. A focused stale-writer regression covers generation
ordering. Forced-crash/relaunch, busy-interaction retries, output-pressure capture
cost and complete change-source coverage remain pending.

Connected opt-in automatic startup recovery and quit checkpoints through an
absolute session_file setting, frozen at startup. Normal launches recover it;
explicit layout/session/directory/command and benchmark launches do not touch it.
Unrestorable files fall back to a fresh window. Normal Quit captures before
shutdown and awaits background atomic save inside the shared three-second
cleanup deadline; closing all windows records an empty session. OS quit remains
best effort under GPUI's shorter deadline. Added system.quit RPC using the normal
confirmation flow. Native explicit and automatic two-window restore smoke passed;
automatic Quit wrote a newly captured checkpoint and all shells exited. Parent
directory creation, change-triggered/crash checkpoints, corrupt-file fallback
runtime coverage and native confirmation/OS-quit acceptance remain pending.

Added File/command-palette Save Session to produce the multi-window recovery
format from live window state. Capture preserves window enumeration order and
the active-window index, uses shared layout capture, excludes benchmark windows,
and rejects busy editing/restore/close interactions before showing the save
picker. Serialization and atomic replacement run on the background executor;
the saved state is the action-time snapshot. Desktop cargo check passed with no
remaining unused session-writer warning. Native save-dialog and captured-session
round-trip acceptance remain pending, as does automatic checkpoint scheduling.

Verified multi-window session startup through a new isolated native smoke and
mise smoke-session-restore task. Headless inspection starts no shells; an invalid
directory in the second window returns exit 2 before any fixture shell starts.
Valid startup restores two windows, Unicode workspace names, metadata/pins,
1:3 vertical split weights, tab labels and selected tab. Four distinct live shell
PIDs are observed and all exit after cleanup. Both the initial smoke and extended
preflight rerun passed; executable evidence is in
docs/benchmarks/session-restore-smoke.md. Visual/OS activation, active-pane focus,
partial-spawn rollback and automatic checkpoint scheduling remain pending.

Connected multi-window recovery files to startup with --session PATH and the
headless --check-session PATH. Preparation validates all windows/directories
before spawning shells and shares the existing cancellable preparation path;
failure in a later window drops and waits for previously prepared shells.
Startup opens saved windows in order and activates the recorded window (first
if none); an empty session opens a fresh default window. Live stdio RPC can be
combined with session startup. Existing CLI tests and a focused session-mode
conflict test passed. Native multi-window restoration and rollback fixtures are
still pending. Automatic checkpoint capture/scheduling is not yet connected.

Added a versioned multi-window SessionSnapshot format for recovery, preserving
window order and optional active-window index alongside existing layouts. Loading
is capped at 8 MiB, sessions at 16 windows, and the shell budget is shared across
all windows (256 total tabs). Empty sessions are valid. Window and session saves
share an atomic same-directory temporary-file replacement with file sync; invalid
state is rejected before replacing an existing checkpoint. Fourteen focused
snapshot tests passed, including multi-window round trip, prior-checkpoint
preservation, aggregate shell-budget rejection and oversized/version validation.
This is storage infrastructure only: automatic capture/write scheduling and
startup recovery are not wired yet. It restores fresh shells, not live processes,
and does not claim directory-fsync/power-loss durability.

Prepared layout capture for reuse outside the manual Save dialog. Window capture
is now a separate layout_snapshot method, and saved active-pane indices are
derived directly from depth-first layout order. Existing split/swap operations
already maintain matching pane-vector order, so this removes an implicit
dependency rather than fixing a demonstrated current restore failure. A focused
nested-tree swap/removal index test and desktop cargo check passed. Automatic
multi-window persistence, startup recovery and crash-safe scheduling remain
unimplemented; manual save behavior is retained.

Made the tab picker reachable through the command palette and a scoped terminal
shortcut (Cmd+Shift+T on macOS, Ctrl+Alt+Shift+T elsewhere), with matching control
tooltips. Added Home/End navigation and a picker-owned blur subscription so
switching focus/workspaces cannot leave a hidden picker blocking automation.
Explicit selection/dismissal releases that subscription; picker opening is
idempotent and refuses to interrupt tab rename/close prompts. Desktop cargo check
passed. Native shortcut routing and focus/overflow visual acceptance remain pending.

Added a compact tab picker to multi-tab pane headers. It lists tab titles with
active/unread indicators in a bounded scrolling overlay, selects by stable
terminal ID on click, and supports arrow navigation, Enter to choose and Escape
to restore terminal focus without changing tabs. Selecting a tab reveals it in
the existing horizontal strip. Outside clicks dismiss the picker, and automation
mutations treat an open picker as an active interaction. Desktop cargo check
passed. Native overflow, focus, narrow-pane placement and keyboard acceptance
remain pending while native UI access is unavailable.

Verified the last-tab cross-window transfer lifecycle through the native RPC
harness. A sixth fixture shell sets a Unicode variable, moves its sole tab into
an existing workspace, retains its PID/state, and loses only its empty source
workspace/window. The removed destination ID subsequently returns not-found.
All six fixture children exited after app termination. The harness now prints
its passed report only after cleanup assertions succeed. The focused live run
passed in under a second without rebuilding the unchanged application binary.
Busy/restore destinations and actual visual/OS-focus acceptance remain pending.

Added live surface.move_to_workspace with explicit surface/workspace IDs and
optional window activation, sharing the palette's transfer implementation.
Both same-window and cross-window destinations are supported; same-workspace
requests are no-ops. The native live-shell smoke initially caught an incorrect
terminal-ID-as-pane-ID lookup that crashed the new path; replaced it with direct
owner lookup and reran successfully. A multi-tab terminal crossed windows and
returned with the same identity/PID, destination selection and stale-ID errors
verified. All five fixture shell variables survived the complete sequence, and
cleanup reaped them. Desktop build passed. Last-tab source-window closure,
destination busy/restore cases and actual visual/OS-focus acceptance remain pending.

Added command-palette destinations for moving the active terminal tab into a
named workspace in another existing window. Live destination validation rejects
stale windows/workspaces and editing/restore/close states before source removal.
The transfer retains the terminal entity/PTY and label, rebinds window focus and
input subscriptions, targets the destination's remembered pane, follows zoom,
selects only the transferred tab, and removes an empty source pane through normal
workspace cleanup. Desktop cargo check passed. Native UI inspection was attempted
but the Mac is locked and automatic unlock failed; cross-window tab PID/state,
last-tab cleanup, stale-target and visual acceptance remain pending.

Added visual preset choosers inside workspace color/icon editing. Color swatches
and icon buttons reuse the existing menu presets, wrap to the sidebar width,
show the current draft selection and named tooltips, and include a clear option.
Choosing a preset updates the text draft through its undo path and restores text
focus; Enter commits and Escape cancels through the existing metadata editor.
Custom hex colors and glyph entry remain available. IME composition is not
overwritten by a preset click. Desktop cargo check passed; native visual,
pointer/keyboard and undo acceptance remains pending.

Added a reproducible release history-export benchmark and mise bench-history
task. A 100×32 mixed-Unicode fixture retained 4,519/77,194 history rows under
4/64 MiB budgets. With 64 MiB retention, the 1 MiB RPC cap took 2.324 ms warm p50
and 3.191 ms first access after compression; exporting the entire 5.48 MB text
took 12.550 ms warm p50 and 15.906 ms first access. Default 64 KiB reads stayed
below 0.32 ms in this run. Results and limitations are recorded in
docs/benchmarks/history-export.md. Retain the RPC cap; whole-app concurrent
render/input latency and allocation peaks remain unproven.

Connected full-history export to live surface.read_text with optional
scope:"history"; viewport remains the default. UI lookup creates a weak,
one-shot session job without locking the engine. A background executor performs
the bounded export, returns retryable busy errors on parser contention, and does
not retain a PTY/session while queued. The extended contention regression passed;
native live-shell smoke passed with Unicode soft-wrap reconstruction, history
byte truncation and invalid-scope rejection, alongside existing transfer/PID/state
checks. Desktop build passed. History output is capped at 1 MiB through RPC.
The formatter still holds the engine mutex during traversal; background execution
does not prove large-export rendering latency. Incremental contention mitigation
and quantitative large-history measurements remain pending.

Added bounded full-history plain-text export to the terminal core using Ghostty's
streaming formatter. It reads the active screen's retained history oldest-first,
unwraps soft wraps, preserves selection and viewport, and stops formatter output
at a caller-specified limit capped at 16 MiB. Truncated output ends on a UTF-8
boundary. Buffer capacity requests grow geometrically within the export limit,
avoiding a reallocation for every character callback. Three focused tests passed:
history/selection/viewport/alternate-screen behavior, every byte boundary of a
multibyte wrapped fixture, and 10,000 single-byte callbacks with at most three
capacity growths. This is a synchronous core API only; background session/RPC
integration and large-history responsiveness remain pending. Existing RPC text
reads still export the viewport only.

Added a focused PTY contention regression for snapshot reads and targeted input.
A separate thread holds the terminal parser mutex with pending output; try_frame
returns busy and input_without_scroll still accepts input before lock release.
Afterward, the shell receives the Unicode input, pending output remains visible,
the earlier snapshot stays immutable and the shell exits. The test passed in
0.02s. Its bounded hold/500ms guard detects accidental blocking; it is not an
interactive latency benchmark or proof of flood-output performance budgets.

Added surface.read_text for fresh bounded viewport snapshots, with dimensions,
scroll offset, explicit truncation and retryable busy errors. Session::try_frame
does not wait for the parser lock. Frame text extraction now streams directly
into one output string, preserving UTF-8 boundaries and trimming visual row ends
without intermediate per-row strings/vectors. Focused multibyte/spacing/limit
tests passed. The native live-session read check initially exposed a fixture
assumption that soft-wrapped output is contiguous; the fixture now accounts for
documented visual row breaks and passed, including Unicode output, byte limits,
invalid limits and all transfer/state checks. Full-scrollback export and sustained
snapshot-read latency remain pending.

Added surface.send_text with bounded nonempty UTF-8 payloads (16 KiB), identity
targeting and explicit queue-acceptance/error responses. It uses the existing
bounded writer without scroll-follow or terminal-engine locking, and does not
change focus/selection. The native live-session smoke delivered distinct Unicode
state values to all five fixture shells, verified selection was unchanged, moved
both workspaces, then queried each retained variable through its original
surface ID. All original PIDs stayed live; stored values survived; cleanup reaped
the fixtures. Compilation and the full smoke passed, including oversized/stale
input errors. Output-pressure throughput, queue saturation through RPC and visual
viewport acceptance remain pending; queued success is not execution confirmation.

Added cached PTY process_id and reaper-completion process_exited to surface
discovery without acquiring terminal/exit locks. Extended the native harness with
opt-in live fixture shells and a mise smoke-live-sessions task. Five independently
started shell PIDs remained live and unchanged through populated-workspace and
last-workspace moves; all five disappeared after harness cleanup. The complete
native run passed, including layout/identity, selection, zoom and notification
checks. This verifies process continuity, not arbitrary shell variable/history
state, visual focus or latency. PID reuse after exit is documented.

Added workspace.move_to_window over live RPC with explicit source/destination
IDs and optional activation. UI and RPC now share a result-returning transfer
path; both windows reject transfer during editing, palette, restore or close
interactions. Same-owner requests are no-ops. The native smoke moved a populated
workspace, repeated the request without duplication, moved the last source
workspace, verified empty-source window removal, retained every terminal ID and
layout tree, and focused a moved terminal. Compilation and the full smoke passed;
build hash is recorded in docs/benchmarks/live-rpc-smoke.md. The fixture uses
exited shells, so running-shell PID/state and visual activation remain unproven.

Verified automatic non-login Bash through a real macOS PTY and Ghostty terminal.
The fixture loads the private startup wrapper, observes its initial OSC 7 path,
changes to a Unicode/space/percent-containing directory, checks the decoded path
and command output, fills scrollback, navigates between shell-marked prompts and
waits for clean shell exit. It shares the existing zsh PTY fixture; both tests
passed in 0.07s. This closes the basic Bash wrapper-to-terminal integration gap;
user themes, full command-status markers, login-mode Bash and other platforms
remain pending.

Added explicit Unix non-login startup through RMUX_SHELL_LOGIN=0, retaining login
shells as the default. Non-login Bash now receives a private --rcfile wrapper
that sources user .bashrc before embedded rmux hooks; the wrapper lives with the
session and never changes user files. Integration opt-out remains honored.
Explicit command launches and Windows are unaffected. The focused macOS Bash
wrapper test passed for user rc loading, hook chaining, integration idempotence
and temporary-file cleanup; existing Bash integration tests also passed.
Automatic Bash login startup remains pending because login shells bypass rcfile;
native non-login PTY/theme acceptance and other-platform startup remain pending.

Added opt-in manual fish integration using named prompt/preexec/postexec/posterror
events and built-in URL escaping for OSC 7. The hooks preserve user prompt
functions, guard re-sourcing, support the existing opt-out, report command status
and close unfinished command regions at the next prompt. macOS bundle packaging
now includes the script. Added a no-install fish fixture and mise task. Python
fixture syntax and bundle shell syntax passed; fish syntax/runtime acceptance is
pending because no fish executable is installed and mise has no short-name fish
registry entry. No tool was installed. Automatic loading, input-start markers,
theme/event ordering and interactive syntax-error acceptance remain pending.

Made the command palette height-aware. Its maximum height follows the window
viewport, the overlay uses a smaller top inset below 480px, and the results list
can shrink while preserving its scroll handle. Search input does not shrink;
below 300px the descriptive heading/footer are omitted to leave more room for
input and results. Normal-size windows retain the existing 48px inset and 320px
result-list cap. Desktop compilation passed. Native compact-window clipping,
resize and keyboard/IME acceptance remains pending while visual access is locked.

Added command-palette Page Up/Page Down navigation in eight-result jumps, clamped
to list bounds while arrow keys retain wraparound and scroll-to-selection.
Existing IME composition protection remains. Long command titles have hover
hints, selected-result hover remains distinct, and clicks resolve the displayed
command's identity against current search results instead of trusting a captured
row position. Desktop compilation and focused empty/single/long-list navigation
boundary tests passed. Native keyboard, hover and visual acceptance remain pending.

Aligned sidebar workspace interactions with terminal tabs: standard double-click
opens the existing name editor; click callbacks resolve current workspace IDs
rather than captured list indices. Truncated names have full-name hover hints,
selected rows retain a distinct hover color, row spacing is explicit, and the
36px sidebar header keeps its height and divider aligned with the workspace
header. New Workspace exposes its platform shortcut in a tooltip and, where
space permits, inline. Desktop compilation passed for the primary change; native
visual/interaction acceptance remains pending because CUA reports the Mac locked.

Added workspace.rename with identity targeting, shared sidebar-editor name
normalization and existing busy/deadline guards. It changes metadata without
selecting or activating the workspace; active names also update the window title.
Sharing normalization fixes blank-looking control/whitespace-only names in the
editor, keeping them from later failing saved-layout validation. Compilation and
the native two-window smoke passed, including Unicode normalization, blank-name
and stale-ID errors, and renaming an inactive workspace without selection changes.
The smoke also covers the preceding JSON move/ID lookup changes. Native pixel
appearance and performance measurements remain pending.

Reduced automation allocation overhead. Target IDs are converted once per UI
request and compared as generation-bearing EntityIds, avoiding per-candidate
string allocation across windows/panes/tabs. Noncanonical spellings and overflow
remain not-found errors rather than aliases. Discovery now moves JSON arrays,
workspace layouts and response payloads instead of deep-cloning them during
flattening/wrapping; recursive split serialization also moves child trees.
All ten automation tests passed on the final change, including ID generation and
spelling checks plus nested layout serialization. The two-window native smoke
passed after the ID conversion and initial array-move change; later JSON wrapper
cleanup received focused test coverage. Allocation/RSS/latency benefits have not
been measured; no numeric performance claim is made.

Added window.create with optional activation (default false), bounded/deadlined
dispatch, close-prompt/shutdown guards and explicit creation errors. It uses the
existing new-window path, now parameterized for focus/app activation; ordinary
UI launches retain their previous behavior. Responses identify the new window
and its initial workspace. Expanded the native smoke to create a second window,
verify discovery ownership, focus terminals in both windows and retain all
original IDs. Compilation and the native smoke passed; recorded the build hash.
OS focus behavior, creation latency, shell-start failure behavior and visual
acceptance were not measured by this fixture.

Added explicit pane.zoom.set with strict pane_id/zoomed parameters, identity
resolution, existing busy/deadline guards and live capability advertisement.
Zoom-in selects the pane's current terminal without OS activation; zoom-out only
clears that pane's zoom and repeated requests do not toggle. Expanded the native
macOS smoke to check malformed/stale targets, repeated zoom-in, unrelated-pane
zoom-out, surface focus transferring zoom, and unzoom restoring the same layout
weights and terminal IDs. Compilation and the expanded smoke passed. Recorded
the executable hash in docs/benchmarks/live-rpc-smoke.md. Visual appearance,
cross-window behavior and latency remain outside this check.

Live stdio mode now combines with saved-layout restoration or directory/startup
command options while rejecting headless inspection combinations. Extended the
native harness to restore two workspaces, three panes and four tabs, then select
an inactive tab, sibling pane and different workspace. All target selection and
parent-link assertions passed without changing terminal IDs; notification,
stale-ID and EOF checks still passed. All six focused startup tests passed.
Evidence and executable hash are updated in docs/benchmarks/live-rpc-smoke.md.
Cross-window behavior, zoom, visual focus and latency remain unverified by this
check; no additional tools were installed.

Ran the live automation path against a freshly built native macOS debug app.
The isolated single-window/single-pane fixture passed discovery identity links,
workspace selection, surface focus, stale-ID errors, notification response
suppression and stdin-EOF survival. The fixture uses /usr/bin/true as its shell,
temporary settings and only terminates its own process. Added the reusable
scripts/smoke-live-rpc.py harness and mise smoke-live-rpc task. Evidence and exact
executable hash are in docs/benchmarks/live-rpc-smoke.md. This closes the basic
GUI/pipe integration gap; visual focus, cross-window changes, zoom, concurrency,
latency and other-platform acceptance remain pending.

Added surface.focus with strict surface_id/activate parameters and live-only
capability advertisement. Resolution searches current window/workspace/pane/tab
ownership by identity; stale IDs return a structured not-found error. The existing
terminal navigation path selects workspace and tab, remembers focus and moves
zoom to the target pane; OS activation remains opt-in. Shared automation busy
checks now also protect tab rename/close interactions for both surface focus and
workspace selection. Desktop compilation and all nine automation tests passed,
including malformed parameters, standalone rejection, routing, response IDs and
notification execution. Native focus, zoom, moved-tab and live-pipe acceptance
remain pending.

Extended live discovery with pane.list and surface.list, linking terminal tabs
to stable process-local pane/workspace/window IDs. Results expose tab titles,
selection/attention, cached directory labels, counts and zoom without accessing
PTY or Ghostty state. workspace.list now includes the nested split tree, axes,
relative weights and zoomed pane ID; clients need not infer geometry from internal
pane order. Desktop compilation and all eight focused automation tests passed,
including nested split serialization. Native live discovery and mutation/focus
acceptance remain pending.

Added live workspace.select with strict workspace_id/activate parameters and
process-local identity targeting across windows. Selection uses the sidebar path
and preserves its workspace/pane focus history; OS activation is explicit and
defaults off. Busy editing, palette, restoration, close-dialog and shutdown states
reject the operation. Unknown targets return a structured error without index
fallback. An atomic queued/started/canceled handshake prevents expired queued
mutations from executing later; lost replies after execution starts explicitly
report unknown outcomes. Selection notifications execute without replying.
Desktop compilation and all seven focused automation tests passed, covering
parameter validation, notification execution, typed errors, queue bounds and
deadline/cancellation state transitions. Native selection/focus and live-pipe
acceptance remain pending; no compatibility or native performance claim is made.

Added --rpc-stdio-live to launch the GUI with read-only discovery through inherited
pipes. window.list and workspace.list return process-local entity IDs, selection,
metadata and counts from UI state without terminal locks, snapshot extraction or
focus changes. Blocking IO runs on one named worker; a bounded eight-request
channel wakes the UI on demand, with a five-second reply deadline and nonblocking
reply delivery. EOF detaches automation without closing windows. Standalone mode
still advertises only its own methods. Compilation and all 54 desktop tests passed,
including live bridge routing, disconnection, saturation, capability separation,
notification suppression and CLI conflict coverage. Native GUI-to-pipe acceptance,
socket transport, targeting/mutations and events remain pending.

Added --rpc-stdio as a headless automation discovery foundation. It serves
system.ping and system.capabilities with JSON-RPC response IDs, structured
errors, notification suppression and a 64 KiB per-request limit; oversized
requests terminate the stream. It bypasses settings loading and UI/PTY creation,
and rejects launch/inspection combinations. Capabilities explicitly identify
standalone mode without live workspace access. All 52 desktop tests passed,
including protocol and CLI conflict coverage; an actual cargo-run stream returned
both discovery responses and exited successfully. Live-window transport, caller
scope, targeting and events remain pending; this is not cmux wire compatibility.

Stopped unpressed mouse-motion reports outside the inset text area, while
retaining local selection drags and releases for presses started in the grid.
Cell-based application mouse reports now use the actual terminal rows/columns
instead of ceiling the pixel area, and clamp trailing partial-cell coordinates
to the last valid cell. SGR pixel reports retain pixel coordinates. Focused tests
passed for fractional cell widths, bottom/right partial cells, resize, negotiated
motion and pixel reporting; desktop compilation passed. Native padded-pane and
drag interactions remain unverified.

Added terminal_padding (0–32 logical pixels, default 8). The terminal element
derives one inset content rectangle for PTY sizing, text/cursor placement,
selection coordinates and IME input bounds, while painting its background across
the outer rectangle. Content clipping uses the inset; find/status bars retain
their own layout. Padding clicks focus without generating an application mouse
press. Reload applies through existing observers; tiny panes clamp the content
size to zero instead of producing negative geometry. The focused geometry test
and desktop compilation passed. Native DPI, drag-selection, IME and visual
acceptance remain pending; earlier app benchmarks used zero inset.

F12 renderer diagnostics now include snapshot_storage: cell-vector capacity,
logical text bytes, inline/heap cell counts and heap text payload bytes. Frame
accounting walks cached data only when explicitly exported, with no engine lock
or fresh snapshot extraction. Added focused accounting assertions to the short/
long-grapheme fixture; that test and desktop compilation passed. Documentation
distinguishes cached/possibly stale hidden-tab data from Ghostty storage, allocator
overhead, shared allocations and process RSS. Native collection remains pending.

Added cursor_glyphs_reused/cursor_glyphs_rebuilt to opt-in F12 renderer diagnostics.
The counters record actual block-cursor glyph cache hits and rebuilds, without
timers or automatic exports. Existing row counters remain separate. Documented
their scope so native runs can measure reuse during repaints and invalidation
after text/font/color/DPI changes. Native collection remains pending.

Added a one-entry block-cursor glyph cache per terminal. Repainting the same
covered cell now reuses its shaped line instead of rebuilding text/font runs.
The key includes the complete cell, cell metrics, font family and display scale;
style, color, text and font/DPI changes rebuild it. Hidden-terminal cache release
and window rebinding clear the entry alongside shaped rows. The cache owns at
most one cell and shaped line and adds no timers. Desktop compilation passed;
native font/color transitions and cursor-cache latency benefits remain unmeasured.

Added macos_option_as_alt, default true, to let users opt into native Option text
composition. When false, Option-modified character/space presses without Control
bypass terminal key encoding and remain on AppKit's text/dead-key path. They do
not enter the terminal pressed-key map, avoiding orphan release reports. Control+
Option and navigation keys retain encoding; Linux/Windows behavior is unchanged.
Configuration default/type coverage and desktop test compilation passed. Native
international-layout, dead-key and TUI/IME acceptance remain pending.

Added terminal_theme presets dark and light on top of the existing color/palette
configuration. Dark retains current defaults; light supplies a white background,
dark foreground, blue cursor and a darker ANSI palette. Explicit colors and
palette override the preset, while null inherits it. Existing resolved-value
comparison and preference observers apply changes live without new timers.
Focused preset/override/validation coverage and desktop test compilation passed.
The preset affects terminal content only; app chrome, OS appearance following
and native visual acceptance remain unfinished.

Changed renderer text-run construction to determine each compatible style run
and its byte length before allocating its String. Long ASCII runs now reserve
their required capacity once instead of repeatedly growing the buffer; short
style runs do not reserve the remainder of the row. This adds a cell traversal
and is an allocation reduction, not a measured native speedup. Updated the empty-
cell test fixture for immutable small-string storage. Focused row-cache tests
cover concealment, decorated blanks, style boundaries, wide cells and trimming;
native shaping/latency measurements remain pending.

Changed snapshot Cell.text from String to the existing smol_str dependency:
short graphemes remain inline, eliminating individual cell-string heap allocation
on the common path, while long graphemes remain supported. Renderer shaping
still builds owned text runs. All 61 terminal tests passed, including new
ASCII/CJK/combining and long-grapheme storage coverage; desktop compilation
passed. An eight-engine release run recorded 251.6 MiB/s and 0.148 ms snapshot
p99 (earlier separate run: 227.2 MiB/s and 0.186 ms). Raw evidence and limitations
are recorded in docs/benchmarks; no native latency or RSS improvement is claimed.

The current ARM64 Linux desktop cross-check passed via mise run
check-linux-desktop after the accumulated dirty-state, large-paste, cursor,
window-transfer, lifecycle and color/palette changes. This checks Rust/FFI
compilation against the Linux dependency build; it does not link or run the
desktop or establish native appearance, input or lifecycle acceptance. Windows
validation remains deferred as requested. Updated T03's feature ledger to include
the implemented color and ANSI palette settings while keeping native gates open.

Added ansi_palette as an optional, exactly 16-entry #RRGGBB configuration array.
New terminals apply it at creation; Reload Settings updates existing terminals
only when the palette changes. The engine updates default ANSI entries while
preserving default indices 16–255, leaving application OSC overrides separate.
Null resets the built-in defaults. Focused tests passed for normal/bright ANSI
cells, unchanged extended/true-color values, application override/reset behavior,
built-in reset and array-length validation. Desktop compilation passed. Native
theme appearance and full T03 acceptance remain pending.

Preference reload now compares each terminal's last successfully applied color
defaults before acquiring its engine lock. Unchanged colors avoid setter calls,
revision increments and full color-driven snapshot invalidation when changing
unrelated settings. Failed applications remain retryable on the next reload;
window transfers retain the applied state. Native access was checked again and
the Mac is still locked, so visual acceptance remains pending.

Added foreground_color/background_color/cursor_color configuration as validated
#RRGGBB strings or null defaults. TerminalView applies colors at creation and
through the existing event-only preference observer on reload. Ghostty default-
color setters preserve application OSC overrides; clearing configuration restores
rmux defaults. Snapshot revisions and Ghostty dirty state invalidate affected
render rows through the existing cache path. Focused tests passed for color
updates, old-snapshot isolation, OSC override/reset precedence and default reset;
configuration validation/serialization coverage and desktop compilation passed.
Native appearance/reload acceptance and full palette/theme controls remain open.

Centralized shutdown registration in TerminalView::new so normal tabs, splits
and restored terminals all use one path. Each registered handle now waits on
the existing reaper completion channel and removes completed registry handles
as soon as that channel closes, releasing empty vector capacity too. Previously,
normal creation pruned old handles but restore only appended them, and a closed
tab's bookkeeping could survive until another creation. The waiter retains only
the shutdown handle, not the session, PTY or terminal, and uses no polling timer.
Desktop compilation passed; native repeated restore/close memory and descriptor
measurements remain pending, so no RSS improvement is claimed.

Added a system-quit cleanup fallback through GPUI's on_app_quit hook. Shutdown
handle collection and process-stop requests now share begin_shutdown with the
normal Quit action; all requests are issued before awaiting any completion, and
the quitting flag prevents duplicate work. The normal action retains confirmation
and its three-second wait. GPUI's system hook cannot veto quitting and gives
returned futures only 100 ms, so this remains best-effort cleanup, not system-
quit confirmation or guaranteed teardown. Desktop compilation and the focused
stubborn-shell shutdown test passed; native system-quit acceptance remains open.

The command palette now offers named destination windows for workspace moves,
labelled by window-list number and the destination's active workspace. Both
next-window and explicit-target actions use the same transfer function. Palette
entries capture the original workspace entity ID and window handle rather than
re-resolving by mutable list index or label; a closed target fails before source
removal, and a missing source cannot move another workspace accidentally.
Desktop compilation passed. Native multi-window transfer, stale-target and focus
acceptance remain pending; full W06 parity is not claimed.

Added Move Workspace to Next Window to the menu and palette. It chooses the next
existing normal rmux window, transfers the same workspace/layout/terminal
entities with rebound window subscriptions, preserves metadata/pin state and
last-pane focus, and inserts pinned workspaces at the pinned-group boundary.
Destination ownership is established before source removal; an emptied source
window closes and the destination activates. Missing destinations and pending
restore/window-close operations report errors before mutation. Desktop
compilation passed. Native process-identity, focus, confirmation, pinned-order
and source-window teardown acceptance remain pending; W06 stays in progress.

Find Older/Newer now queues the requested navigation when reopening a closed
find bar, so one shortcut both starts the search and selects a result after its
scan completes. Repeat-find prefers the remembered query over unrelated current
selection; explicit Find retains selection-prefill behavior. Empty queries still
open the input without navigating. The existing worker retains movement during
incremental scans, avoiding UI-thread search work. Desktop compilation passed;
native shortcut/reopen acceptance remains pending.

Fixed tab-close focus handling: removing an inactive tab no longer unconditionally
focuses that pane's selected terminal. A replacement receives focus only when
the removed terminal contained it. Removing the final tab in a split now leaves
another focused pane alone rather than selecting a layout neighbor. Desktop
compilation passed; native pointer/prompt focus transitions remain unverified.
Rebuilt target/debug/rmux.app through mise, including the recent dirty-state,
large-paste, inactive-cursor and tab-interaction changes. Bundle creation and its
headless --version check passed; this is a development build, not a release.

Added the roadmap's eight-pane case to engine/PTY benchmark arguments and default
matrices. Corrected engine idle sampling to consume the last dirty frame before
timing unchanged reads. Sequential release runs recorded 227.2 MiB/s engine
throughput, 0.186 ms flood-snapshot p99, 63 ns unchanged reads, and 31.25/3.59 MiB
Ghostty resident page storage before/after compression. The real eight-session
PTY fixture completed 128 responses under seven flooding peers at 1.366 ms
response-snapshot p99; no benchmark children remained afterward. Raw JSONL and
scope are in docs/benchmarks. This is engine/PTY evidence, not native GUI latency
or proof of full P0 performance acceptance.

Tab-strip pointer polish: double-click selects and opens the existing rename
editor; middle-click uses the existing close-confirmation flow; title tooltips
expose labels truncated at 220 px. Close controls now have a centered 22×22 px
target and tooltip, and labels have a 48 px minimum for stable short-title tabs.
Pointer callbacks resolve terminal identity rather than relying on an old tab
index. Existing horizontal scrolling and active-tab reveal remain in place.
Desktop compilation passed; native click/drag/overflow visual acceptance remains
pending while the host UI is unavailable.

Inactive terminals now show a hollow cursor by default instead of losing their
insertion-position indicator. The new unfocused_cursor setting accepts hollow
or hidden and live-reloads across existing terminal views through an event-only
global observer. Cursor focus now includes window activation, so inactive
windows do not retain a focused cursor. Focused application cursor shapes,
application-requested cursor hiding and the IME bar caret remain intact. The
focused preference/shape test passed with desktop test compilation; native
visual focus-transition acceptance remains pending. No blink timer was added.

Large clipboard paste now accepts up to 1 MiB of UTF-8 text (previously just
under 64 KiB). Encoded data moves into one writer-queue message, retaining the
bracketed-paste envelope without interleaved keys or terminal replies. A per-
session permit admits only one queued/in-flight paste and is released on all
encoding, enqueue and write failure paths; the existing 32-message queue and
64 KiB ordinary-input cap remain. Focused tests passed for the exact size limit,
permit lifetime/queue rejection, existing sanitization and a real macOS PTY
receiving 140 KB of Unicode/multiline text followed by ordinary input in order.
Cross-platform pressure/cancellation acceptance remains pending for T07.
Native visual inspection was attempted again but the Mac remains locked.

Fixed stale per-row Ghostty dirty flags after snapshot extraction. The complete
owned snapshot now consumes render state with ghostty_render_state_clean rather
than clearing only the global flag. A regression test verifies that a subsequent
single-row edit exposes only that row, while the old snapshot stays immutable.
All 55 terminal library tests passed. Re-running the same 160×48 release fixture
across 1/4/16 terminals measured 2.11–3.85 µs extraction means versus the prior
122.6–130.3 µs, retaining identical row reuse. Raw results are in
benchmarks/cleaned-dirty-snapshot-reuse.jsonl. This removes unnecessary full-screen
decoding; whole-app latency and native UI parity remain unverified.

Added and ran the release dirty_snapshot_bench fixture across 1/4/16 terminals
at 160×48. Identical rewrites preserved 48/48 row identities; changing one row
preserved 47/48. Engine extraction means were 122.6–130.3 µs, showing that cell
decoding/comparison remains substantial even while renderer reshaping can be
avoided. Raw results and scope are recorded in benchmarks/dirty-snapshot-reuse.jsonl
and the benchmark README. No native UI speedup or before/after claim is made.

Dirty-row extraction now compares complete cell content/style against the prior
snapshot before replacing its row handle. Identical rewrites and selection-only
changes retain row identity, allowing the renderer to reuse shaped glyphs even
when Ghostty conservatively marks those rows dirty. Actual text/style changes
still replace rows through copy-on-write. Focused tests passed for identical
rewrites, selection isolation, style invalidation and unrelated-row reuse;
desktop compilation passed. The comparison adds CPU work to dirty extraction;
net native shaping/latency benefits remain to be measured.

Added Open Settings File to the menu and palette. Its explicit invocation
prepares the configured path on a background worker, atomically creates defaults
only if missing, and opens it through the system association. Existing content,
including temporarily invalid edits, is preserved. It shares the settings
in-flight gate to avoid overlapping opens/reloads. Focused create/preserve
coverage and desktop compilation passed; native editor handoff remains pending.

Saved-layout validation now rejects control characters in workspace names and
fallback tab labels, matching existing custom-title/description restrictions.
The shared decoder applies this before restore preflight or shell creation and
through --check-layout; save validation also rejects such state. Focused coverage
passed for newline, carriage return, escape, NUL and DEL while retaining the
valid Unicode fixture. Split geometry guards were already present and unchanged.

Added --check-layout PATH for headless structural validation using the same
bounded 2 MiB loader as restore. It validates schema/version, geometry and tab/
workspace limits without loading user preferences or spawning sessions. Launch
options and conflicting inspection modes are rejected. Focused parser coverage
and desktop test compilation passed. This command intentionally does not claim
directory availability or shell executability; restore retains its preflight.

Added --dump-config to print effective, reusable JSON preferences and exit before
GUI/PTY startup. Serialization excludes internal source/override bookkeeping;
the values include defaults and the applied close-policy environment override.
It rejects launch options, duplicates and combining with --check-config. A real
headless invocation printed the expected defaults and exited zero; focused parser
coverage passed. Separate launch environment variables are not part of this dump.

Pressed-key tracking now pairs recognized shifted/unshifted aliases through a
symbolic key identity instead of the raw UI string. A press reported as ? can
therefore pair with a release reported as / after Shift is lifted; the original
sent key is retained for release encoding and focus-loss cleanup. Unicode
identities remain distinct, repeats keep one entry, and unsent shortcut/text
events still cannot emit orphan releases. Five focused keyboard tests and desktop
compilation passed. This does not replace physical scan-code/layout support;
native modifier-order acceptance remains pending.

Single-character non-ASCII key identities now use Ghostty's unidentified-key
path with a Unicode codepoint, allowing report-all mode to encode them rather
than silently falling through. Focused coverage confirms é produces CSI 233 u
only under negotiated report-all mode. Desktop key handling also leaves ordinary
character identities without native text on the composition path, avoiding a
premature key report for those dead-key events. Desktop compilation passed;
native dead-key, international layout and IME acceptance are still pending.

Kitty report-all-keys mode now routes mapped ordinary character presses through
Ghostty instead of bypassing the encoder. The desktop supplies native key text
for unmodified/Shift characters, preserving associated-text reports separately
from the unshifted key identifier. Existing pressed-key tracking pairs releases;
normal text and active composition retain their native input paths outside this
mode. Four keyboard tests passed, plus focused shifted associated-text coverage
and desktop compilation. Non-ASCII key identities, physical layout mapping and
native application/IME acceptance remain incomplete.

Shifted ASCII punctuation now maps to Ghostty key identifiers instead of falling
through to native text handling when combined with Control/Alt. Focused encoder
coverage confirms Ctrl+Shift+underscore/caret/question-mark produce US/RS/DEL,
and Ctrl+Shift+at-sign follows Ghostty's CSI-u disambiguation. Unmodified symbol
text still uses native input/IME. Native keyboard-layout acceptance remains open;
these symbolic mappings do not provide physical-key identity for arbitrary layouts.

Added Reveal Working Directory to Edit and the palette through GPUI's platform
reveal API. It shares local-directory resolution and unavailable-path feedback
with Copy Working Directory, preserving OS paths without constructing a shell
command or file URL. Remote directory reports are not mapped to local paths.
Desktop compilation passed; native file-manager handoff and platform acceptance
remain pending.

Added Copy Working Directory to Edit and the command palette. It uses the
existing local-only OSC directory resolver on demand, with no filesystem polling
or process inspection. Unavailable/remote metadata leaves the clipboard untouched
and reports why; valid paths copy as literal text rather than shell commands.
Host workspace all-target compilation passed, including examples/tests after the
shared Frame API changes. Native clipboard acceptance remains pending.

Added copy_on_select (default false) to bounded JSON preferences. When enabled,
releasing a local mouse selection copies through the existing Unicode-aware,
size-limited clipboard path; application mouse reports do not trigger copying.
The preference is read at release, so Reload Settings applies to existing
terminals. Configuration validation and desktop test compilation passed.
Native clipboard, Shift-override and rectangular-selection acceptance remain open.

The optimized macOS bundle has been rebuilt with the recent snapshot, search,
navigation and cache-lifecycle changes. Benchmark packaging now ad-hoc signs
after changing the bundle identifier and launch environment, then verifies the
signature. Generated the eight-pane benchmark bundle successfully and its
--version command exits zero. These are packaging/headless checks, not a native
eight-pane performance result. Developer ID signing/notarization remain separate.

Rebuilt the macOS bundle and found its copied executable was killed at startup
(exit 137), while target/debug/rmux ran normally. Code-signature verification
failed for the bundle. The packaging script now applies a local ad-hoc signature
after copying the executable, plist and resources. The rebuilt bundle verifies
successfully, and its --version and --check-config commands both exit zero.
This is local development signing, not Developer ID signing or notarization.
The fresh CUA check still reported a locked Mac; native UI acceptance is pending.

Frame extraction now allocates its UTF-8 scratch buffer only after finding a
dirty row. Unchanged frame reads no longer allocate that 64-byte temporary.
Focused styled/Unicode snapshot coverage passed. The release snapshot benchmark
follow-up measured 64–83 ns unchanged-frame means (four-terminal fixtures:
67/66/66 ns for 24/48/120 rows); raw results and single-run caveats are recorded
in benchmarks/lazy-scratch-snapshot-costs.jsonl and the benchmark README.
Native latency and allocation instrumentation remain pending.

Added and ran a release snapshot microbenchmark at 1/4/16 sequential terminals
and 24/48/120 rows. Unchanged engine frame means were 72–99 ns; Frame clones were
3–5 ns. A synthetic copy of the old row/selection vectors measured 49–250 ns and
grew with row count. Raw results are in benchmarks/shared-snapshot-costs.jsonl;
the benchmark README documents the fixed-order, one-run scope. This does not
measure native UI latency, dirty frames or whole-app memory, and is not a
before/after comparison of full builds.

Selection spans now share an Arc-backed vector from engine Frame through the
paint snapshot. Dirty text rows only copy that vector when their selection span
actually changes; unchanged snapshots clone handles without copying either row
or selection vectors. Copy-on-write preserves existing highlight snapshots
through selection changes and clears. Focused selection lifetime and unchanged
snapshot reuse tests passed, along with desktop compilation. Native performance
measurements remain pending.

Engine Frame snapshots now share their outer row list as well as individual
cell rows. Cloning an unchanged frame avoids allocating a row vector and
incrementing each row's reference count. Dirty-row replacement and resize use
copy-on-write, preserving older snapshots and sharing rows Ghostty leaves clean. Focused
coverage passed for outer-list reuse, changed-row isolation and resize lifetime;
desktop compilation passed. Selection vectors still clone, and native allocation
and latency measurements remain pending.

The shaped-row cache now shares its complete row vector with the paint snapshot
through an Arc. Unchanged repaints check source-row identities and clone only
that outer handle, avoiding a vector allocation and one reference-count update
per row. Changed content uses copy-on-write so any outstanding paint snapshot
remains coherent; font/scale/background changes still invalidate the cache.
Four focused row-cache tests passed with desktop test compilation. Native paint
latency and allocation measurements remain pending; no speedup figure is claimed.

Added RMUX_SHELL as an explicit executable override for normal and restored
shell terminals, before inherited SHELL/COMSPEC and platform fallback. Shell
paths now remain OS strings rather than requiring UTF-8; spaces are literal,
an empty override errors, and empty inherited values use the platform default.
Existing Unix login arguments and zsh integration remain in the shared launch
path. Direct startup commands and benchmark fixtures are unchanged. Focused
resolution coverage and desktop compilation passed; native custom-shell launch
acceptance remains pending.

Metrics export now attempts the engine lock without waiting when collecting
memory statistics. A busy parser produces a null memory sample while the render
counters are still exported; null is documented as unavailable, never zero.
The normal blocking memory API remains available for engine measurements.
Desktop compilation passed. Snapshot formatting still runs on demand on the UI
thread; file writes remain asynchronous and single-flight.

Render-metrics JSONL file opening/appending now runs on the background executor.
An application-wide in-flight flag ignores repeated export requests until the
write completes, avoiding concurrent appends and an unbounded export queue.
Completion clears the flag even if the originating workspace has closed; write
errors appear on that workspace when it still exists. Snapshot collection still
runs on demand on the UI thread and covers all tabs in the active workspace.
Desktop compilation passed; native export responsiveness remains unverified.

Opt-in render diagnostics now report cache_releases and rows_released alongside
rows_rebuilt/rows_reused and paint timings. Inactive-tab, hidden-workspace and
zoom release paths share one counter update; empty releases do not count. These
are cumulative dropped cache handles, not bytes freed or GPU atlas eviction.
No timers or automatic writes were added. Desktop compilation passed; native
before/after switching measurements remain pending.

Shaped-row cache release now covers workspace visibility changes and panes hidden
by zoom. Each window keeps a weak reference to its last rendered workspace and
releases that workspace's caches when the rendered target changes, covering
navigation paths without scanning every hidden workspace each frame. Zoomed
workspace rendering clears caches of the other panes. These operations neither
notify nor stop terminal processes; visible rows reshape on return. Desktop
compilation passed. Native switch/zoom latency and RSS measurements remain open.

Pane rendering now releases shaped-row caches belonging to its inactive tabs,
without notifying or adding timers. Engines, PTYs and scrollback remain owned;
returning to a tab rebuilds its visible glyph rows. Empty caches skip mutation.
This drops cached row handles and vector capacity, though GPUI may retain shared
font/atlas resources and the current paint frame can temporarily own rows. Four
focused cache tests passed, including release ownership/capacity. Whole-app RSS
and tab-switch latency are not yet measured; hidden-workspace caches remain a
separate lifecycle gap.

Concealed terminal cells now skip glyph shaping and decoration painting entirely,
while their backgrounds and column positions remain intact. Previously they
were painted in the cell background color, which could make their outlines
visible over a selection highlight. Visible text runs now stop at concealed
cells, and overlines are suppressed there too. Three focused row-cache tests
passed, covering concealed run boundaries alongside existing Unicode/style and
decorated-blank behavior. Native selection-highlight appearance remains pending.

Added direct tab selection within the focused pane: Cmd+Shift+1–9 on macOS,
Ctrl+Shift+1–9 elsewhere, plus Select Tab 1–9 palette actions. Next/Previous Tab
are now exposed in the palette too. Selection uses the existing pane path to
reveal the tab, focus its terminal and preserve its process; absent numbers do
nothing. Existing workspace/pane shortcuts remain distinct. Desktop compilation
passed; native shortcut routing and platform acceptance remain pending.

Added Last Prompt to the menu and command palette. It navigates from the live
viewport backward to a nearby prompt and then forward to the newest retained
marker, so a prompt inside the active screen is not accidentally skipped.
Unmarked output falls back to the live viewport. Focused engine coverage passed
for long trailing output, an arbitrary older viewport, a latest visible prompt
and no markers; desktop compilation passed. Native acceptance remains pending.

Invoking Find while its bar is already open now focuses and selects the query
for replacement, with explicit FindInput shortcut bindings on macOS and other
platforms. The shared text-field Select All path preserves active IME marked
text and resets pointer selection state otherwise. This does not recreate the
worker or resubmit an unchanged query. Desktop compilation passed; native
keyboard and IME acceptance remain pending.

Find now remembers the last query per terminal when closed, including closes
caused by Clear Scrollback or transfer to another window. A valid current
selection takes precedence when reopening; otherwise the saved query returns
selected for replacement and the on-demand search worker restarts. Retention is
one bounded query string, not a running worker or search index, and is not added
to layout snapshots. Desktop compilation passed; native acceptance remains open.

New Find bars now prefill from a single-line terminal selection up to 4 KiB and
start the existing asynchronous search worker immediately. The text starts
selected for replacement; reopening an existing bar preserves its query. A
bounded selection extractor checks formatted byte size before allocating its
output, so a large selection is not copied just to seed Find. Multiline/control
text and oversized selections leave the field empty. Focused Unicode extraction
limit coverage and desktop compilation passed; native interaction remains open.

Refreshed target/debug/rmux.app through `mise run bundle`, including the current
preferences, font handling and hyperlink implementation. The build and bundle
copy completed successfully. A fresh native-UI availability check reported the
Mac locked and automatic unlock unavailable; no launch or click acceptance is
claimed. The bundle is ready for the next unlocked native verification pass.

Link opening now checks the displayed frame revision against the engine under the
session lock. Output, viewport movement, resize, retention changes and selection
navigation invalidate older snapshots; synchronized-output holds retain the old
frame revision. This prevents mixing cached text with newer wrap metadata or
opening a newly substituted OSC target before its frame is displayed. The check
adds scalar bookkeeping without per-frame URI scanning. Four focused hyperlink
tests and desktop compilation passed, including output holds, scrolling and
resize invalidation. Native click acceptance remains pending.

Plain-text HTTP(S) URLs now use the same modifier-click path as OSC 8 links.
Detection runs only on click, follows soft-wrap flags, maps wide Unicode cells to
text offsets, excludes prose punctuation and preserves balanced URL parentheses.
Explicit OSC 8 targets win. Logical lines are bounded at 8 KiB; lines clipped at
either viewport edge are rejected to avoid launching truncated addresses. Three
focused hyperlink tests and desktop compilation passed, including wrapping,
wide-cell prefixes, explicit-target precedence and clipping. Native interaction,
hover feedback and file-path routing remain open.

OSC 8 HTTP(S) hyperlinks now open in the default browser with Cmd-click on macOS
or Ctrl-click elsewhere. Lookup reads the clicked viewport cell under the engine
lock and allocates URI text only on demand, capped at 8 KiB. Ordinary clicks keep
their selection/application-mouse behavior; link presses do not send a matching
release to the terminal application. Focused engine tests passed for scrollback,
alternate screens, wide Unicode link text, out-of-grid coordinates and oversized
URI rejection (the current Ghostty parser drops the oversized OSC itself).
Desktop compilation passed. T10 remains in progress: native click acceptance,
hover feedback, plain-text URLs and local/remote path routing are still pending.

The engine benchmark now accepts a scrollback budget and reports retained history
lines. A four-engine release comparison at 0/1/4/16 MiB per terminal is recorded in
benchmarks/engine-scrollback-budgets.jsonl. At 1 versus 4 MiB, total engine resident
storage was 3.13 versus 15.63 MiB with similar throughput (~268/~267 MiB/s). Zero
retention used 1.56 MiB but ran slower (~174 MiB/s); 16 MiB used 62.50 MiB before
compression. These are one-run engine-only measurements, not whole-app memory or
native interaction acceptance. The benchmark README records scope and caveats.

Configured scrollback retention is now set before PTY child creation, covering
normal shells, direct startup commands and manual/startup layout restoration.
This prevents early output being retained or evicted under the old default while
waiting for a view. The benchmark constructor retains its fixed 4 MiB budget.
A focused real-PTY test passed for immediate startup output with zero and 1 MiB
limits; desktop compilation passed. Native UI and broader memory measurements
remain pending.

Added scrollback_mib preferences (0–256, default 4) applied when a terminal view
is created. Zero disables retained history; reload affects newly created views
without trimming existing sessions. The shared engine setter uses Ghostty's
retention API and clears stale selection state. Focused engine coverage passed for
existing-history removal, active-screen preservation, disabled future history and
resumed retention; preference validation and desktop test compilation passed.
The limit is an engine history budget rather than a total RSS/GPU cap. Native
configuration acceptance and memory measurements at alternate limits remain open.

Use Configured Font now applies the configured family and size to the focused
terminal through View/palette actions, preserving its session and history. Font
changes invalidate measurement and cancel selection-drag scrolling; row-cache
font keys trigger reshaping on the next paint. Reset Font Size still changes only
size. Requests for the already-selected family/size now avoid measurement reset
and redraw. Desktop compilation passed; native live-font switching, reflow and
IME acceptance remain pending.

New tabs and splits now inherit the source terminal's font family as well as its
size. New workspaces without a source terminal use the current configured default;
the benchmark fixture explicitly retains its platform font. Each new terminal
measures its inherited font independently on first paint. Desktop compilation
passed. Native inheritance after reload, mixed-font layouts and resize acceptance
remain pending.

Manual layouts now preserve each terminal's font family as well as its size.
The optional field keeps older layouts compatible: absent families use the
current configured default. Restored families are validated before shells launch
and measured when their views render. A focused snapshot test passed for Unicode
names, round trips, omitted fields and invalid/control/oversized names; desktop
test compilation passed. Native mixed-font restore and fallback acceptance remain
pending.

Added validated font_family preferences for newly created terminals. Row shaping,
cursor text and IME preedit/hit testing use the same per-terminal font; row-cache
keys include the family. Grid width is measured from the resolved font's zero
advance, with line height at least its ascent/descent extent. Measurement occurs
once per font size rather than per frame, and zoom invalidates it. Existing
terminals retain their family; restored layouts use the current configured one.
The focused preference validation test and desktop test compilation passed.
Native glyph alignment, IME/selection geometry, fallback fonts and rendering
performance remain unverified while desktop access is locked.

Reload Settings is now available through the menu and palette. File reading and
validation run on the background executor; successful results replace global
defaults/policy together, and failures retain current settings with a visible
diagnostic. Requests are coalesced across windows. The source path is made
absolute before startup directory changes, and the startup environment override
is retained. No filesystem watcher is added. A focused test passed for changed
font defaults, preserved override, invalid reload rollback and missing required
files; desktop test compilation passed. Native reload feedback and font reset
acceptance remain pending. Existing font sizes are preserved until explicit reset.

Added `rmux --check-config` for headless validation of the same file/environment
settings used at startup. It returns 0 with a success message or 2 with the
configuration diagnostic, and rejects combinations with layout, directory or
command launch options. Real binary invocations passed for valid, invalid and
missing explicit files, help despite invalid settings, and command conflicts.
No GUI or terminal process is started by this mode.

Added startup JSON preferences for font_size and confirm_close. Unix defaults to
XDG_CONFIG_HOME/rmux/config.json or ~/.config/rmux/config.json; Windows uses
APPDATA/rmux/config.json. RMUX_CONFIG selects an explicit required file, and
RMUX_CONFIRM_CLOSE overrides file policy. Missing default files use defaults;
invalid values, unknown keys and files over 64 KiB fail before shell preparation.
No files are automatically created and there is no watcher. New terminals and
Reset Font Size use the configured default, while splits inherit their source
size and restored layouts retain saved sizes. The focused defaults/validation/
missing-file/size-limit test and desktop test compilation passed. Native font
acceptance, live reload and settings UI remain pending.

Close policy now evaluates activity lazily. Always/never settings and empty scopes
skip terminal-engine locks and foreground-process-group queries entirely. The
focused policy test passed with assertions that those branches never invoke the
activity query. The macOS bundle was refreshed before this policy optimization;
a fresh native-access check again reported the Mac locked, leaving GUI acceptance
pending.

Termination prompts now share one application-wide permit, preventing tab,
workspace, window and Quit requests from stacking confirmation dialogs. The
permit releases on answer or task drop, independently of whether its original
window/entity can still be updated. A focused test passed for overlap rejection
and release/reacquisition after cancellation; the desktop test build passed.
Native modal-dialog behavior remains pending.

Active close warnings now include live direct commands and Unix foreground jobs
whose PTY process group differs from the owning shell PID, in addition to program
status records. This uses portable-pty's on-demand foreground-group query, without
process-table scans or timers. A real macOS PTY test passed for idle shell → sleep
job → Ctrl+C → idle and direct-command detection/cleanup. Desktop compilation
passed. Linux runtime and native dialogs remain pending; Windows job detection,
background jobs and shell-replacing exec commands are not covered by this signal.

Close-warning checks now use a boolean session/status query without cloning,
sorting or resolving inherited app strings for every record. The bounded status
scan returns as soon as a working/blocked record is found, and exited sessions
return false. Existing state-transition coverage now checks the predicate before
and after task completion; that focused test and desktop compilation passed.
This removes avoidable allocation from tab/workspace/window/quit checks without
adding polling or background work. Native close latency remains unmeasured.

Added a shared startup close-warning policy through RMUX_CONFIRM_CLOSE:
active (default), always or never. Tab/workspace/window/native-window-close and
Quit action paths use it consistently; empty scopes do not prompt. Invalid values
fail before layout/command shell preparation, while help/version remain available.
Prompt wording now describes termination/output loss without assuming every
prompt was triggered by reported work. The focused policy test passed for empty,
idle, active and invalid settings. Native dialog acceptance, OS-originated quit
routes, persistent settings UI and unreported foreground-task detection remain
pending.
The Quit rmux action now scans all rmux windows for reported active work and
shows one application-wide confirmation, defaulting to Cancel. Repeated Quit
actions are suppressed while it is pending; confirmation enters the existing
asynchronous process-cleanup path. No-active-work and last-window cleanup retain
their direct shutdown behavior. Desktop compilation passed. Native multi-window
quit/cancellation acceptance, OS-originated quit routing outside the rmux action,
warning configuration and non-reporting process detection remain pending.

Close Window now checks all workspaces for reported active work. The menu/shortcut
action and GPUI native-window close callback share a confirmation path, defaulting
to Cancel. A pending window prompt suppresses duplicate requests; a pending
workspace prompt must be resolved first. Empty/idle windows retain direct close.
The native callback is installed once per window and holds a weak root reference.
Desktop compilation passed. Native close-button, cancellation and multi-window
dialog acceptance remain pending, as do application-quit warnings and policy
configuration. This does not detect work that omits program-status reporting.

Close Workspace now checks all terminals for reported working/blocked tasks and
asks for confirmation before terminating the workspace. Confirmation resolves the
workspace entity rather than its list index, so reordering cannot redirect it;
removed or transferred targets are ignored. Empty-workspace cleanup and workspace
detachment retain their non-prompting ownership-transfer paths. Tab/workspace
checks share the same active-work predicate. Desktop compilation passed; native
dialog acceptance, window/app close routing and configuration remain pending.

Tab close requests now warn for live sessions with any working/blocked program
status record. The prompt defaults to Cancel; confirmation resolves the original
terminal identity and focus handle before removal, avoiding index drift or
cross-window rebind races. Repeated requests are suppressed while pending and
workspace rebind clears pending editor/prompt state. Transfer operations do not
go through termination confirmation. Desktop compilation passed; native prompt
acceptance remains pending. Workspace/window/app closing, configurable policy and
detection of foreground work that does not report status are still unfinished.

Added Focus Pane 1–9 palette actions and terminal-scoped shortcuts: Cmd+Option+
number on macOS, Ctrl+Alt+Shift+number elsewhere. They select the pane in traversal
order, retain its active tab/session and move zoom when zoomed. Missing pane
numbers are ignored. Desktop compilation passed; native shortcut dispatch,
keyboard-layout compatibility and zoom interaction remain pending. W04 and W10
implementation records now include numeric pane and named terminal selection.

Direct startup commands are now prepared before GUI initialization. An executable
spawn failure prints its cause to stderr and exits with code 2, rather than
opening an empty workspace. The first pane consumes the prepared session once;
subsequent panes still create normal shells. A real headless invocation with a
missing absolute executable built successfully and exited with code 2 and the
expected diagnostic. Successful command-window rendering remains unverified;
rmux still does not propagate a completed child's exit code to its launcher.

Direct-command tabs now use the program's basename as their fallback label,
bounded to 128 control-free characters. Command arguments are not displayed in
tab labels. Automatic OSC titles and explicit renamed tabs still take precedence.
Exit and post-exit input messages now say “process” so they describe both shells
and directly launched programs accurately. The focused real-process exit test
passed (status 7 and rejected subsequent input), and desktop compilation passed.
Native command-tab rendering remains pending; layouts still restore fresh shells,
not previously launched commands.

Startup accepts `rmux [-d PATH] -- PROGRAM ARG...` and runs the argv directly in
the first terminal, without shell interpolation. Arguments retain OS-string
boundaries; later tabs/panes use the normal shell. Commands inherit the launching
directory unless overridden, receive the terminal environment, and use existing
PTY output/exit handling. Layout and benchmark combinations are rejected. Focused
parser and real Unix PTY tests passed for literal metacharacters, Unicode/space
arguments, directory/environment and invalid empty commands; desktop compilation
passed. Native first-window rendering and Windows execution remain pending.
Spawn failures now report before GUI startup (see the newer startup entry above).
Completed child exit status is not propagated to the rmux launcher.

The command palette now lists every terminal in its window with workspace, pane,
tab number and display title, using existing fuzzy matching. Selection resolves
live IDs and reveals the destination pane through the shared terminal-focus path;
closed targets are ignored. Custom tab names now also appear in notification
cards, and tab/drag/palette/notification labels share one title resolver. Entries
are built when opening the palette rather than maintained by a polling task.
Desktop compilation passed; native switching, stale-target and zoom acceptance
remain pending.

Rename Tab is available through the menu and command palette, using the existing
text editor with Enter/Escape handling. Custom names override automatic OSC titles;
an empty submission restores automatic naming. Overrides live on terminal entities
so pane/workspace moves retain them, and optional snapshot fields preserve them
through manual save/restore. Input is bounded to 128 characters and control-free.
Desktop compilation and the focused snapshot test passed for older layouts,
Unicode round trips and invalid/oversized names. Native editing, transfer and
restore acceptance remain pending.

Automatic Bash startup experiment: the installed macOS Bash 3.2 did not execute
an ENV injection file under interactive --posix, including login/non-login and
--noprofile variants. A proposed private wrapper therefore failed its startup
test and was removed before enabling it in Session::shell_in. Existing login
startup and manual integration remain unchanged; terminal-core compilation
passed after removal. Do not assume Ghostty's POSIX/ENV injection works on this
Bash version. Automatic loading needs a version-compatible approach and isolated
startup-order coverage before being enabled.

Added manually sourced Bash integration for OSC 7 directory tracking and OSC 133
prompt/input boundaries. It preserves scalar PROMPT_COMMAND hooks (including
trailing comments), supports array hooks, avoids duplicate sourcing and replaces
its own prompt suffix after theme changes. A focused macOS Bash 3.2 fixture passed
for hooks, idempotence, prompt replacement, opt-out and Unicode/control-character
paths; it caught and fixed Bash 3.2's signed-byte printf behavior. The bundle recipe
now includes the script. Automatic Bash startup, command lifecycle markers,
modern Bash array-hook runtime coverage and native UI acceptance remain pending.

Last Terminal now selects the destination tab before the workspace selection
focuses its pane, avoiding a transient focus on that pane's previously active tab
and unintended unread acknowledgement. The source pane is remembered first.
Desktop compilation passed. The macOS bundle was refreshed; a new native-access
attempt confirmed the Mac remains locked, so interactive acceptance is still
pending.

Last Terminal is now available in the menu and command palette. Each window keeps
up to 32 recently focused terminal IDs, fed by existing focus/metadata events.
The command resolves the prior terminal against live workspaces/panes/tabs,
prunes stale IDs, selects its workspace and tab, and exits pane zoom to reveal it.
History owns no terminal/session references and follows tab moves within a window.
Desktop compilation passed. Native mouse/keyboard focus history, repeated toggles,
closed-tab fallback and cross-window detach acceptance remain pending; no default
shortcut has been assigned yet.

Scroll to Top and Scroll to Bottom are now explicit View menu and command palette
actions, using the existing terminal viewport path. This makes history boundaries
discoverable alongside Previous/Next Prompt and the existing keyboard navigation.
Desktop compilation passed; native menu/palette dispatch acceptance is pending.
Dedicated last-prompt behavior remains separate and unfinished.

Layout restore now checks every saved working directory before launching any
shells, including tabs in nested splits and other workspaces. Missing paths and
regular files fail preflight; cancellation is checked during traversal. The
focused test passed for nested paths, missing/file/directory transitions,
unspecified directories and cancellation. Spawn-time checks and rollback remain
in place for filesystem races or later process errors. Native restore acceptance
remains pending.

Startup now accepts --layout PATH and uses the same bounded snapshot validation
and fresh-shell preparation as manual restore. The prepared window is published
without creating a throwaway default shell. Preparation occurs before GUI startup;
errors report to stderr with exit code 2. Working-directory override and benchmark
mode combinations are rejected. Restore failure/cancellation now waits for owned
session rollback before returning, avoiding immediate CLI exit racing asynchronous
child reapers. Parser coverage passed for layout paths and conflicting/missing
arguments. Native startup rendering and partial-restore cleanup acceptance remain
pending; this does not resume processes or scrollback.

Session startup now validates every explicit working directory before allocating
the terminal or PTY. Missing paths and regular files produce a directory-specific
error instead of portable-pty's Unix fallback to HOME. This covers restored tabs,
inherited directories and startup overrides through the shared spawn path. A
focused test passed for missing and non-directory targets, verifying rejection
before executable lookup. Filesystem changes racing validation and spawn remain
subject to the underlying PTY adapter; this is not an atomic directory handle.

Added startup CLI handling for --help/-h, --version/-V and --working-directory/-d.
Help/version return before GUI initialization; unknown/missing/duplicate arguments
and inaccessible directories report an error with exit code 2. Paths remain OS
strings. The selected directory is passed explicitly as the fallback for new
shells; a focused terminal's reported directory takes precedence. This explicit
fallback is required because portable-pty otherwise starts Unix shells at HOME,
regardless of the app process cwd. Parser tests and a real headless --version run
passed. Native launch/directory inheritance and live-window CLI/RPC remain pending.

Idle compression cancellation now propagates to queued background clones through
a shared cancellation guard owned by the UI task. Workers check it before trying
the engine lock and again after obtaining it, so focusing a terminal, opening
Find or dropping the view skips queued work. A step already past the final check
can still complete its bounded call. Two focused tests passed for cancellation,
nonblocking busy-lock handling, changed history activity and weak ownership;
workspace compilation passed. Native scheduling and latency acceptance remain
pending.

Equalize Pane Sizes is now available through View and the command palette. It
resets sibling weights at every split to equal shares, cancels divider dragging
and exits pane zoom. Tree topology, leaf order, focused terminal and sessions
are retained; saved layouts naturally record the updated weights. A focused
nested-layout test covers preserved axes/identities and equalized weights.
Native command routing, resizing and visual acceptance remain pending.

Compression latency instrumentation now measures each bounded call and the first
post-compression scroll-to-top plus frame extraction. The focused 1/4/16 release
run needed 11 calls per terminal; maximum calls were 0.050/0.062/0.063 ms and
maximum history revisits 0.143/0.151/0.149 ms. Revisited frame contents were checked.
Raw timings and scope limits are recorded in benchmark docs. These observed
engine-only maxima do not cover contention, active PTY pressure, shaping/GPU
presentation or UI scheduling, and are not worst-case responsiveness guarantees.

Automatic idle compression is now wired for unfocused terminals with no Find
worker open. Output updates a timestamp and shares one pending task rather than
creating a timer per chunk. After an initial two-second delay, a background probe
captures Ghostty's activity token and waits another two quiet seconds before the
first step. Changed history activity restarts that delay; pending bounded steps
yield 16 ms, and a busy engine lock retries after 50 ms without blocking on it.
Focus or opening Find cancels scheduling. Completed/unsupported passes stop, with
a final UI check retaining output wakeups that raced completion. Compression work
uses a weak terminal handle, so it cannot retain a closed session; an already
dispatched bounded step may finish during cancellation. No compression redraws
are requested. Compilation and the focused activity/busy-lock/teardown test
passed. That test showed Ghostty's token does not change for every active-screen
write, hence the separate UI output-idle timestamp. Native scheduling, cancellation,
decompression latency and whole-app memory/idle-CPU measurements remain pending.

Added the bounded incremental Ghostty compression API and a focused history
preservation test. The test passes for copied Unicode scrollback before/after
compression and later output. The engine benchmark now measures post-compression
storage outside parser timing. A local 1/4/16 run reduced reported resident page
bytes from 4,096,000 to 470,684 per terminal (88.5%) for repetitive output; total
sequential compression passes took 0.284/1.115/4.838 ms. Raw data and limits are
documented in benchmarks. Automatic UI idle scheduling, per-step latency and
decompression interaction measurements remain pending; no whole-app memory or
responsiveness claim follows from this engine-only result.

Custom workspace icon and hex-color editors now use the existing inline metadata
field through File and the palette. Icons accept one bounded grapheme; colors
accept exactly six ASCII hex digits with an optional #. Empty input clears the
setting. Invalid input preserves the previous value and leaves the editor open
with an explanation. Undo/Redo and IME support come from the shared TextField;
saved-layout validation and workspace movement retain these values as before.
Workspace compilation passed. Native editor/visual acceptance, graphical pickers
and automatic persistence remain pending.

Workspace glyph icons now have View-menu and palette choices (Terminal, Code,
Build, Cloud, Notes, None), a fixed-width sidebar slot, and manual layout
persistence. Whole-workspace detachment retains the icon. Old files default to
None; imported values must be one nonblank grapheme, at most 64 bytes, without
control characters. Rendering uses text glyphs with no image asset cache.
Workspace compilation passed. Focused persistence coverage checks old files,
an emoji sequence round-trip and invalid values; native glyph appearance/menu
acceptance and a custom icon picker remain pending.

The release engine benchmark now records Ghostty resident/virtual/image bytes
after timing. A focused 1/4/16-terminal run completed: filled page storage was
4,096,000 bytes per terminal (3.91/15.63/62.50 MiB totals), with no image storage.
Throughput was 258.3/262.9/260.2 MiB/s and snapshot p99 0.160/0.141/0.141 ms in this
single sequential fixture. Raw JSONL and interpretation are recorded in the
benchmark docs. This establishes engine-storage scaling for this workload, not
whole-app memory, concurrent PTY scaling, input-to-pixel latency or a release gate.

Opt-in render-metrics dumps now include Ghostty memory accounting per terminal:
primary/alternate resident page bytes, reserved virtual bytes, compressed bytes,
image bytes and compression support. Values are queried only when a metrics dump
is requested, under the existing engine lock; no idle sampler or retained history
is added. Missing query results serialize as null. These figures exclude Rust
snapshots, fonts, GPU resources and other process allocations; compressed bytes
are already included in resident bytes, while image bytes are separate. A focused
engine test passed for primary allocation, lazy alternate-screen allocation and
screen preservation, and workspace compilation passed. This adds measurement
capability; it does not establish a new whole-app memory or latency result.

Text fields now support Option+Left/Right and Option+Backspace/Delete on macOS,
with Ctrl equivalents elsewhere, for Unicode word-segment navigation/deletion.
Shift extends from the existing anchor; whitespace is skipped between segments
and punctuation/emoji remain selectable segments. Word deletion participates in
the bounded undo history and expands endpoints to whole graphemes. The focused
test passed for combining accents, emoji, whitespace deletion and selection
anchors. Native platform shortcut acceptance remains pending.

UI text fields now have Undo/Redo in Edit with platform text-editing shortcuts.
History retains at most 32 prior states across undo/redo, including UTF-16
selection and direction; new edits discard the redo branch. With the existing
4 KiB input cap, retained text is bounded near 128 KiB per open field and is
released with that field. Composition records its initial state once and blocks
Undo/Redo while marked text is active. A focused test passed for Unicode text
and selection restoration, redo, branch invalidation and the 32-state bound.
Native IME/clipboard/shortcut acceptance remains pending. A fresh native UI
attempt still found the Mac locked; code work continues independently.

UI text fields now support double-click Unicode word-segment selection and
triple-click Select All. Cut is available in Edit and through Cmd+X on macOS or
Ctrl+X elsewhere, scoped to text fields; empty selections leave the clipboard
untouched and active composition is not cut. Multi-click gestures cancel the
single-character drag anchor. A focused word-selection test covers combining
accents, emoji sequences, whitespace, UTF-16 positions and end-of-document clicks.
Native multi-click, clipboard and IME acceptance remain pending.

Persistent text fields now support mouse drag selection and Shift-click extension
with a retained anchor and reversed-selection tracking. Click/drag hit testing
snaps to the nearest shaped grapheme boundary, matching keyboard behavior for
combining text and emoji. Release inside or outside the field ends the gesture;
keyboard edits and composition cancel pointer state. Active composition is left
to the platform input method. This uses pointer events and the existing caret
scroll positioning, adding no timer. Native drag/outside-release, horizontal
scrolling and IME interaction acceptance remain pending.

Find, palette and workspace text fields now move the caret and delete by Unicode
grapheme clusters rather than scalar values. Combining accents, emoji modifiers,
ZWJ sequences and flags stay intact during arrow movement, Shift selection,
Backspace and Delete. Keyboard deletion expands partial-cluster selections;
platform IME replacement ranges retain their existing UTF-16 semantics. The
already-resolved unicode-segmentation crate is now a direct desktop dependency.
All seven focused composition tests passed, including the new cluster cases and
existing surrogate/IME replacement tests. Native IME, pointer selection and
accessibility acceptance remain pending; terminal grid rendering is unchanged.

Find match navigation now has Edit-menu and palette commands plus Cmd+G /
Cmd+Shift+G on macOS and F3 / Shift+F3 elsewhere. Bindings target terminal/find
contexts, and invoking navigation with Find closed opens its input. Existing
worker navigation coalescing handles commands received while searching. Find-bar
arrow/close targets now reuse the 28-pixel pane-control styling with hover labels.
Code inspection confirmed refreshes remain event-driven on output/resize, with
no idle polling added. Workspace compilation passed with the scoped bindings;
native shortcut routing and visual acceptance remain pending.

Clear Scrollback is implemented in Edit and the command palette. It temporarily
sets Ghostty's history retention to zero, then restores the queried byte limit;
no bytes are injected into the parser or PTY. The UI cancels selection/autoscroll
and closes Find before history removal. A focused engine test passed for removing
history while preserving visible text, clearing selection, restoring future
history retention and continuing an escape sequence split across the clear.
Native command routing, alternate-screen behavior and visual acceptance remain
pending; this command does not clear the current screen or restart the shell.

Automatic zsh integration now has an end-to-end macOS PTY check using the real
Session and Ghostty engine. It verifies initial directory reporting, changing
into a path containing spaces/Unicode/percent, output arriving through the PTY,
semantic prompt navigation through generated scrollback and normal shell exit.
The fixture uses isolated startup files and the same wrapper preparation as
production. Its first run exposed an expected-path mismatch between macOS /var
and /private/var; canonicalizing the fixture path resolved it, and the check
passed. This proves the local shell-to-engine path, not native UI rendering or
other-shell/platform support. No broad regression suite was repeated.

New Unix zsh sessions now load the integration automatically. Embedded scripts
are written to a private temporary directory retained by the session, and startup
wrappers source existing user files in order, carry forward ZDOTDIR changes, then
restore the user's directory before .zshrc and normal .zlogin processing. RCS
disablement skips integration, and RMUX_SHELL_INTEGRATION=0 bypasses the wrapper.
No user startup files are modified. Workspace compilation and a focused real-zsh
startup test passed for unset/custom ZDOTDIR, .zshenv redirection, login file order,
RCS opt-out, loaded hooks and temporary-directory cleanup. The prior hook test
covers emitted markers, encoded paths and command status. Global startup files
that override ZDOTDIR before the wrapper, unusual prompt frameworks, Linux runtime,
native UI behavior and other shells remain unverified or unimplemented.

Added sourceable zsh integration with OSC 133 prompt/input/output/completion
boundaries and OSC 7 local directory reports. It uses built-ins only, appends
existing hook arrays, guards duplicate sourcing and wraps the prompt with one
zero-width input marker. Directory bytes are percent-encoded, including UTF-8
and control characters. The script activates only for interactive rmux zsh and
supports opt-out before sourcing. No user startup files are edited; automatic
loading remains pending. macOS packaging now copies the script into Resources.
Syntax checking and a focused real-zsh run passed for hook preservation,
idempotence, prompt/output markers, exit status, encoded paths and opt-out.
The test uses no user rc files and is available as `mise run test-zsh-integration`.
Native prompt rendering, prompt-theme compatibility, automatic loading and other
shell integrations remain open. This does not establish full shell integration.

Previous/Next Prompt now route View-menu, palette and terminal shortcuts through
Ghostty's semantic viewport navigation. macOS uses Cmd+Up/Down; other platforms
use Ctrl+Shift+Up/Down. Navigation is relative to the viewport top and requires
shell markers; no matching prompt is a no-op. The engine owns prompt tracking,
with no separate history index or periodic work in rmux. A focused OSC 133 test
passed for forward/backward navigation, clamping to the earliest marker and
unmarked-output no-op; workspace compilation passed. Native shortcuts, shell
integration and dedicated last-prompt behavior remain pending.

Select All now uses Ghostty's selection derivation and installs the result under
the same engine lock, without copying scrollback until Copy is requested. The
operation resets any ongoing selection gesture and cancels UI autoscroll. Edit,
the palette, Cmd+A on macOS and Ctrl+Shift+A elsewhere route to the terminal;
text fields retain their existing select-all behavior. Copy retains its existing
16 MiB bound. A focused engine test passed for history plus Unicode, alternate
screen isolation, returning to the primary screen and clearing selection. Native
shortcut, highlighting and clipboard acceptance remain pending.

Move Workspace to New Window is implemented in View and the command palette.
The destination retains the workspace, pane and terminal entities, split weights,
tab order/selection, zoom state, last focused pane, name, description, color and
pin flag. Terminal focus subscriptions and workspace pane-event subscriptions
are rebound to the destination window. Workspace subscriptions now have explicit
ownership per pane and are removed on pane removal, replacing detached observers.
The source entry is removed only after destination creation succeeds; failure
rebinds input to the original window. No replacement sessions are spawned.
Workspace compilation and the macOS app bundle build passed. Native repeated
movement, focus/IME, unread behavior, source-window closure and multi-pane lifecycle
acceptance remain pending. Movement into an existing window remains absent.

Workspace colors are selectable by name through the View → Workspace Color
submenu and command palette: blue, green, amber, red, purple, cyan, or None.
Both routes dispatch the same parameterized action. A small square alongside
the sidebar name carries the accent, separate from the unread dot. Color changes
notify the window without touching terminal state or adding periodic work.
Manual layouts retain optional 24-bit RGB values, defaulting older files to None.
Workspace compilation and a focused snapshot test for missing color, valid RGB
extremes/round-trip and out-of-range rejection passed. Native visual/menu acceptance,
an arbitrary color picker, workspace icons and automatic persistence remain open.

Workspace descriptions now have a File-menu/palette editor, a secondary sidebar
line, and manual layout persistence. The editor shares the existing focus/IME
path used for workspace names, accepts empty text to clear the description, and
caps input at 256 Unicode scalar values after trimming/filtering controls.
Snapshots default missing descriptions to empty and reject control characters or
values exceeding 1024 bytes. Descriptions remain independent of terminal metadata.
Workspace compilation and a focused snapshot test for older files, Unicode
round-trip, length limits and control rejection passed. Native editing/layout
acceptance remains pending; icons, colors and automatic persistence remain open.

Tab detachment is implemented through View and the command palette. It creates a
new window around the existing terminal entity and PTY, then removes the source
tab and any emptied pane/workspace. Window focus subscriptions are replaced;
transient find, composition, pointer state and rendering cache are reset while
terminal output, session, font size and metadata remain attached to the entity.
No replacement shell is spawned. Failed window creation retains the source tab
and restores its focus wiring. Workspace compilation and the macOS app bundle
build passed. Native testing was attempted but the Mac remained locked; detachment,
last-window lifecycle and input/IME acceptance remain pending. Moving tabs into
an existing different window remains absent.

Workspace pinning is available through View and the palette. Pin moves the active
entry after existing pinned entries; unpin moves it to the start of unpinned
entries, retaining workspace/session identity and focus history. Keyboard reorder
stops at the group boundary, and drag reorder clamps to that boundary. Sidebar
rows label pinned entries. Snapshot pin flags default false for older files;
restore stably groups pinned entries first and resolves the saved active workspace
by identity after ordering. Workspace compilation and a focused pin round-trip/
backward-compatibility test passed. Native ordering, restored focus and visual
acceptance remain pending; cross-window workspace movement is still absent.

Toggle Program Status now opens a per-terminal detail region through View or the
palette. It displays all bounded records sorted by ID, child-depth indentation,
inherited app names, state/blocked kind, progress, title and message. The region
has its own scroll area and close control, capped at 220 logical pixels/45% of
terminal height. Mouse events are stopped before reaching the underlying terminal
handlers. Record snapshots are copied on opening and status revision changes only
while open, then released on close; hidden details do not maintain a second full
record list. Focus acknowledgement refreshes open details. Workspace compilation
passed; native sizing, scrolling, accessibility and focus acceptance remain pending.

Fixed a program-status exit race: once the PTY owner exits or reading completes,
late buffered working/blocked/idle reports no longer recreate live work. Final
done/error reports can still arrive and remain until acknowledged. Reaper cleanup
now precedes publishing shell-exit status. A focused engine test passed for exit,
late active reports, a final completion result, repeated cleanup and acknowledgement.
The latest macOS bundle built successfully before this follow-up fix. A native
status/notification smoke attempt was again blocked by the locked Mac; no UI result
is claimed and native verification remains pending.

OSC 7501 now has tab UI projection. The store returns one owned summary only when
its revision changes, prioritizing blocked, error, done, working, then idle; the
latest updated record wins ties. Inherited app names resolve at read time, and a
count reports additional records without copying the whole store. Tabs show state,
blocked kind, optional progress and additional-record count, with app/title/message
on hover. Unfocused blocked/error/done reports mark attention. Focus acknowledgement
refreshes the tab summary synchronously, avoiding stale completion labels when no
new PTY bytes arrive. Workspace compilation and a focused priority/inheritance/
revision test passed. Native labels, hover accessibility, attention and lifecycle
acceptance remain pending; the UI does not yet expose every hierarchical record.

OSC 7501 callbacks now populate the record store from Ghostty-validated reports,
with bounded string copies and removal of invisible display-direction formatting
from titles/messages. Identity strings are rejected rather than truncated if
over-limit. Semantic prompt-start callbacks remove transient states; PTY child
reaping and reader completion also perform transient cleanup. Terminal focus
acknowledges done/error records. Reset is handled by Ghostty's root clear callback.
Workspace compilation and all four focused program-status tests passed, including
actual OSC decoding, blocked kind/base64 message, inheritance, prompt cleanup,
subtree clear, reset and a nonempty support-query reply. UI status projection is
not yet connected. Native process-exit ordering, focused-terminal completion and
full reply-content acceptance remain pending.

OSC 7501 storage groundwork now implements whole-record replacement, hierarchical
clear, dynamic nearest-ancestor app inheritance, prompt/exit cleanup and terminal
acknowledgement cleanup. It retains at most 64 records (the protocol's allowed
lower bound), evicting the least recently updated ID. Repeated updates refresh
eviction order; missing clears do not increment the revision. Three focused tests
passed for replacement/inheritance/subtree boundaries, lifecycle cleanup and
bounded eviction. The rules come from the pinned libghostty-vt terminal.h contract.
This is a tested storage component only: the callback, prompt/exit wiring and UI
are not yet connected, and rmux does not advertise OSC 7501 support yet.

OSC 9;4 progress reporting now flows from Ghostty's callback through the shared
metadata read into tab labels. Running percentages, error, paused and indeterminate
states are distinguished; indeterminate uses a static Working label rather than an
animation. Callback state is a small Copy value with no allocation, and unchanged
reports do not notify the UI. Remove reports and terminal reset clear progress.
Workspace compilation and a focused protocol test passed for running/error/paused,
indeterminate, remove/reset and separation from BEL/desktop notifications. Native
tab appearance remains pending. This does not implement program-status OSC 7501
or the CLI/RPC status/progress/log interfaces.

Empty-source cleanup during tab transfers no longer briefly focuses a surviving
source pane. Layout removal is now separate from focus selection: explicit close
selects a survivor, while same-workspace moves, cross-workspace moves and pane joins
remove source ownership without changing focus. Their destination selection stays
the only focus transition. This prevents a neighboring unread terminal from being
acknowledged merely because a different terminal was moved. Split collapse and
WorkspaceEmpty lifecycle events remain shared. Workspace compilation passed;
native unread-preservation regression verification remains pending.

Notification cards now have Dismiss controls. Dismissal checks the displayed
message revision before acknowledging it, so a stale card cannot dismiss a newer
OSC message. It hides that revision and clears the terminal's attention marker
without focusing it; later revisions appear again. The click stops propagation
to the card's focus action. Dismissal state stays with the live terminal and does
not add message storage. Panes containing unread tabs now show an amber border;
non-benchmark panes reserve the same one-pixel border when read, avoiding geometry
changes when attention arrives. Benchmark geometry is unchanged. Workspace
compilation passed; native dismissal, ring appearance and focus acceptance remain
pending. Dismissal acknowledges the terminal's aggregate attention, including BEL.

View → Toggle Notifications and its palette action now open a right-hand panel
listing the latest OSC message for each live terminal in the window. Cards show
workspace/terminal labels, unread state, title and body. Clicking resolves the
workspace/terminal entity IDs against current state, selects the matching tab,
updates zoom if needed and focuses it without visiting other tabs first. Closed
terminals disappear; moved terminals are found under their current workspace.
Panel content is collected only while open, using existing metadata notifications
for updates. No separate retained history or background timer was introduced.
The panel is 340 logical pixels, capped at 45% of the window, with its own scroll
region and close control. Formatting and workspace compilation passed; native
layout, message readability and click/focus acceptance remain pending. The panel
does not persist or provide native OS notification delivery.

Ghostty's desktop-notification callback now receives OSC 9 and OSC 777 requests.
The engine copies bounded title/body input (256/1024 bytes before lossy UTF-8
conversion), strips control characters and keeps only the latest Arc-owned message
and revision. UI metadata fetches share the engine lock and only clone changed
messages; coalesced PTY updates mark unfocused terminals unread through existing
tab/workspace markers and next-unread navigation. No OS service, timer or growing
history queue was added. Message display/history and OSC 99 remain absent.
Workspace compilation and a focused engine protocol test passed for OSC 9/777,
unchanged revisions, distinct BEL handling, OSC 9;4 progress exclusion, bounded
message copying and owned-message lifetime. The initial oversized OSC test was
discarded by Ghostty's 2048-byte parser buffer; the fixture now exercises our cap
within that parser limit. Native marker acceptance remains pending.

Last Workspace now returns to the previous visited workspace, then toggles back
on repeated invocation. View and the command palette expose the action without
assigning a new default shortcut. History stores one entity ID per window (no
retained workspace/session), and explicit switches, new-workspace creation,
next-unread navigation and tab transfers update it. Sidebar reordering leaves the
identity intact; closing the remembered target clears it. Existing remembered-pane
focus is restored by the common selection path. Workspace compilation passed;
native navigation acceptance remains pending. Full pane/tab focus history and
last-prompt navigation remain unimplemented, so W12 is only partially implemented.

New terminal tabs and splits now inherit the active source terminal's cell metrics,
so changing font size does not reset readability when adding adjacent work. New
workspaces retain the default size; benchmark splits explicitly retain default
metrics to keep their established fixture comparable. Workspace compilation passed.
The native zoom/grid check was attempted after a successful bundle build, but CUA
reported that the Mac was locked and automatic unlock failed. No native zoom
result is claimed; zoom/selection/IME and restored-size interaction checks remain
pending until the desktop is available. Implementation work can continue independently.

Layout snapshots now record each terminal's font size and apply it when creating
restored views, before their first paint. The optional version-1 field defaults to
14 for older files; validation rejects nonfinite and out-of-range (8–40) sizes
before restore starts shells. The five focused snapshot tests passed, including
font-size round trip, mixed old/new tab fields, invalid size rejection and atomic
save preservation. Native saved-layout zoom appearance remains pending. This
supersedes the earlier font-zoom slice's persistence limitation.

Per-terminal font zoom now supports Increase/Decrease/Reset actions through the
Edit menu, command palette and terminal-context Cmd+=/-/0 (Ctrl equivalents on
Linux/Windows). Sizes are bounded to 8–40 and reset to 14. Cell width/height scale
from the existing 8.5×20 default geometry. Shared CellMetrics now drive terminal
row shaping, backgrounds, decorations, cursor, PTY rows/columns, mouse reporting,
selection, wheel scrolling and IME preedit placement/hit testing. Row-cache keys
include cell metrics so font changes reshape retained rows; drag autoscroll ends
on zoom. Text-field UI remains at 14. Terminal entities retain zoom when moved;
new terminals use defaults, and layout snapshots do not yet persist font size.
A focused metrics test and macOS bundle build passed after compile fixes. Native
zoom/selection/IME, font-family configuration and DPI acceptance remain pending.

Palette search now supports per-word subsequence matching (for example `splt blw`
finds Split Below). Contiguous matches rank before fuzzy matches; fuzzy candidates
prefer fewer skipped characters, then earlier matches, with original command order
breaking ties. Empty queries retain original order. Normalized titles are cached
only for the lifetime of the open palette; query changes perform ranking without
background workers or idle polling. Matching walks Unicode scalar values rather
than slicing at arbitrary byte offsets; accent folding and locale collation are
not implemented. Two focused tests passed for ranking, word order, repeated-letter
rejection, empty queries and Unicode subsequences. An initial ranking fixture did
not contain the requested subsequence and was corrected. Native fuzzy-search
interaction acceptance remains pending.

Sidebar workspace rows now accept terminal-tab drops from another workspace in
the same window. Hover acceptance validates the source workspace, pane and terminal
IDs against current entities; cross-window and same-workspace drops are rejected.
The transfer method now accepts explicit source identities, so dragging an inactive
tab moves that tab rather than the currently active terminal. Palette and next-
workspace actions resolve their active terminal before calling the same method.
Existing workspace-row drag reordering remains on its own branch. Weak owner
handles in drag closures avoid retaining the window root. Formatting and workspace
compilation passed; native drag interaction remains pending. No background work
or new idle polling was added.

The command palette now includes named tab-transfer destinations for every other
workspace in the current window. It captures names/positions on open, retains
stable workspace entity IDs and resolves the destination at execution. A removed
target or the current workspace is a no-op. The Next Workspace transfer action
uses the same transfer method. macOS bundle compilation passed.

Native macOS debug targeted-transfer smoke passed: create three workspaces, set
`RMUX_WORKSPACE_MARK=alive` in Workspace 1's only terminal and print PID `75454`.
Search `move tab workspace 3` in the palette and execute the named result. Workspace
1 disappears, Workspace 3 remains selected (now sidebar position 2), and its pane
has the original destination tab followed by the selected transferred tab. Printing
the marker/PID returns `alive` and `75454`. This verifies the last-tab/empty-source
path and a nonadjacent destination for this fixture. Multi-pane, zoomed destination,
duplicate-name and cross-platform acceptance remain pending; cross-window detachment
is not implemented.

Move Tab to Next Workspace now transfers the active terminal entity into the
next sidebar workspace's remembered pane, wrapping at the end. It preserves the
tab label and session, appends after existing destination tabs and switches focus
directly to the moved terminal. A zoomed destination shows the target pane. Empty
source panes collapse through existing layout removal; an emptied workspace is
removed through the existing WorkspaceEmpty subscription. A one-workspace window
is unchanged. The action is available through View and the command palette.
Workspace compilation passed; native multi-workspace continuity, last-tab removal
and restore acceptance remain pending. Cross-window detachment is still absent.

Join Pane into Next appends the focused pane's tabs to the next pane in traversal
order (wrapping), preserving their order and active terminal. Batch transfer
rebinds metadata subscriptions and selects only the original active terminal,
avoiding focus/attention acknowledgement for every intermediate tab. The empty
source split collapses; zoom and divider drag end. Single-pane invocation is a
no-op. The action is in View and the command palette. macOS bundle build passed.

Native macOS debug smoke passed for a break/swap/join sequence: create a second
tab, set `RMUX_MOVE_MARK=alive` and print shell PID `74420`; use the palette to move
the tab into a right split, swap the focused pane to the left, then join it into
the next pane. The UI returned to one pane with two tabs, the moved tab selected.
Printing the marker and PID again returned `alive` and `74420`, confirming shell
continuity through this sequence. Focus styling followed the terminal at each
step. This does not cover multi-tab batch order, unread markers, below splits,
restored layouts or other platforms. No performance conclusion is drawn from
the debug Metal HUD. Full W09 acceptance remains pending.

Move Tab to Split Right / Below now moves a multi-tab pane's active terminal into
a new adjacent pane. The new pane retains the existing TerminalView, session,
label and terminal state; no shell is spawned or shutdown handle added. The
source tab is removed only after the layout accepts the destination. Traversal
order, metadata subscriptions and focus are updated; zoom and divider drag end.
A single-tab pane is unchanged. Both actions are in View and the command palette.
Formatting and the macOS bundle build passed with existing dependency warnings.
Native session continuity and interaction acceptance remain pending.

View → Swap Pane with Next and its command-palette action exchange the focused
pane with the next leaf in layout order, wrapping at the end. The operation
retains pane/tab/session entities and focus, preserves split axes and weights,
updates the traversal vector used by cycling and layout snapshots, cancels any
divider drag and exits zoom. A single pane or missing focused pane is a no-op.
The focused nested-layout test passed, covering geometry preservation and missing
targets. Native focus/session continuity after a swap remains pending; break/join
operations are still absent. No shortcut was assigned for this new action.

The command palette now includes workspace-name results with sidebar positions
to distinguish duplicate names. Selecting a result resolves the stored workspace
entity identity against the current window before switching, so reordering cannot
redirect it to another workspace and a removed target becomes a no-op. Results
are captured when the palette opens; native workspace-search acceptance remains
pending. Formatting and workspace compilation passed with existing dependency
warnings. This adds no background search worker or idle polling.

The feature ledger now records 20 families as `in_progress`, each with a partial
implementation description, source paths and remaining gaps. The other 112 remain
`not_started`; no family or phase is accepted. The roadmap defines this partial
status explicitly and the inventory no longer claims all implementation is absent.
JSON count and referenced source-path checks passed.

File → Close Workspace and the command palette now close the active workspace as
a unit. Explicit close and last-pane removal share one removal path: discard the
workspace's owned views/subscriptions, select a surviving workspace or close the
window when none remain. Terminal session drops use the existing asynchronous
reaper; no new process-killing mechanism was added. Bundle compilation passed;
native multi-pane close/cleanup verification remains pending. No new shortcut was
assigned, avoiding changes to the current Close Tab/Close Window bindings.

Workspace rows now show the focused/remembered pane's active terminal directory
as secondary text when the shell reports a valid local OSC 7 URI. Ghostty's PWD
callback increments a revision; metadata reads decode the path only after a
change, under the existing shared metadata lock. Terminal reset invalidates both
title and directory. UI labels filter control characters; shell inheritance keeps
the original path bytes. No filesystem polling or shell startup edits were added.
A focused engine test passed for OSC reporting, unchanged ordinary output and reset;
macOS bundle compiled. Native sidebar directory switching is pending. Git branch,
remote-directory metadata and automatic shell integration remain unfinished.

Startup redraw investigation: after a fresh CUA launch, invoking the window's AX
Raise action displayed the shell prompt and current title without clicking the
terminal. GPUI 0.2.2's macOS `window_did_change_occlusion_state` stops its display
link for occluded windows and restarts it when visible; CUA documents background
app launch. Together these support an automation visibility explanation for the
previous stale captures, rather than evidence that normal startup requires a
terminal click. Future native smoke tests should raise the app before judging
rendering. No forced redraw timer or framework patch was introduced. This does
not establish a startup latency measurement or cover every foreground launch path.

Native command-palette smoke passed: Cmd+Shift+P opened the overlay, typing `split`
filtered to two actions, Down selected Split Below, and Enter dismissed the palette
and split the original pane below, focusing the new terminal. The new pane's blue
accent and the other pane's muted accent were visually confirmed. Initial launch
again needed a click before content redraw resumed; that observation remains open.
Palette rows now also show shortcut labels resolved from GPUI's highest-precedence
binding in the original focus context, captured once when opening the palette.
This avoids a second hard-coded shortcut map. The shortcut-label addition builds
but has not had a native visual check.

A first command palette now opens with Cmd+Shift+P (Ctrl+Shift+P elsewhere) and
View → Command Palette. It lists 22 implemented actions, filters case-insensitive
title substrings for every query word, supports wrapped Up/Down selection,
Enter/click execution, Escape/outside-click dismissal and scroll-to-selection.
The shared text field supplies IME/editing, with arrow navigation disabled during
composition. Dismissal restores the previous focus before dispatching actions,
so pane actions target their original terminal. The overlay occludes mouse input
to content underneath. Bundle compilation passed; native rendering/action routing
are pending. Fuzzy ranking, action shortcut labels, dynamic enablement, custom
commands and workspace/file search remain unfinished.

Selected terminal tabs now distinguish keyboard focus: the focused pane in the
active window uses a bright blue accent and brighter text, while other panes keep
a muted selected-tab accent. Focus-in/out (including descendant find fields) and
window activation invalidate pane metadata so the indicator follows keyboard
navigation. This changes color only, without resizing the terminal viewport.
macOS bundle builds passed; native visual acceptance is still pending.

The PTY update path now reads bell/title metadata under one engine mutex lock,
instead of two, and emits at most one tab metadata event when both change in the
same coalesced update. Unchanged titles retain the allocation-free revision check;
title retrieval errors do not suppress bell delivery. This is a scoped overhead
reduction, not a new throughput/latency measurement. macOS bundle compilation
passed; existing native bell evidence predates this refactor.

View → Next Unread Terminal (Cmd+Shift+U; Ctrl+Alt+U elsewhere) now navigates to an
attention-marked terminal across panes, hidden tabs and workspaces in the current
window. Search starts in the active workspace, then wraps in sidebar order; pane
and tab ordering break ties rather than notification timestamps. The selected
workspace/tab are revealed, saved pane focus is updated, and existing pane zoom
follows the target. Navigation avoids briefly focusing an unrelated tab and thus
acknowledging its bell. No unread target is a no-op. Desktop check and bundle build
passed; native navigation is pending. Cross-window notification navigation and a
persistent notification history are still unfinished.

Workspace rows now aggregate terminal attention through pane/workspace metadata
events. Hidden workspaces retain an amber indicator until their requesting terminal
is focused; tab removal/transfer also invalidates the aggregate. No polling is
added. Native smoke passed: `sleep 3; printf '\a'` in workspace 1, then Cmd+2,
produced a sidebar marker after the delay; Cmd+1 returned and cleared it. Numbered
workspace selection and rendered shortcut hints were observed for positions 1/2.
Initial post-launch captures were stale until clicking the window; subsequent
redraws worked. Startup activation/redraw delay remains an unresolved observation,
not a proven renderer deadlock. Other attention scenarios remain unverified.

Terminal BEL now increments a wrapping counter in Ghostty's synchronous callback.
The existing coalesced PTY update path reads that counter and marks an unfocused
terminal as needing attention; pane tabs show an amber dot and clear it on active
window terminal focus. This adds no polling, timer or event queue per bell.
Tab title/attention updates share a `TabMetadataChanged` event. A focused engine
test covers multiple BELs, terminal reset and OSC title terminators (which must
not ring). Desktop compilation passed. Native attention rendering/clearing is
unverified. Workspace-level badges, notification center, sounds, OS notifications
and agent notification protocols remain unfinished.

Layout restore now allows one pending operation per originating window, shows
an opening status with Cancel, and uses an owned cancellation token. Closing the
originating window cancels its preparation; checks run before file access, between
shell launches, after preparation and before publishing the new window. Cancelling
retains the pending-operation guard until the worker returns, avoiding overlapping
replacement batches. Already prepared sessions drop through normal cleanup.
The focused cancellation test verifies owner-drop cancellation before file access;
desktop check passed. In-flight OS reads/spawns are not forcibly interrupted, and
native cancellation plus mid-batch process cleanup remain unverified.

Direct workspace selection now binds Cmd+1–8 to the first eight workspaces and
Cmd+9 to the last (Ctrl+Alt+1–9 on Linux/Windows). Missing numbered workspaces are
ignored. Sidebar rows show corresponding position hints, recalculated after
reordering. A scroll handle reveals the active workspace after selection,
creation, restoration and keyboard reordering without continuous scrolling or
polling. Desktop compilation passed; native key routing and hint rendering remain
unverified.

Native layout restore smoke passed for a two-workspace/four-shell fixture opened
through the macOS picker: `Restore Beta` became active, its left/right split was
1:2, sidebar width was 260px, and the saved second tab in the left pane was focused
with a `docs` prompt while the right pane showed `crates`. This exposed a hidden
selected tab in a narrow tab strip. Pane tab strips now use GPUI's scroll handle
to reveal the active item after restore, selection, reorder, close, transfer and
title changes. The correction builds; native tab-reveal verification is pending.
The fixture is in `target/restore-smoke.json` (local disposable build output).

File → Open Layout in New Window… now uses the native file picker, bounds reads
to 2 MiB plus one byte, validates the saved schema, and prepares all fresh shells
on the background executor. It creates a new window only after preparation
succeeds, with saved workspace names/order, active pane/tab/workspace, nested split
weights, tab labels, reported directories and sidebar visibility/width. Partial
preparation is dropped on error; existing windows are not replaced. Restored panes
reuse the normal close/move subscriptions and shutdown registry. Desktop check
and macOS bundle build passed. The native restore case above passed; resource
rollback and large-layout responsiveness remain unverified; automatic saves/recovery remain
unfinished. Saved directories that cannot start a shell fail the restore rather
than silently substituting another directory.

Pane zoom is available through View → Toggle Pane Zoom and Cmd+Shift+Enter on
macOS (Ctrl+Shift+Enter elsewhere). It mounts only the selected pane while retaining
the original split tree, weights and terminal entities. A visible restore strip
returns to the layout; pane cycling retains zoom and selects the next pane.
Adding/closing a pane restores the split view. Directional focus is currently
inactive while zoomed; zoom state is not exported in layout snapshots. A native
smoke check verified the shortcut expanding the right pane and clicking Restore
returning to two panes with the marker output preserved. Process identity was not
independently measured, and cycling/structural changes while zoomed remain untested.

The workspace sidebar now has a draggable 5px divider, a 160–480px preferred-width
range, double-click reset to 200px, and viewport clamping that reserves terminal
space in narrow windows. Drag events are captured before terminal mouse handlers,
without timers or background polling; hiding the sidebar cancels a drag. Save
Layout records the preferred width with a backwards-compatible default for earlier
version-1 files. Snapshot tests cover round-trip, invalid widths and legacy files.
Native drag/reset interaction passed: the divider moved from x=404 to x=544
in a 2× screenshot, then double-click returned it to its original position.

UI parity with cmux is an explicit active-goal requirement: spacing, hierarchy,
clean visual design and shortcuts must be compared against the reference rather
than inferred from feature availability. The current prototype is not visually
accepted. A read-only native cmux comparison confirmed right-aligned pane controls,
an accent above the selected tab, and an active-workspace header. The first pass
adds a 32px tab strip with fixed 28px new-tab/split-right/split-down controls,
shortcut tooltips, vector split glyphs and a selected-tab accent. The prototype
shortcut banner is replaced by the current workspace name. Native testing verified
both split buttons, including clicking Split Down in an unfocused left pane while
the right pane had focus: only the left pane split. Tooltip text and the tab accent
were visually observed. The workspace-header replacement was also visually checked
in the zoom/sidebar smoke run. Browser controls, sidebar metadata,
titlebar integration, accessibility and full visual parity remain unfinished.

Layout persistence now has a versioned JSON schema and File → Save Layout… using
the native save picker. It captures the current window's workspace names/order,
active workspace/pane/tab, nested split proportions, tab labels and available
shell directories. Disk encoding/writing runs in the background and replaces the
destination through a synced temporary file. Limits cover versions, depth, tab
counts, file size, indices and finite positive weights. Format round-trip and
invalid-state tests passed; native file-picker interaction remains unverified.
Loading/restoring is now implemented as described above; automatic saves and crash
recovery are not implemented yet.
Snapshots describe fresh shells, not process or scrollback resumption. Non-UTF-8
directory paths currently produce an explicit serialization error rather than a
lossy saved path.

New tabs and splits inherit the active terminal's shell-reported current directory
through Ghostty's OSC 7 state and `Session::shell_in`. Directory reads happen only
when creating the new surface, without polling or per-output string allocation.
The local file-URI parser decodes percent escapes, supports Unix non-UTF-8 path
bytes, recognizes localhost/current host, and rejects remote hosts, UNC paths,
malformed escapes and embedded NUL. Absent/unusable reports use the existing shell
startup behavior; no shell files are modified. Two focused tests passed for URI
decoding/rejection and live OSC report/reset handling; desktop compilation passed.
Native shell inheritance and Windows path handling remain unverified. Automatic
shell integration installation and persisted directory restoration are unfinished.

The shared find/rename text field now supports Shift+Left/Right and Shift+Home/End
selection with a fixed anchor, reversed caret placement, and matching native
UTF-16 selection direction. macOS Command+Left/Right moves to the text edge.
A focused test passed for astral Unicode boundaries, shrinking/reversing a
selection and collapsing it without Shift; the desktop test target compiled.
Movement is by Unicode scalar, not full grapheme clusters; word motion and mouse
drag selection in text fields remain unfinished.

Tab drag now also moves an existing terminal entity into another pane in the same
workspace. The workspace coordinates removal/insertion, activates the moved tab,
and collapses an empty source pane. Title subscriptions are owned per tab,
removed from the old pane and installed in the destination; closed tabs also
release their observer. This avoids accumulating detached title observers across
moves. The PTY, scrollback, find state and terminal view are retained. Scope IDs
reject cross-workspace/window drops. Desktop compilation passed; native transfer
and process-identity verification remain pending.

Drag reordering is implemented for tabs within their pane and workspace rows
within their window, with a labeled drag preview and highlighted drop targets.
Drops resolve stable source/target entity IDs, preserve existing entities and
activate the moved item. Workspace drag is disabled while its rename editor is
open. A desktop compile check passed; native drag behavior remains pending.
Cross-workspace/cross-window moves, insertion-edge previews and drag autoscroll
remain unfinished.

Workspace rename is implemented through Cmd+Shift+R / Ctrl+Shift+R and File →
Rename Workspace. The sidebar opens an inline editor with the existing name
selected; Enter saves, Escape cancels, and both return focus to the remembered
terminal pane. Empty names retain the existing name; saved names are trimmed,
control-filtered and limited to 128 characters. Workspace selection updates the
native window title. Find and rename share a text-input entity with existing
Unicode/IME, paste/copy and select-all behavior. Desktop compilation passed;
native rename and post-refactor find behavior remain pending. Names are not yet
persisted across restarts.

Directional pane focus now uses current visible terminal bounds, preferring
aligned neighbors and then nearest facing edges/centers. Cmd+Ctrl+Arrow on macOS
and Alt+Shift+Arrow elsewhere navigate without wrapping; layout-order cycling
remains available. Native View menu entries expose the four directions. A focused
test passed for a resized nested layout, all directions, outer edges and an
unlaid-out pane; the desktop test target compiled. Native shortcut behavior has
not yet been checked.

Separate native window creation and closing are now implemented. Cmd+Shift+N /
Ctrl+Alt+N creates an independent workspace window; Cmd+Shift+W / Ctrl+Alt+W closes
the focused window. The app-wide shutdown registry remains shared, and closing
the last window takes the existing quit/cleanup path. Added native menus for the
main file, edit and layout actions, plus an explicit window title. One desktop
compile check passed. Multi-window native lifecycle validation, moving workspaces
between windows and window restoration remain outstanding.

A short macOS debug smoke check now confirms the basic find flow, match count and
highlight, Escape, tab creation/content retention, nested splits, divider dragging
and last-tab pane collapse. The test app exited. See
[the scoped native evidence record](benchmarks/p1-native-smoke.md) for the build
hash, scenarios and unchecked cases. This supersedes the earlier "native pending"
notes only for those specific interactions; no full feature or phase is accepted.

Per the user's latest direction, Windows validation is deferred and short feature
iterations take priority over broad validation. P1 layout work now proceeds while
P0 acceptance gaps remain visible. Nested row/column splits replace the flat
normal-mode layout; split right/below targets the focused pane, close collapses
empty branches, and surviving pane entities retain their sessions. Focus cycling
follows tree order. The existing equal-width benchmark fixture is preserved.
One focused layout test passes and compiles the desktop target. Sidebar workspaces
are now implemented with create/select/cycle actions, sidebar collapse, retained
workspace entities and last-pane focus restoration. Only the active workspace is
mounted in the visible layout; inactive terminals stay alive. An empty workspace
is removed, and the window closes only after the last workspace is gone. A single
desktop compile check passed for this iteration; native UI verification was not
run.

Per-pane tabs now support creation, selection, closing and keyboard reordering.
Only the selected terminal is mounted; hidden tab entities retain their sessions
and scrollback. Closing the last tab removes its pane and collapses its split.
Workspace order can also be changed by keyboard without recreating sessions.
The benchmark fixture hides tab chrome. Shell OSC 0/2 titles update tab labels
through Ghostty callbacks; unchanged titles do not allocate a new string or
notify the tab strip, and displayed titles are bounded to 128 characters. Full
terminal reset clears the title. A targeted test covers fragmented OSC input,
ordinary-output suppression, Unicode title bounds and reset; it passed, as did
the desktop compile check. Native tab UI and shortcut verification remain pending.
Cross-container drag movement and persistence remain unfinished. Windows
validation remains deferred per user instruction.

W03 divider resizing is now implemented for nested row/column splits. Dragging
updates only adjacent branch weights; pane and terminal entities stay intact.
Double-click equalizes the adjacent branches. Dragging clamps each adjacent
branch to 48 logical pixels when space permits (half the pair when it does not).
Closing a branch preserves surviving relative weights, and structural layout
changes discard any active drag. Benchmark dividers remain thin and noninteractive.
Two focused layout tests passed, including resizing bounds, surviving weights,
and nested collapse; this also compiled the desktop target. Native drag behavior
has not yet been verified, so W03 is not marked verified.

The already-running Linux desktop link attempt finished with missing target
libraries `xcb`, `xkbcommon`, and `xkbcommon-x11`. No further platform validation
or library installation was pursued in this iteration.

## P0 — in progress

### Latest P1 search iteration

Added an on-demand search engine wrapper around Ghostty's incremental search API.
Each step performs at most one bounded feed and one bounded tick. Feed, query
replacement, navigation and teardown serialize with the PTY parser; scan ticks
use copied data without holding the terminal mutex. Search exposes match counts,
selected index and pending status, refresh after terminal changes, and older/newer
navigation that scrolls to and highlights the selected match. Empty queries release
results. Queries are capped at 4096 bytes. Current matching follows Ghostty:
ASCII case-insensitive, otherwise byte-exact; regex and match-case controls are
not implemented.

Two focused engine tests passed for scrollback counts, bidirectional navigation,
query clearing, output refresh, Unicode across soft wraps and alternate-screen
transitions. The on-demand background worker is now implemented: latest-query
coalescing, bounded navigation, coalesced refresh/wakeup/result notifications,
revision-tagged results, and cancellation on drop. It waits on a channel when
caught up and checks cancellation between bounded scan steps. A focused worker
test passed for query replacement, navigation, output refresh and shutdown.
The find bar is now integrated into each terminal: Cmd+F / Ctrl+Shift+F opens it,
Enter and Shift+Enter navigate older/newer matches, and Escape or × closes it and
returns focus to the terminal. It shows selected/total counts and pending state,
refreshes after PTY output or terminal resize, and highlights the selected match.
The query field uses native text-input/IME hooks, UTF-16 range conversion, a
4096-byte limit, horizontal caret scrolling, paste/copy and select-all. A focused
persistent-editor Unicode/IME commit test and desktop compilation passed. Native
find interactions, full editing conventions, all-match highlights and search
performance remain unverified or unfinished; search parity is not claimed.

### Latest P1 selection iteration

Terminal selection now uses Ghostty's tracked gesture state and owned active
selection. Implemented drag selection, native double/triple-click word/line
behavior, Alt-drag rectangles, Shift override for mouse-reporting applications,
per-row highlight ranges, and Cmd+C / Ctrl+Shift+C clipboard copy. Extraction
uses the Ghostty formatter with soft-wrap unwrapping and a 16 MiB output limit.
Gesture/event handles are reused; clipboard text is allocated only on copy.
Fractional cell widths are normalized before passing integer geometry to Ghostty.
Two targeted tests passed for Unicode word/line copy, retained selection during
output, selection clearing/highlighting, fractional-width drag, rectangles and
release. Desktop compilation also passed. Native pointer/clipboard behavior,
keyboard selection and selection performance remain unverified or unfinished;
full selection parity is not claimed.

Selection edge autoscroll is now implemented using Ghostty's autoscroll gesture
ticks. Pointer capture keeps an active drag working outside its original pane.
A 30 ms one-shot timer is rearmed only while dragging beyond a vertical edge
with history available in that direction. Release, focus loss, pointer reentry,
gesture invalidation and reaching the history boundary stop the timer. Three
focused selection tests pass, including scrolling to both ends of history and
stopping; desktop compilation passed. Native pointer capture and timer behavior
remain unverified.

The macOS prototype builds and runs using mise-managed Rust 1.99.1, GPUI 0.2.2, portable-pty 0.9.0 and the official checksum-pinned libghostty-vt 1.0.1 library. Apple Metal Toolchain installation was explicitly approved by the user and completed. No Homebrew installation is part of setup.

Implemented:

- Owned Ghostty FFI wrapper with dirty-row snapshots and shared unchanged rows.
- PTY reader, bounded asynchronous input queue, coalesced redraw notifications and child cleanup on session drop.
- GPU text drawing with cell positions, colors, basic styles, Unicode clusters and a cursor.
- Faint text blending, strikethrough, overline, underline color and curly underline mapping. Terminal-requested bar/block/underline/hollow cursor shapes and explicit cursor colors are carried into drawing; block cursors repaint visible cell text in contrast and preserve hidden-text behavior.
- Bounded per-visible-row shaping cache: unchanged engine row identities reuse shaped text; changed rows rebuild. DPI/background changes invalidate the cache; pane movement applies offsets at paint time. Background spans are batched independently of text and include wide/blank cells.
- Interactive login shells, terminal resize, basic clipboard paste and side-by-side panes.
- Terminal-state-aware control keys through Ghostty's encoder, including application cursor mode and modifiers; bracketed paste with control-byte sanitization.
- Project-local library bootstrap and macOS application bundle tasks.
- Release bundles and an opt-in clean-shell benchmark workload (`RMUX_BENCH_PANES`), plus read-only macOS idle sampling scripts.
- Portable PTY pressure harness (`mise run bench-pty`) with 1/4/16 sessions, distinct response markers, per-request deadlines and parsed-output progress checks. Initial macOS results are recorded in [PTY baseline](benchmarks/pty-baseline.md); GUI latency and repeatability are not established.
- Opt-in fixed-size render diagnostics via `RMUX_RENDER_METRICS` and F12: pane paint/prepaint counts and CPU durations, row-cache reuse/rebuild counts, parsed bytes, notifications and focus state. No idle polling or automatic file writes. Workspace check passes; native metric capture remains pending. See [diagnostic procedure](benchmarks/render-diagnostics.md).
- Child exit status reporting, rejection of input after exit, and application shutdown when the last window closes.
- Focused-pane close and forward/backward focus cycling. Session construction now arms cleanup before spawning workers and starts the reaper first, so partial startup errors retain cleanup ownership. Already-reported exited children are not signaled again on normal session drop.
- Linux source bootstrap with mise-pinned Zig 0.17.0; successful x86_64 Linux GNU library cross-build on macOS. Windows checksum-pinned package bootstrap and DLL bundle scripts are present but not native-tested. Build scripts now resolve the project-local dependency on all target OSes.
- Linux/Windows shortcut and font defaults, plus Windows branches for PTY/ConPTY session tests. Native Windows/Linux desktop validation is still pending.
- Ghostty-backed scrollback navigation through wheel/trackpad, Shift+PageUp/PageDown and Shift+Home/End; a position marker appears in history. Accepted input returns to the live viewport without adding a history buffer or idle timer.
- Ghostty mouse encoder integration for left/middle/right press/release, motion and vertical wheel reports. Tracking modes and wire format come from terminal state; Shift+wheel overrides reporting for local history. Motion reports do not force the viewport to live output. Encoder/geometry state is retained so same-cell motion can be deduplicated.

Verification completed November 19, 2026:

- Workspace builds with build-time Metal shaders.
- Terminal parsing/style/Unicode/cursor/query-reply and real PTY output tests pass.
- A focused interactive-input-after-resize test passes.
- Input encoding tests cover application cursor keys, Ctrl+C, Shift+Tab, native-text fallback, paste newline conversion and bracketed-paste framing/sanitization.
- A shell exit-status test verifies nonzero exit reporting and input rejection after exit.
- Native window smoke check confirms a shell prompt, executed command output and a second pane via Cmd+D. Explicit interactive shell startup fixed the initially blank window.
- Cached-renderer smoke check confirms red backgrounds across wide/blank cells, combining characters, underlined trailing spaces, and reflow after a split. Workspace check and debug bundle build pass. A before/after throughput measurement has not yet been made; earlier benchmark numbers refer to the earlier binary.
- Scrollback tests cover history retained during new output and alternate-screen transitions. Native scrollback smoke check reached the oldest fixture output and returned to the live prompt on paste. Selection/copy, search and a draggable scrollbar are still pending.
- Seven terminal tests now pass. Mouse coverage checks disabled reporting, SGR press/release/wheel bytes, tracking-dependent motion suppression, repeated-cell deduplication, fractional-width cell coordinates, pixel reporting and mode disable. Workspace check passes. Native TUI mouse interaction, multi-button chords, horizontal wheel and mouse cursor-shape requests remain unverified or unfinished.
- The eighth terminal test exercises eight sequential create/drop cycles and verifies both shell exit reporting and release of the terminal state. Debug bundle build passes. Native pane-close/focus-cycle verification was attempted but the computer-use tool failed and reset its session; those UI actions remain unverified. Startup thread-failure injection, stubborn children/descendants, writer-thread retention and long-run resource plateaus still need coverage.
- Nine terminal tests now pass, including faint/style extraction, colored curly underline, hidden text, DECSCUSR cursor shapes, OSC cursor color and cursor visibility. Workspace check passes. New decoration/cursor drawing has not received a native visual check. Double/dotted/dashed underlines currently fall back to a single line; text/cursor blinking is not yet scheduled.

These are functional checks, not performance benchmarks. The debug Metal HUD is not a reliable baseline for release memory, idle rendering or input latency.

IME composition now retains UTF-16 selection and marked ranges, applies explicit
partial replacements, preserves surrounding pending text, and normalizes ranges
without splitting surrogate pairs. Unmarking commits pending text once; an empty
marked-text replacement cancels it. Four desktop unit tests pass, covering Japanese
composition, partial replacement, emoji offsets, commit and cancellation. The
desktop test target compiles and links with GPUI. `mise run test` now includes both
workspace crates; all 13 tests pass. Candidate bounds and point hit testing now
use the same shaped glyph positions as preedit rendering, converting between
UTF-8 byte indices and UTF-16 platform offsets. The caret follows the composition
selection; a background and selection highlight keep it readable over terminal
output. These native geometry/drawing changes still need an input-method UI check.
Multiline/wrapped preedit, bidirectional selection geometry, and reconversion of
already-sent terminal input remain unfinished.

Keyboard encoding now accepts press/repeat/release actions. A focused test checks
Kitty event reporting after protocol negotiation and suppression after restoring
legacy mode, including releasing Ctrl before the letter key. The desktop pairs
releases only with encoded presses, and releases outstanding keys when the pane
loses focus. Releases do not return the viewport to live output. Encoding uses a
stack scratch buffer, avoiding a heap allocation for suppressed releases. All 14
workspace tests passed, the additional modifier-release assertion passed, and
the final workspace check passed. Native release/focus transitions remain
unverified. Printable Kitty press reporting, physical/non-US key mapping and
window-deactivation cleanup were still open at that check; native text and IME still own ordinary
printable input.

Focus reporting now uses Ghostty's focus encoder when DEC mode 1004 is enabled.
Pane-focus and window-activation subscriptions share a deduplicated focus state;
losing focus releases tracked keys before sending focus-out, including when
switching away from the application. Focus reports and releases preserve the
scrollback viewport and add no polling or idle timers. A focused test verifies
disabled/enabled/disabled reporting and both focus sequences; all 15 workspace
tests pass. Native activation/pane-switch ordering remains unverified, as do
mouse-button release reporting on deactivation and input-method focus transitions.

Next short iterations:

1. Complete keyboard protocol integration for printable keys, key releases, non-US layouts and IME; control keys and bracketed paste now use Ghostty.
2. Measure release idle CPU, memory and output throughput at 1/4/16 panes with repeatable workloads.
3. Improve shutdown/error reporting and test shell lifecycle.
4. Validate Linux/Windows dependencies and rendering on those platforms.
5. Begin P1 workspace sidebar and nested split layout after the P0 gates are met.

Still missing: selection/copy, scrollback search, complete terminal styles, robust IME behavior, accessibility, complete mouse interactions, persisted workspaces and native validation of cross-platform shortcuts/packaging. Feature inventory acceptance criteria remain authoritative; scaffolding does not imply feature parity.

`mise run bench-engine` builds a release-only engine harness and emits JSON for 1/4/16 terminals. It uses 160×48 cells, 4 MiB scrollback budgets, styled Unicode output and periodic snapshots after warm-up. This isolates parsing/snapshot cost; it does not measure PTY throughput, input-to-display latency, CPU at idle or GPU memory.

An initial [release app measurement](benchmarks/app-baseline.md) now covers one/eight idle clean shells and basic shutdown. Idle CPU exceeded the provisional target and memory did not establish a stable baseline; P0 performance is still open. Before P1, add visibility/redraw instrumentation, resolve idle cost and validate the embedding path on Linux/Windows as required by the roadmap.

A subsequent [instrumented one-pane run](benchmarks/render-diagnostics.md) captured
zero additional terminal paints across 92 seconds and approximately 0.02% of one
core over a nested 60-second interval. This supplies native evidence that the
terminal can remain render-idle; it is not a controlled comparison with the earlier
high CPU observation. No speculative renderer patch was made. Multi-pane native
diagnostics, physical memory attribution and cross-platform gates remain open.

The same instrumented release then ran eight idle panes: zero additional paints
over 92 seconds, approximately 0.02% frontend CPU over 60 seconds, and physical
footprint 49.8M to 49.9M with a 275.4M startup peak. Native reverse focus cycling
and closing removed the expected pane and its shell; the other seven stayed
alive until normal quit removed all recorded processes. See the diagnostics
report for evidence and limitations. These observations do not establish latency
under GUI output pressure, long-run retention or cross-platform acceptance.

Row text batching is now isolated from GPUI shaping so its column/style decisions
can be checked without a native window. Two focused tests pass for color-run
boundaries, wide-cell tails, trailing blank trimming, underlined spaces and empty
decorated cell representations. Empty decorated cells retain a column in the
renderer, and batching stops at noncontiguous columns. Ghostty's erase-character
operation clears underline styling, so no erase-related rendering bug is claimed.
Native decoration appearance remains a separate verification requirement.

Synchronized output (DEC 2026) now retains the previous snapshot until Ghostty
reports the hold released. Parsing and PTY replies continue while held. A 250 ms
deadline clears an abandoned hold; the UI schedules at most one pending timeout
per pane and suppresses output-triggered redraw notifications during the hold.
There is no idle polling. The callback state shares the existing stable allocation
used for PTY replies. A focused test covers unchanged row ownership while held,
release, repeated-enable behavior, timeout recovery, a new hold after timeout,
resize and full reset. All 18 workspace tests pass. Native timer/presentation
behavior under a synchronized-output workload still needs verification; the prior
release idle measurements precede this change.

Native macOS synchronized-output smoke check passed using
`scripts/fixtures/synchronized-output.sh`: the baseline appeared first, then the
held text appeared while the producer was still in its ten-second no-output
interval, before the release line. The release line and shell prompt subsequently
appeared, and normal quit closed the test app. This verifies timeout-driven native
recovery without another PTY write, not the exact 250 ms presentation latency or
absence of partial frames within a normally released transaction. The debug
binary SHA-256 was
`70956955f2fa6f6eaa092f227ec3993709c72a241abfd8889e75527bea0f4d59`.

macOS pane teardown now uses a kqueue process/user-event wakeup. The reaper retains
exclusive child ownership until natural exit or a close request; on close it can
invoke portable-pty's owning-child termination path (SIGHUP, grace period, then
forced termination). The UI does not wait through that grace period, and no idle
polling was introduced. This avoids adding a delayed raw-PID kill racing with
reaping/PID reuse. Each live macOS session currently owns one additional kqueue
descriptor, marked close-on-exec. Existing session tests pass, plus a focused
macOS test verifies a shell ignoring SIGHUP terminates and releases terminal state
after pane drop. Linux/Windows retain their existing kill paths. Process-group
descendants and app-wide graceful shutdown that waits for all reapers remain open;
pane teardown does not prove those cases.

The rmux Quit action and last-window-close path now request all registered shell
shutdowns before invoking GPUI quit, then asynchronously await reaper completion
with a three-second upper bound. Requests are issued together; the UI remains
event-driven while waiting. Lightweight shutdown handles retain neither PTYs nor
terminal state, and finished handles are pruned when adding a session. This also
covers recently closed panes whose reapers are still working. The stubborn-shell
test now retains a shutdown handle and verifies completion plus terminal release.
All 19 workspace tests passed before that additional assertion; the strengthened
focused test passed afterward. Native application-quit verification is pending.
GPUI's final OS-driven shutdown hook only allows 100 ms, so externally initiated
termination that bypasses rmux's Quit action, forced termination, and descendants
remain outside this guarantee. Linux's existing SIGHUP-only path may reach the
three-second deadline for a stubborn child and still needs escalation support.

Native quit follow-up: debug binary
`df35a9fd10fc7aea635cd41a237de7a187274c1f4ad998c2d40a183d94600168`
exited normally with both clean test-shell PIDs removed. The attempted interactive
SIGHUP trap could not be confirmed because native automation returned inconsistent
window state. A subsequent fixture launch via bundle `SHELL` override actually
started `/bin/zsh -i -l`; it was also quit and its recorded PID removed. Neither
attempt verifies native stubborn-shell quit. The reusable
`scripts/fixtures/hangup-resistant-shell.sh` wrapper is syntax-checked and remains
available for a launch with a verified environment. No broad shutdown guarantee
is inferred from these normal-exit observations.

Linux now has a process-wakeup implementation using `pidfd_open`, `eventfd` and
blocking `poll` with no periodic timeout. It uses the same exclusive owning-child
reaper and termination escalation as macOS, rather than a cloned SIGHUP-only
signaller. File descriptors are close-on-exec; failures to obtain a process
descriptor abort session startup with an explicit error instead of silently
falling back to weaker cleanup. This requires a kernel and sandbox policy that
permit pidfds. The stubborn-shell test is enabled for Linux as well as macOS.
All five session tests still pass on macOS after separating the platform backends.
Linux compilation/runtime validation is pending. Existing OrbStack Docker was
available, but its existing test image contains no mise or compiler; no tool
installation was performed. This backend does not establish Linux desktop support.

The Linux terminal backend now passes `cargo check` and its unit-test executable
cross-links for x86_64 GNU Linux using mise-managed Rust 1.99.1 and Zig 0.17.0.
The Rust target was installed through the project-local mise target option.
Reproducible tasks are `check-linux-terminal` and `build-linux-tests`. Execution
in the existing ARM64 container failed before tests started because the x86_64
dynamic loader is absent; Zig rejected a fully static glibc link. Runtime Linux
tests and desktop compilation remain pending. These are compilation results,
not evidence of working Linux PTYs or rendering.

ARM64 Linux runtime milestone: all 13 terminal-core tests passed in an existing
Debian 13.7 ARM64 container on OrbStack, including real PTY output/input/resize,
exit reporting, repeated session teardown, and a shell ignoring SIGHUP. The
container used no network, mounted test artifacts read-only, and installed no
tools. Rust's ARM64 target was installed through mise; Ghostty was cross-built
with the pinned Zig. Zig rejected Rust's Cortex-A53 erratum linker flag, so the
run explicitly opted into removing that flag for this Apple Silicon test host;
the portable default still rejects it. This is not a portable ARM64 release
artifact. [Execution record](benchmarks/linux-arm64-terminal-tests.txt).
Linux desktop rendering, Wayland/X11, native IME and Windows remain unverified.

Windows terminal cross-check now passes for `x86_64-pc-windows-msvc`, including
test code. The target was installed through mise, and the already-downloaded
Ghostty Windows package was checksum-verified before extraction. The first check
caught eight compiler errors caused by assuming unsigned C enum bindings in key
arithmetic and color-tag matching. Those operations now use the generated
`GhosttyKey` and `GhosttyStyleColorTag` types. All 13 terminal tests pass on macOS
after the fix. `mise run check-windows-terminal` preserves this check. Windows
linking, DLL loading, ConPTY execution and desktop rendering remain unverified.

The full ARM64 Linux desktop now passes `cargo check`, including GPUI's default
X11 and Wayland paths. Native dependency compilation uses mise-managed Zig via
the target compiler/archiver wrappers; Rust target triples are translated at that
boundary. The check exposed an upstream xattr 0.2.3/libc compatibility error.
The licensed `vendor/xattr` patch maps missing-attribute errors to ENODATA on
Linux/Android while preserving native ENOATTR on macOS/BSD. The macOS workspace
also checks successfully afterward (old xattr deprecation warnings remain).
`mise run check-linux-desktop` records the build setup. Linking, shared-library
resolution, native display/IME behavior and runtime xattr behavior remain open.
Upstream xattr tests were not run: Cargo rejects dependency tests with development
dependencies outside the workspace; the crate was not added to product workspace
members solely to bypass that restriction.
