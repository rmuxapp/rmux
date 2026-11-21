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

## Initial compatibility result

The pinned WASM dependency graph compiles through GPUI CE Web. The shared
desktop field does not yet compile against it. Current unique diagnostics:

- `E0061`: this method takes 2 arguments but 1 argument was supplied (2 occurrence(s)).
- `E0599`: no method named `x_for_index` found for struct `gpui::ShapedLine` in the current scope (7 occurrence(s)).
- `E0063`: missing field `fade_out` in initializer of `gpui::ContentMask<_>` (1 occurrence(s)).
- `E0061`: this method takes 6 arguments but 4 arguments were supplied (2 occurrence(s)).
- `E0277`: the trait bound `Alpha<Hsl, f32>: From<Alpha<Rgb, f32>>` is not satisfied (2 occurrence(s)).
- `E0063`: missing field `letter_spacing` in initializer of `gpui::TextRun` (2 occurrence(s)).
- `E0061`: this method takes 3 arguments but 4 arguments were supplied (2 occurrence(s)).
- `E0599`: no method named `closest_index_for_x` found for struct `gpui::ShapedLine` in the current scope (2 occurrence(s)).

This is an expected failing compatibility probe, excluded from the root workspace.
Do not treat it as a working web app. Next: isolate text layout/paint and focus
adapters while keeping the actual editing and view source shared, then rerun the
probe and validate rendering. The desktop remains on GPUI 0.2.2.
