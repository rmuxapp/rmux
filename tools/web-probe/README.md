# Shared rmux web compatibility probe

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
