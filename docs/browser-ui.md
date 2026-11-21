# Shared native and browser UI

The browser demo must converge on the actual rmux application, not a separately maintained JavaScript imitation. Sample sessions replace local processes; they should not replace workspace behavior or UI code.

## Current state

Desktop uses GPUI 0.2.2 and libghostty-vt. The local-only Cloudflare site uses JavaScript and ghostty-web 0.4.0. It shares neither the Rust application UI nor workspace state. It is a layout showcase, not a browser build of rmux. Keep that distinction visible until the shared build works.

Rustable demonstrates the intended architecture: its browser host imports the same application crate and mounts its egui UI with WebRunner. Its demo services differ from native services, while its UI remains shared.

## Implementation path

1. Share existing application behavior without rewriting it. `rmux-core` now owns the existing pane tree, directional navigation and bounded focus history; desktop consumes that crate. Preserve current behavior and tests.
2. Run a bounded compatibility probe using an actual rmux GPUI view on a web-capable GPUI distribution. Check native API compatibility before changing the production dependency. Do not replace GPUI with a second UI framework merely for the demo.
3. Extract shared views and application state from the desktop entry point. Introduce platform services for terminal sessions, files, persistence, notifications and window control. Native hosts keep real PTYs; browser hosts receive bundled sample sessions and no server access.
4. Adapt the same terminal grid renderer to a browser terminal engine interface. A WASM VT parser alone does not provide shared UI. Verify cell snapshots, input, selection, resize and focus with the shared view.
5. Build and test the browser host, then embed its output in the existing local-only Cloudflare site. Keep the marketing site off GitHub. Replace the JavaScript showcase only after the real UI renders and responds correctly.

## Candidate browser backends

The pinned GPUI 0.2.2 has native macOS, Linux/FreeBSD and Windows backends, with no web backend. Newer forks provide a route worth probing:

- [GPUI CE web guide](https://docs.rs/gpui-ce/latest/gpui/_documentation/guides/web/index.html): one canvas, WebGPU with WebGL2 fallback, bundled fonts, optional workers with cross-origin isolation.
- [GPUI Kit WebAssembly guide](https://gpui-kit.com/docs/webassembly/): shared Rust views through gpui-pre-web; explicitly describes browser support as a showcase path requiring product-specific validation.

These are candidates, not validated dependencies. Do not claim native performance or full parity from their examples. Avoid a production renderer migration until a probe establishes the required behavior.

## Acceptance

The browser must use the same workspace/tab/pane state, command definitions, palette, shortcuts and view layout as native, with explicit platform exceptions for reserved browser shortcuts and OS-only services. Switching workspaces must retain session state; resize and zoom must not mix terminal contents. Measure startup, idle work, input latency and memory at representative pane counts. Keep native desktop performance and release builds working throughout.
