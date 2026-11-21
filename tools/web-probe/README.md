# Shared UI web compatibility probe

This isolated crate compiles the actual rmux desktop `TextField`, composition
and undo modules against GPUI CE 0.3.0. It does not copy those implementations
and does not change the production GPUI dependency. Clipboard forwarding is a
small host adapter; the probe declares the actions the existing field consumes.

Run `mise install`, then `mise run check` here.
Run `mise run build` and `mise run serve` to open the probe at
http://127.0.0.1:4319. The build uses pinned mise-managed wasm-bindgen, and the
bundled Inter Regular font is distributed under `assets/Inter-OFL.txt`. The WASM Rust target is managed
by this directory's mise configuration, with no independent rust-toolchain file.

A successful check establishes source compatibility only. The browser host includes a bundled licensed font, WASM loader and macOS editing
shortcuts. Browser rendering, typing, select-all/replacement and undo were verified in the
in-app browser. IME and clipboard behavior remain unverified. This is not a finished demo. The next gate
is rendering the shared input and exercising editing/IME in a browser, before
attempting the full workspace or changing rmux's native renderer.

## Compatibility result

`mise run check` passes for `wasm32-unknown-unknown`. The same desktop field
also passes the native desktop compile. Small `text_layout` adapters translate
focus, text shaping, caret geometry, clipping and painting between GPUI 0.2.2
and GPUI CE 0.3.0. The text field source, composition, undo and tooltip are shared.

The CE adapter uses its backend caret geometry and byte-index hit testing; it
does not approximate glyph positions. Native calls preserve the previous APIs.
The bundled font and shared field render in the in-app browser. Typing,
select-all/replacement and undo passed a focused interaction check. IME, clipboard,
other browsers and full application parity remain unverified.

## Shared palette presentation

The probe imports the same `palette_view` panel, input surround, result row,
footer and keyboard-navigation functions used by the native command palette.
It also uses native `RankedSearch`. The seven fixture commands only report
selection; they do not pretend to create workspaces or terminals. A browser check
verified `splt` fuzzy matching and Down/Enter selection of Split Below, and the
panel fits its contents. Full application actions and result metadata are pending.

Command labels/identities and RankedSearch now come from `rmux-core`, also used
by desktop. The probe exposes a small subset of the shared catalog; dispatch
still reports selection only. Adding a label does not implement its service.
