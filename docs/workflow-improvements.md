# Workflow improvement delivery

Post-0.1.24 implementation and verification are complete and included in the
authorized v0.1.25 release batch. The longer-term parity/performance roadmap remains separate.
All development commands used mise; release builds remain on GitHub and native
Windows runtime verification remains deferred. Existing user tools and the
unrelated `scripts/check-generated-appcast.py` file were preserved.

## Requirement audit

| Goal requirement | Delivered behavior and evidence |
| --- | --- |
| Investigate intermittent input/shell-exit failures | Captured desktop input `wWADfail` where the fixture sent `fail`. `RMUX_STARTUP_ACTIVATE=0` prevents the smoke fixture taking keyboard focus. Five consecutive runs passed during investigation; the final live-shell run verifies state retention, targeted Unicode input, failure/EOF exits and child cleanup. [Final report](benchmarks/workflow-live-rpc-smoke.json). The earlier unchanged-variable failure is consistent with contamination but was not captured directly. |
| CPU/render/memory baseline at 1/8/32 | Accepted native visible-idle, visible-busy, hidden-busy, minimized-busy and minimized-idle records cover every count. CPU/RSS, per-terminal paint/output counters and cleanup were audited. Published hidden-idle measurements are also retained. [Current matrix and limitations](performance.md#current-workflow-baseline). |
| Evidence-based rendering optimization | A profile exposed hidden-view invalidation without painting. Pure output now skips invalidation after snapshot release; metadata/errors/exits retain independent updates. Corrected hidden benchmarks prove continued output and zero paints while skipping notifications for every terminal. [Hide/show smoke](benchmarks/render-visibility-smoke-debug.json) proves Unicode retention, fresh rows on return and continued visible painting. No percentage speedup is claimed. |
| Common Settings controls | Direct editors cover six colors, font family and session/socket/history paths, including swatches, native browsing, validation, defaults and per-setting resets. Save merges only draft changes, preserves unrelated JSON edits and rejects same-field conflicts. Revert/undo preserve unrelated values and environment-controlled close policy. Editor/preference/reset tests and native apply/cancel/save/reset/revert/undo/reload checks passed. |
| Restart hints and search explanations | Path and startup controls show save/restart guidance. Search results explain exact configuration keys or normalized matching concepts, including aliases and typos. The local weighted term-vector index remains cached; explanations are computed when the query changes. Catalog coverage and matching tests pass. |
| Actionable startup/socket/session recovery | Errors identify the directory/file and offer concrete retry/restart or Save Session actions. Existing endpoints, files, symlinks and recovery files are preserved. Socket preservation tests pass. Toasts keep full diagnostic details in Copy/tooltip while showing concise actions. |
| UI polish and small windows | Native Settings verified at roughly 756×552 logical pixels: categories/footer remain visible, theme choices wrap, content scrolls, editors and wrapped errors fit. Keyboard search/submission/cancel, button focus, color swatches, save tick toast and undo/reload were exercised. Routine success notices are concise; one-result grammar is corrected. Added and verified native Window → Minimize / Cmd+M, with user shortcut precedence preserved. |
| Workspace-wide terminal search | Results group by window/workspace, retain headings during filtering, highlight contextual literal excerpts and navigate past headings. Fixed a resize race by deferring Find navigation until the destination frame is ready. Native tests searched eight terminals across two windows, filtered Documentation/Notes, and verified the destination window raised with Find at 1 / 1 and the visible match highlighted. Grouping/navigation/search tests pass. |

## Validation

- `mise exec -- cargo test --locked -p rmux-desktop`: **320 passed, zero failures,
  six intentional ignores** (manual benchmarks and a child helper exercised by
  its parent).
- `mise exec -- cargo build --locked -p rmux-desktop -p rmux-cli`: passed after the
  final code changes.
- `mise exec -- python scripts/smoke-live-rpc.py --live-shell`: passed against the
  final debug binary; report records binary and harness hashes.
- Focused tests include 19 Settings, 43 preference, six terminal-search, 13
  command-palette and three diagnostics tests; all passed during implementation.
- Native UI observations and per-terminal counters establish the separate visual
  and rendering checks described above; unit tests alone do not establish them.
- Benchmark artifact audit verified counts 1/8/32, matching terminal identities,
  CPU/RSS fields, required per-terminal paint/output behavior, and no surviving
  measured children after close/quit. `git diff --check` passed.

## Interpretation limits

The native performance numbers are single debug samples with Metal HUD overhead,
not release budgets, GPU-memory measurements or end-to-end input latency. Dense
32-pane layouts are stress fixtures. Historical samples with unverified native
visibility or the extra busy survivor are explicitly excluded from accepted
scenario evidence. Their raw records remain for transparency. Broader performance
roadmap items in `performance.md` are future work, not claimed delivered here.

No user configuration was used for native test edits; all fixtures used temporary
configuration. Release publication and platform packaging run through GitHub Actions.
