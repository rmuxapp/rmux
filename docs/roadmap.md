# rmux delivery roadmap

Build a Rust application for macOS, Linux and Windows that preserves cmux's workspace, pane and surface model. Deliver terminal performance first, then add the complete documented capability set in stages. Features deferred to later phases remain parity obligations.

The [132-family feature inventory](feature-parity.md) is the scope ledger. Every row has a stable ID, upstream evidence and a proposed acceptance criterion. The [contract inventories](reference/README.md) capture finer command/configuration details. Current implementation status and remaining gaps are recorded in [features.json](reference/features.json). Partial slices do not satisfy an entire feature family.

## What parity means

1. **Interaction parity:** equivalent workspace layout, keyboard workflows, terminal behavior and state ownership.
2. **Functional parity:** each documented capability works, including lifecycle, errors, persistence and permission behavior.
3. **Automation parity:** command names alone are insufficient. Target resolution, parameters, output, aliases, exit codes, events and side effects must be specified and tested. A cmux compatibility mode is a proposed separate interface; rmux remains the primary name.
4. **Platform coverage:** macOS, Linux and Windows pass the same portable behavior tests. OS-specific capabilities require declared exceptions or remote-host equivalents.
5. **Performance:** each phase passes the [performance gates](performance.md). Optional features incur costs when enabled; their costs must be visible and bounded.

The latest GitHub release endpoint returned **v0.65.2.2**, published October 27, 2026. The inventory also includes newer main-branch documentation and explicitly marked beta/unreleased work. It does not claim all 132 families are stable, nor that a static source inventory proves runtime support. [Release](https://github.com/manaflow-ai/cmux/releases/tag/v0.65.2.2)

## Phase sequence

### Current delivery priority

Focus on polish and release quality of the existing terminal workspace: coherent
native and WASM interactions, keyboard and focus behavior, clear menus and
controls, dependable session handling, and reliable downloads and updates.
Resolve reported regressions before adding new feature families. The phases
below retain the longer-term parity scope; they are not the immediate work queue.

Implement related improvements in substantial batches, then run targeted checks
and a combined build. Avoid rebuilding or deploying each small change. Native
release builds remain on GitHub; native Windows runtime validation stays deferred
at the user's request.

| Phase | Outcome | Main dependencies | Exit gate |
| --- | --- | --- | --- |
| P0 | Prove the portable terminal and renderer | Ghostty API, GUI integration, PTY adapters | One terminal and eight splits on all three OS families; IME, DPI, Unicode, output stress and teardown verified |
| P1 | Usable cmux-style terminal workspace | P0 | Windows, sidebar workspaces, pane tabs, nested splits, navigation, selection and search work without process recreation |
| P2 | Reliable everyday desktop and automation core | P1 | Restore, config, metadata, notifications, RPC/events, CLI, diagnostics and installation pass functional and performance tests |
| P3 | Browser and file workspace | P2; browser-engine decision | Browser automation and file tools work beside busy terminals; lazy creation and memory reclamation demonstrated |
| P4 | Remote work and live session ownership | P2; P3 for remote browser | SSH, relay, reconnect, transfers, Mosh, local tmux and remote tmux tests pass with explicit ownership semantics |
| P5 | Agent workflows and advanced automation | P2; P3/P4 where applicable | Supported provider adapters, Feed, Vault, resume, messaging, templates and rules pass isolated compatibility suites |
| P6 | Advanced desktop and beta parity | Relevant P3–P5 components | Canvas, custom sidebars, hibernation, advanced browser features and configuration gaps are resolved individually |
| P7 | Cloud, device, mobile and platform services | Stable identity and transport contracts | Separate service/client deliverables meet their own acceptance and operational requirements |

These are dependency phases, not calendar estimates. P3 and P4 can evolve independently after the shared core; P5 features need only their actual prerequisites. Record implementation work against individual feature IDs instead of marking an entire phase done after a demo.

## P0 first implementation backlog

- Pin the Ghostty revision/API and Zig toolchain. Wrap ownership, callbacks and terminal lifecycle in a small FFI crate.
- Prototype GPUI integration with one terminal-specific drawing element. Compare against Iced plus wgpu if composition, IME, accessibility or resource sharing is unsuitable. Framework selection is not final.
- Implement Unix PTY and Windows ConPTY adapters, including resize, process exit, cancellation and teardown.
- Route terminal output through the engine to incremental render snapshots. Keep output out of the general UI state store.
- Implement text shaping, glyph caching and terminal cell drawing. Exercise CJK, combining characters, emoji, alternate-screen TUIs and high-volume output.
- Run the same one-pane/eight-pane benchmark on macOS, Windows and Linux, including Wayland and X11 where supported.
- Record baseline memory attribution and input/frame timings before adding the sidebar.

**Acceptance gate:** accept P1 only after the embedding path works on all target platforms and no architectural blocker remains. Following the user's instruction to prioritize speedy implementation and defer Windows validation, P1 layout implementation can proceed alongside unfinished P0 validation. This does not mark either phase accepted. Revisit the renderer or library boundary if needed; do not conceal platform gaps behind a macOS-only prototype.

## Same layout specification

Retain the hierarchy **window → workspace → pane → surface**. A workspace occupies a sidebar row; a pane is a split region; a surface is a tab inside that region. Preserve this distinction in code, UI and APIs. [cmux concepts source](https://github.com/manaflow-ai/cmux/blob/a76816e4f0788003758b9434ab702068a640fb5d/web/app/%5Blocale%5D/%28landing%29/docs/concepts/page.tsx)

Before P1 is accepted, capture reference layouts and interaction recordings at fixed dimensions, theme, font and DPI. Cover a single pane, four nested splits, tab overflow, workspace groups, sidebar collapse, drag previews, notifications, multi-window movement and restored layouts. Measure spacing, minimum sizes, focus order and hit targets. The current documentation inventory is not a pixel-measured design specification.

Use Command-based defaults on macOS and platform-appropriate bindings elsewhere. Provide an explicit compatibility preset where useful. Preserve behavior while adapting platform window controls, fonts and accessibility.

## Keep complete scope affordable at idle

The default terminal path should not start a web engine, cloud connection, transcript indexer or computer-use helper. Browser surfaces initialize when opened. Transcript indexing, remote transports and optional integrations have explicit lifecycles and bounded work queues. UI visibility and feature availability are separate concepts: hiding a pane must not silently kill an active task.

Agent hibernation is a separate product feature, not a substitute for efficient terminal memory management. It intentionally stops eligible processes and needs provider-specific resume correctness. Track it under G12 rather than including it in ordinary hidden-pane cleanup.

## Completion and release policy

A feature progresses through `not_started`, `specified`, `in_progress`, `implemented`, `verified` and `released`. Verification records must include platform/build, upstream comparison target, scenario, expected outcome, actual outcome and evidence artifact. `blocked`, `unsupported_platform` and `intentional_difference` are explicit non-parity outcomes.

Use three truthful release labels:

- **Terminal workspace milestone:** P0–P2 complete; browser/remote/agent gaps remain visible.
- **Desktop parity milestone:** all applicable P1–P6 rows verified; platform exceptions published.
- **Ecosystem parity milestone:** implemented P7 features also verified, including separate mobile/service clients. RFC-only entries remain a watchlist until upstream implementation is established.

No calendar commitment or fixed memory promise should be published until P0 supplies measurements and P3 resolves browser costs.
