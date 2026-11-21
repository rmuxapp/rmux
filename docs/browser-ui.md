# Shared native and browser UI

The browser demo must converge on the actual rmux application, not a separately maintained JavaScript imitation. Sample sessions replace local processes; they should not replace workspace behavior or UI code.

## Current state

The local shared browser build now also uses native pane framing, control sizes
and SVG icons. Tooltips select the host font explicitly, avoiding the unavailable
macOS font in the browser overlay. Split buttons and zoom/unzoom are verified;
rapid text-event delivery still needs correction. This build is not deployed.

Desktop uses GPUI 0.2.2 and libghostty-vt. The local-only Cloudflare site uses JavaScript and ghostty-web 0.4.0. It shares neither the Rust application UI nor workspace state. It is a layout showcase, not a browser build of rmux. Keep that distinction visible until the shared build works.

Rustable demonstrates the intended architecture: its browser host imports the same application crate and mounts its egui UI with WebRunner. Its demo services differ from native services, while its UI remains shared.

## Implementation path

1. Share existing application behavior without rewriting it. The terminal snapshot contract (`Cell`, `Frame`, cursor/colors and bounded text export) is now in core as well; native engine types are re-exports. `rmux-core` now owns the existing pane tree, directional navigation and bounded focus history; desktop consumes that crate. Preserve current behavior and tests.
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

## Terminal rendering adapter

The owned frame contract and style-run grouping now live in rmux-core and are
used by the native renderer. Keep row Arc identity and style-run batching when
connecting the web host. GPUI 0.2.2 shapes with an explicit cell-width argument;
GPUI CE 0.3.0's shape_line signature does not accept that argument. Verify fixed
column placement with a bundled monospace font, wide/combining cells and styled
ASCII runs before calling the renderer compatible. Do not silently replace the
native cache or advertise a plain text widget as the terminal renderer.

The isolated host now compiles that same row_cache.rs and cell_metrics.rs with
host-specific shaping/paint API adapters. A captured native Ghostty fixture
renders through paint_row in split panes with a bundled monospace font. ASCII
columns are explicitly placed because CE lacks fixed-width shaping; this is
correctness-first and has not passed performance acceptance. The fixture does
not substitute for browser VT parsing, live input, cursor/selection interaction
or wide/combining-character acceptance.

The host now also connects live ghostty-web 0.4.0 instances to that painter. Each
pane owns an engine; sparse JSON patches replace dirty rows while preserving
unchanged Rust Arc identities. Sample commands exercise basic ASCII input and
independent state. Native input/IME, cursor painting, selection, resize and
workspace integration remain acceptance work. Browser engine cell exports also
lack native custom underline decoration and overline details.

The browser host now retains independent workspace entities with live pane/tab
state. Native sidebar presentation and workspace shortcut defaults are shared
source modules. Switching releases hidden rendering caches without dropping
terminal engines. Browser checks cover restoring split layouts/tabs/output and
keyboard workspace creation, numbered selection and wraparound. The host still
owns a separate workspace service model; this is progress toward shared UI, not
completion of the Rustable-style single application model.

Workspace group ownership and ordering now live in rmux-core, extracted from the
native model with its existing tests and restore validation. Both hosts consume
that model and shared numbered/cyclic selection rules. Browser group controls
are not connected yet; having the model does not establish group UI parity.
