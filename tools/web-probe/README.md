# Shared rmux web compatibility probe

This isolated host runs actual desktop sources against GPUI CE 0.3.0 without
changing the production GPUI 0.2.2 dependency. Shared sources include the text
field, composition/undo, tooltip, palette presentation/navigation, and split
presentation. rmux-core supplies command identities, search ranking and PaneTree.
Host adapters cover the GPUI text layout API differences and clipboard forwarding.

Use `mise install`, `mise run build`, then `mise run serve` in this directory.
Open http://127.0.0.1:4319. Rust's WASM target and wasm-bindgen are mise-managed.
Bundled Inter Regular is licensed under `assets/Inter-OFL.txt`.

## Current behavior

Commands opens the shared palette. Split Right/Below mutate the shared PaneTree;
Toggle Pane Zoom retains the tree and displays the focused pane. Click a pane to
focus it. The probe is limited to 16 panes. Escape closes the palette. The macOS
text-editing shortcuts are bound. Unsupported command services are not listed.

The terminal renderer is not connected: pane bodies explicitly say so. This is
not the public demo replacement yet. Divider dragging, terminals, tabs, multiple
workspaces, IME/clipboard acceptance, browser coverage and performance remain.

## Verified

Native compilation and the WASM build pass. In-app browser checks covered the
bundled font, typing, select-all/replacement, undo, fuzzy palette search and arrow/
Enter selection. Split Right followed by nested Split Below produces three panes;
zoom/unzoom restores that layout. Compilation and these focused checks do not
establish full native/browser parity.
