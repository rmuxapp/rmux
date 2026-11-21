# Shared rmux web compatibility probe

Run `logs` for 80 bundled sample lines. Wheel scrolling uses the native viewport
API and the same fractional wheel accumulation as desktop. Historical selection
and copying work; typing returns to the live prompt. Browser checks cover these
paths. Scrollbar controls, scroll shortcuts and selection edge autoscroll remain.

Native Ghostty gestures now drive mouse selection. Drag, double-click word and
triple-click line selection and copying pass browser checks. Cmd+C (macOS) or
Ctrl+Shift+C (other platforms) copies; browser copy events handle native menu
commands too. Palette clipboard behavior stays independent. Rectangular geometry
passes engine tests; browser Alt-drag and edge autoscroll remain unverified.

The browser now uses desktop's pinned libghostty-vt 1.0.1 WASM build directly.
`mise run build` includes this engine build; `mise run check-native-vt` checks
selection export, keyboard modes and sparse styled snapshots. Browser checks
cover typing, Ctrl+U, splits and workspace restoration. Scrollback, selection
edge autoscroll and keyboard release routing remain incomplete. Historical notes
below describe the migration; legacy-engine.js preserves the old ghostty-web
benchmark, whose recorded timings do not measure this new runtime adapter.

Latest pane chrome shares native framing, control sizes and SVG icons. Split
Right/Below and Zoom buttons work with shared tooltips using the host font.
Native check/WASM build and browser button/tooltip checks pass. Terminal text now
uses a registered platform input handler and native Composition model. Rapid
ASCII typing and single-line paste pass browser checks, including typing after
split and Ctrl+U. Multiline paste, Unicode sample commands and rendered IME
preedit remain incomplete.

This isolated host runs actual desktop sources against GPUI CE 0.3.0 without
changing the production GPUI 0.2.2 dependency. Shared sources include the text
field, composition/undo, tooltip, palette presentation/navigation, and split
presentation, terminal row cache/painter and cell metrics. rmux-core supplies
command identities, search ranking, PaneTree and terminal frame types.
Host adapters cover the GPUI text layout API differences and clipboard forwarding.

Use `mise install`, `mise run build`, then `mise run serve` in this directory.
Open http://127.0.0.1:4319. Rust's WASM target and wasm-bindgen are mise-managed.
Bundled Inter Regular is licensed under `assets/Inter-OFL.txt`.
Bundled Fira Mono Medium is licensed under `assets/FiraMono-LICENSE` (copied from
cosmic-text 0.14.2's font fixtures).

## Current behavior

Commands opens the shared palette. Split Right/Below mutate the shared PaneTree;
Toggle Pane Zoom retains the tree and displays the focused pane. Click a pane to
focus it. The probe is limited to 16 panes. Escape closes the palette. The macOS
text-editing shortcuts are bound. Unsupported command services are not listed.

Pane bodies now use live ghostty-web 0.4.0 VT instances with the actual desktop
row cache and painter. Click a pane and type `help`, `ls` or `clear`. These are
bundled sample responses, never a real shell. Each pane retains its own engine
and output. The initial keyboard path supports ASCII typing, Enter and Backspace;
native IME/clipboard/input handling is not shared yet.

The browser engine adapter transfers only dirty rows as JSON. Rust retains Arc
identities for unchanged rows, keeping the shared shaping cache effective. No
snapshot polling or timer runs while idle. This boundary still needs performance
measurements; JSON is not the final optimized transport.

The engine's exported cells omit custom underline colors/styles and overline;
those fields currently use foreground/straight underline/no overline. Cursor
geometry and block glyph shaping now use the native cursor_view and CursorCache. Selection, tabs, workspaces,
divider dragging, broader input and browser acceptance remain pending. This is
not the public demo replacement yet.

The older synthetic native frame remains available for renderer comparisons:
`mise exec -- cargo run -q -p rmux-desktop --example web_frame > tools/web-probe/assets/terminal-frame.json`
from the repository root.

GPUI CE lacks the native fixed-cell-width shaping parameter. The host shapes an
ASCII run once and checks every caret boundary plus trailing width against the
measured terminal grid. Aligned runs use one shaped line; mismatches fall back to
per-character placement. The shared row cache retains either representation.
The development toolbar reports cumulative batched/fallback run counts on its
next render, without timers. Browser sample checks reported 22 batched runs and
zero fallbacks. Latency/memory measurement and wide/combining coverage remain.

## Verified

Native compilation and the WASM build pass. In-app browser checks covered the
bundled font, typing, select-all/replacement, undo, fuzzy palette search and arrow/
Enter selection. Split Right followed by nested Split Below produces three panes;
zoom/unzoom restores that layout. Compilation and these focused checks do not
establish full native/browser parity.
The shared row painter renders a two-pane captured frame with foreground and
background colors, bold, underline, strikethrough and concealed-text suppression.
All eight native row-cache tests pass after the adapter extraction.

`mise exec -- node scripts/check-engine.mjs` after building checks sparse updates,
colors, pane isolation, clear and disposal/recreation against the actual WASM
engine. Browser typing verified different sample commands in two independent
panes. Dependency licensing is copied into the build alongside the engine.

Terminal geometry follows shared native cell metrics and padding on split/zoom.
Resizes keep the same engine/session, invalidate all viewport rows, and skip work
when dimensions are unchanged. Engine checks cover shrinking, growing, a 1×1
viewport and repeated no-op geometry. Browser split/zoom retains visible output.

Cursor rendering follows engine block/bar/underline shape when focused, and uses
a hollow cursor when unfocused. Browser checks cover block placement after input
and an unfocused hollow cursor. Native blink scheduling and cursor preference
policy are not connected yet.

Cmd+P opens the palette in the tested in-app browser. Executing a command or
pressing Escape returns keyboard focus to the selected terminal. Splitting
focuses the new terminal; clicking a pane header focuses its terminal. Verified
split → type ls and Cmd+P → Escape → type help without a terminal click.
Other browsers may reserve this shortcut; the Commands button remains available.

Run `mise run bench-browser-engine` after building for the isolated 1/4/16-pane
engine/JSON boundary benchmark. It validates dirty-row counts and reports payload
sizes, median/p95 serialization cost and whole-process memory. See
`../../docs/benchmarks/browser-engine.md` for scope and the recorded baseline.
This does not measure browser rendering or input latency.

Cell payloads use `[text, width, resolved foreground, resolved background, flags]`
tuples; columns come from row position and omitted style defaults are rebuilt in
Rust. Text deserializes into SmolStr to retain inline short-cell storage. The
recorded tuple benchmark cuts dense 16-pane payload by 86.5%; see the benchmark
report for timing and measurement limits.

Cmd+[ / Cmd+] now use the native pane shortcut definitions and shared core
wraparound logic. Cycling follows layout order and preserves zoom. Browser checks
verified commands typed in the selected pane after each shortcut. Shifted bracket
tab navigation remains pending until tabs are connected.

Initial or hidden layouts smaller than one full terminal cell do not resize the
engine. This avoids a destructive intermediate 1×1 reflow before the canvas has
its real dimensions. Fresh-load/split/input verification preserves prompt spacing.

Each pane now hosts up to 16 live terminal tabs. Cmd+T or the plus control adds a
sample session. Click tabs or use Cmd+Shift+[ / Cmd+Shift+] to switch. Tab strip
and active/inactive appearance are shared native sources, as are bracket binding
definitions and wraparound calculation. Browser brace-key aliases retain Shift.
Hidden tabs release row/cursor shaping caches while keeping engine state. Browser
verification covered independent outputs and return to a prior tab. Closing,
reordering, badges and full native tab services remain pending.

Close a tab with ×, middle-click or Cmd+W. The last tab removes its pane through
the shared pane tree; closing all panes exposes New Terminal. Removed terminal
entities drop their Ghostty engine. Browser verification covered active-tab close,
split removal, final empty state and fresh session creation/input. Background-tab
close and middle-click interaction coverage remain pending.

Symbolic key normalization now comes from rmux-core::keyboard, also used by
native Ghostty encoding. Browser Enter/Backspace dispatch uses those shared keys.
This does not yet connect full terminal keyboard encoding or native IME handling.

Input now passes shared key symbols through a version-specific ghostty-web key
adapter into an owned Ghostty encoder per session. The adapter synchronizes DEC
cursor/keypad modes. Encoder checks cover plain input, Enter, Backspace, Ctrl+C,
Ctrl+U and application arrows; browser plain-command input is verified. Full
browser control-key interaction remains open. The pinned library does not expose
native-equivalent keyboard-state synchronization, so Kitty negotiation, release
pairing and IME remain pending. The sample interpreter is still ASCII-only.

Verified browser Ctrl+U with a partial draft followed by a sample command. Draft
erasure uses one engine write; an empty draft skips snapshot work. Automated
browser input needed the explicit Control modifier. Ctrl+C UI delivery remains
unverified, although its engine encoding is covered.

Workspaces now use shared native sidebar/header/row styling and workspace shortcut
definitions. Each retains its pane tree, tabs and terminal output while hidden;
hidden workspace shaping caches are released. The plus button or Cmd+N creates
another workspace (16 maximum). Cmd+Option+Up/Down cycles; Cmd+1–8 selects by
position, Cmd+9 selects the last, matching desktop semantics. The current probe
uses macOS bindings; automatic browser-platform keymap selection remains pending.
Native check and WASM build pass. Browser checks verified Cmd+N, Cmd+1 and
Cmd+Option+Up wraparound with restored output, plus mouse switching with split/tab
restoration. Workspace groups, persistence and the full native state/service model
are still separate acceptance work; this is not the published demo yet.

Ctrl+Cmd+G now creates an empty group using the shared native shortcut. Cmd+N
inside a group adds a child through the shared group model. Click the disclosure
to collapse/expand without changing the active session. Numbered/cyclic navigation
includes collapsed children and reveals a selected child. Browser checks verified
creation, inheritance, collapse and numbered reveal with retained terminal output.
Group editing, ungrouping and drag/reorder controls remain pending.

The shared palette now exposes New Workspace, New Empty Workspace Group,
Next/Previous Workspace and Ungroup Workspaces through shared command identities.
Workspace actions emit events to the browser host. Ungroup retains logical order,
selection and sessions. Browser checks cover palette group creation and ungrouping
with retained terminal output; WASM build passes. Workspace-name search and the
rest of the native command catalog are still pending.

Workspace-name search is now connected. Opening the palette takes a fresh logical
workspace snapshot and indexes it alongside commands with the shared fuzzy matcher.
Selecting a workspace result targets its stable entity and restores terminal focus.
WASM build and browser search/switch/return checks pass with retained output.
The combined list scrolls; keyboard-driven off-screen result visibility still
needs completion.

Command results now use the native ListState/list virtualization pattern and
shared row-reveal helper. Page Up/Down uses the native viewport-based calculation;
filtering resets list position. Six native navigation tests and WASM build pass.
Browser checks with 10 workspaces cover wraparound to a previously off-screen
result, Page Up and filtering after scrolling.

Divider dragging now uses native split_interaction capture and weight updates.
Double-click equalizes the neighboring pair; existing engines resize in place.
Native compilation and WASM build pass. Browser verification covers a horizontal
divider drag and equalization with terminal output retained. Nested and vertical
drag interaction checks remain pending.

Split traversal, branch measurement, divider creation and mouse-down handling now
come from the same native renderer, with host-specific leaf content. Native check
and WASM build pass. Nested vertical dragging exposed a transient upper-pane text
repaint issue (returns on the next pointer interaction); nested equalization also
needs checking. These remain open, despite shared source and passing compilation.

The shared drag-release handler now notifies its host after clearing drag state,
committing a final non-dragging scene. Browser regression confirms the previously
missing upper text is visible immediately after nested vertical dragging. Native
check and WASM build pass. Nested double-click equalization remains unresolved.

Nested double-click equalization is now verified: event tracing confirmed correct
path/count delivery and the resulting frame restored equal heights. Diagnostics
were removed. The palette also offers four Resize Pane directions and Equalize
Pane Sizes through native core operations. Clean WASM build and browser Up/Equalize
interaction checks pass.

Directional focus uses native PaneTree::neighbor and shared shortcut definitions.
Cmd+Ctrl+arrows or Focus Pane commands choose geometrically adjacent panes, retaining
zoom state. Native check/WASM build pass; browser nested Up/Left checks confirmed
subsequent input reaches the intended terminal and other sessions remain intact.

Platform metadata now selects native macOS or Windows/Linux defaults. Split/Below/
Zoom bindings are shared with desktop, alongside workspace and directional focus
bindings. Available-browser macOS checks cover Cmd+D, Cmd+Shift+D and
Cmd+Shift+Enter zoom/unzoom. Native check, WASM build and engine/platform selector
checks pass; Windows/Linux browser interaction and reserved shortcut behavior
remain separate acceptance work.

Palette hints now resolve actual key bindings against the terminal that opened
it and format them with native shortcut_label code. Unbound actions omit hints;
workspace results use their numbered navigation bindings. All 15 native shortcut
tests and WASM build pass; browser visual check confirms macOS labels.

Close Workspace is available in the palette. It removes the workspace entity,
selects the shared logical neighbor, and preserves a surviving group anchor's
position. Closing all workspaces shows a New Workspace empty state; Cmd+N works
there too. WASM build and browser close/empty/recreate/input checks pass. Grouped
close interactions and memory measurements remain pending.
