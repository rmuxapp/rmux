# Build and validation notes

[← User guide](../user-guide.md)

Historical implementation detail collected from the previous guide. For current build instructions and acceptance status, use [Building](../building.md) and [Progress](../progress.md).

## On this page

- [Historical prototype status](#historical-prototype-status)
- [Architecture background](#architecture-background)
- [Build from source](#build-from-source)
- [Build and performance references](#build-and-performance-references)
- [Layout smoke check](#layout-smoke-check)

## Historical prototype status

A working Rust/Ghostty terminal-workspace prototype, with a locally runnable
macOS release build. Start with [Trying rmux](../trying-rmux.md) for launch,
essential shortcuts and the current milestone gaps. Run `mise run open-app`
from this project to build and open it. Full cmux parity remains in development.

## Architecture background

A planned Rust terminal workspace application for macOS, Linux and Windows, using Ghostty terminal technology and a cmux-style workspace layout. The priorities are responsive input, low idle overhead, bounded memory and eventual feature parity.

This repository contains an early working macOS prototype plus the cross-platform roadmap. The prototype connects libghostty-vt 1.0.1, portable-pty and GPUI 0.2.2, renders terminal text directly through GPU elements, and supports workspaces, nested splits and per-pane tabs. Initial measurements and their limitations are recorded in `docs/benchmarks`; performance acceptance remains incomplete.

## Build from source

Development tooling is managed through mise. Never install tools with Homebrew. Installations outside mise require explicit approval; the Apple Metal Toolchain was approved and installed for this machine's GPUI build.

```sh
mise install
mise run bootstrap
mise run run
```

`mise run bundle` creates `target/debug/rmux.app` on macOS. `mise run test` runs workspace tests; `mise run check` checks the workspace. Cmd+D splits the focused pane right; Cmd+Shift+D splits below. Linux/Windows use Ctrl+Shift+D and Ctrl+Shift+E. Splits can nest in either direction. Cmd+V pastes and Cmd+Q quits.
Interactive New Workspace, sidebar/group plus buttons and New Workspace in Directory
prepare their terminal in the background and show **Starting workspace…**. Only one
command/workspace startup can be pending in a window. Directory and placement are
captured at the request; completion inserts relative to the original workspace and
preserves focus if you moved elsewhere. Closing/moving the origin or changing its
group during startup discards the new terminal with a notice. Settings changes are
applied before insertion. Use the notice's Cancel control, **File → Workspaces → Cancel New Workspace**, the palette command, or custom `cancel_new_workspace` binding to
cancel. The launch slot stays occupied until the worker returns; cancellation
cannot interrupt an OS spawn already in progress. Its result is discarded and an
unattached child is cleaned up. Closing the window also signals cancellation.

Initial window and group-creation paths remain separate.

## Build and performance references

`mise run bundle-release` creates the optimized macOS bundle. For isolated idle measurements, run `mise exec -- sh scripts/prepare-macos-benchmark.sh 8`, open the resulting benchmark bundle, and sample its PID with `mise exec -- sh scripts/sample-macos-idle.sh PID`. The benchmark uses clean shells and disables the Metal HUD without changing normal launches or system settings.

The current UI is a P0 rendering prototype. Control keys use Ghostty's encoder and paste respects bracketed-paste mode. Full keyboard protocols, selection/search, accessibility, nested layouts and persistence remain unfinished. Linux/Windows dependency setup is provided, but their desktop builds and runtime behavior have not been validated. See [platform build instructions](../building.md) and [implementation progress](../progress.md). `mise run bench-engine` measures release engine throughput and snapshot costs for 1/4/16 panes.

`mise run bench-pty` measures input calls and response snapshots under output pressure from up to 15 other PTY sessions. It does not measure GUI or input-to-pixel latency.

Start with the [roadmap](../roadmap.md), then the [feature inventory](../feature-parity.md).

| Document | Purpose |
| --- | --- |
| [Roadmap](../roadmap.md) | Delivery phases, dependencies and completion gates |
| [Feature parity](../feature-parity.md) | 132 capability families with evidence and acceptance criteria |
| [Architecture](../architecture.md) | Proposed cross-platform design and decisions requiring prototypes |
| [Performance](../performance.md) | Workloads, measurement definitions and initial targets |
| [Research and sources](../research.md) | Pinned baseline, evidence limits, discrepancies and refresh procedure |
| [Contract inventories](../reference/README.md) | Commands, settings, shortcut IDs and static RPC capabilities |

The comparison baseline is cmux commit `a76816e4f0788003758b9434ab702068a640fb5d`, reviewed November 19, 2026. Main-branch capabilities are not all stable-release features. rmux's architecture and phase assignments are proposals; upstream evidence is identified separately.

## Layout smoke check

For a focused developer check of these layout operations, build the desktop and
run `mise exec -- python scripts/smoke-workspace-placement.py --live-shell`.
It launches an isolated test app and fixture shells, checks retained process state
through resize/split/equalize, and observes cleanup without attaching to existing
rmux windows.
