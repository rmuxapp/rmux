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

Pane bodies use the actual desktop row cache and painter with a captured native
Ghostty frame. The fixture is synthetic, generated from the repository root with
`mise exec -- cargo run -q -p rmux-desktop --example web_frame > tools/web-probe/assets/terminal-frame.json`.
This is not the public demo replacement yet. Live browser VT parsing/input,
cursor/selection interaction, divider dragging, tabs, multiple workspaces,
IME/clipboard acceptance, browser coverage and performance remain.

GPUI CE lacks the native fixed-cell-width shaping parameter. Its host adapter
places ASCII glyphs individually at measured terminal columns and caches the
result through the shared row cache. This adds cold shaping work compared with
native batching; performance and wide/combining-character coverage remain open.

## Verified

Native compilation and the WASM build pass. In-app browser checks covered the
bundled font, typing, select-all/replacement, undo, fuzzy palette search and arrow/
Enter selection. Split Right followed by nested Split Below produces three panes;
zoom/unzoom restores that layout. Compilation and these focused checks do not
establish full native/browser parity.
The shared row painter renders a two-pane captured frame with foreground and
background colors, bold, underline, strikethrough and concealed-text suppression.
All eight native row-cache tests pass after the adapter extraction.
