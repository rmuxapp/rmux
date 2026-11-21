# Shared UI web compatibility probe

This isolated crate compiles the actual rmux desktop `TextField`, composition
and undo modules against GPUI CE 0.3.0. It does not copy those implementations
and does not change the production GPUI dependency. Clipboard forwarding is a
small host adapter; the probe declares the actions the existing field consumes.

Run `mise install`, then `mise run check` here. The WASM Rust target is managed
by this directory's mise configuration, with no independent rust-toolchain file.

A successful check establishes source compatibility only. Browser execution
still needs bundled licensed fonts, WASM bindings/loader, clipboard and keyboard
bindings, and interaction validation. This is not a finished demo. The next gate
is rendering the shared input and exercising editing/IME in a browser, before
attempting the full workspace or changing rmux's native renderer.

## Compatibility result

`mise run check` passes for `wasm32-unknown-unknown`. The same desktop field
also passes the native desktop compile. Small `text_layout` adapters translate
focus, text shaping, caret geometry, clipping and painting between GPUI 0.2.2
and GPUI CE 0.3.0. The text field source, composition, undo and tooltip are shared.

The CE adapter uses its backend caret geometry and byte-index hit testing; it
does not approximate glyph positions. Native calls preserve the previous APIs.
Browser font loading, rendering and interaction are still unverified. Compilation
alone does not establish IME, selection, clipboard or visual parity.
