# Shared native and browser UI

The browser demo must run rmux's application behavior and UI. Bundled sample
sessions replace local processes; they must not replace workspace behavior with
a separately maintained imitation. Rustable's shared application crate remains
the architectural target.

## Published versus shared build

The published Cloudflare demo still uses JavaScript and ghostty-web 0.4.0. It is
a layout showcase, not the shared Rust application. The marketing site remains
local and off GitHub.

`tools/web-probe` contains the replacement under development. It uses GPUI CE
0.3.0 for browser rendering while desktop remains on GPUI 0.2.2. The browser
build shares substantial application code, but its workspace orchestration and
platform services are still separate. Do not describe it as full parity yet.

## Shared implementation

- **Application state:** pane trees, directional focus, resizing/equalization,
  workspace groups and logical ordering, shared creation/visibility rules,
  previous-workspace identity tracking, tab ownership/selection/reordering,
  command identities, palette matching, terminal frames and style runs.
- **Views:** native palette and text field, sidebar/tab/pane presentation,
  recursive split layout and interaction, icons, row cache/painter, cell metrics,
  cursor geometry and draggable scrollback indicator.
- **Input:** shared composition model, workspace/pane/tab shortcut definitions,
  tab action dispatch, fractional wheel accumulation and Shift+PageUp/Down/Home/End.
  Delivered-key tracking is shared too: shifted aliases pair with their original
  press, and repeats do not replace it. The browser routes release events for
  encoded control/non-text keys and drains held keys on terminal focus loss.
- **Terminal engine:** the same pinned libghostty-vt 1.0.1, compiled to WASM with
  its C API manifest. Sparse row patches preserve unchanged Rust row identities.
  Native key encoding, gesture selection, bounded text export and viewport APIs
  drive browser behavior. The historical ghostty-web adapter is benchmark-only.
- **Defaults:** Rust supplies desktop foreground/background and a 4 MiB history
  limit. This is a history limit, not a bound on total browser memory.

Browser tabs retain independent engine/session state. Hidden workspaces/tabs
release shaped-row caches. Tab titles remain attached to session identity.
Closing the last terminal focuses the empty workspace, allowing keyboard or
palette creation of a replacement terminal.

The browser font adapter uses bundled Fira Mono. Desktop uses platform fonts.
GPUI CE lacks the native fixed-cell-width shaping argument, so the adapter uses
batched ASCII runs only when every caret boundary matches the terminal grid;
otherwise it falls back to explicit placement. Do not infer native performance
from successful rendering.

## Verification to date

Focused browser checks cover workspace restoration, groups, splits, resizing,
zoom, palette search, tab creation/navigation/reordering/closing, empty-pane
recovery, rapid ASCII text, single-line paste, scrollback, selection/copy and
scrollbar dragging. See [progress](progress.md) for individual evidence and limits.

Direct engine tests cover Unicode/wide/combining cells, styled sparse snapshots,
keyboard modes and Kitty action encoding, selection gestures and copy limits,
absolute history targets, edge-scroll continuation, configured colors and OSC
reset behavior. These tests do not establish browser event or IME parity.

Selection edge scrolling uses the native continuation request and a cancellable
30 ms timer shared with desktop. Held-pointer browser timing remains unverified.

The [engine benchmark](benchmarks/native-vt.md) measures write, cell extraction
and JSON serialization at 1/4/16 panes. It excludes Rust decoding, shaping, GPU,
presentation latency and browser memory. Broader performance claims remain open.

## Build and artifact path

Use `mise run build` inside `tools/web-probe` for fast development output in
`dist`. `mise run build-release` produces optimized output in `dist-release`.
The **Shared WASM demo** GitHub workflow builds the release artifact, checks the
engine adapter and uploads `rmux-shared-demo`. It does not publish the site.

`build-sizes.json` records raw/gzip/Brotli WASM payload sizes and SHA-256 hashes.
The retired ghostty-web npm package is prepared only for legacy benchmarks; it
is not part of the replacement runtime or optimized artifact.

## Remaining work and deployment acceptance

1. Continue moving workspace orchestration into shared application state/views;
   keep browser sample services behind platform adapters. Tab and group models
   alone do not complete the Rustable-style architecture.
2. Complete browser terminal input: IME preedit, Unicode sample input/paste,
   negotiated report-all printable-key routing and platform-reserved shortcut behavior. Validate held
   edge selection, Alt-drag and cancellation when focus leaves the browser.
3. Close missing UI/service parity against [feature parity](feature-parity.md)
   and [roadmap](roadmap.md), including settings and context menus. The static
   demo has no real shell, filesystem or SSH access by design.
4. Measure optimized startup, idle work, input latency and memory at representative
   pane counts. Verify resize, workspace switching, zoom, Unicode and retained
   output using the optimized browser artifact.
5. Embed the verified shared build in the existing local Cloudflare site and
   replace the published imitation. Keep native performance and release builds
   working throughout; do not publish parity or performance claims beyond evidence.
