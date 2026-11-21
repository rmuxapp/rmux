# rmux performance acceptance plan

## Post-scroll and mouse input comparison

Three alternating 16-pane runs compared the retained style-only benchmark binary
from e8e486a against ffa98af. Both use 15 output-flood panes, one responder and
128 requests per run, waiting for workload shutdown between runs. This compares
the combined changes since that baseline (including width reuse, clipboard/paste
lifecycle, pending scroll and mouse-mode publication), not one isolated change.

| Median of three runs | Earlier binary | ffa98af |
| --- | ---: | ---: |
| Parsed flood throughput | 110.92 MiB/s | 108.71 MiB/s |
| Input-call p99 | 10.54 µs | 7.75 µs |
| Response observed in snapshot p99 | 5.06 ms | 4.87 ms |
| Snapshot-batch p99 | 5.03 ms | 4.84 ms |

Throughput ranges overlap (103.92–113.14 versus 105.42–113.25 MiB/s).
The small uncontrolled sample does not establish a statistically reliable change.
This benchmark exercises input and snapshots under flood output; it does not
dispatch mouse/scroll gestures or measure GPUI painting, visible latency, or
cross-platform behavior. Exact binary hashes and all samples are in
[the raw comparison](benchmarks/pty-post-scroll-mouse.jsonl).

## Native paste admission under output pressure

Native clipboard paste now enqueues before consulting the terminal parser. The
existing writer resolves bracketed-paste mode when it processes the request,
polling a busy parser only for that pending request and checking cancellation
between retries. Borrowed payloads allocate after obtaining the one-paste permit;
owned clipboard strings move into the queue. Neither path locks the parser on
the UI thread. Queue saturation still reports an error and releases the permit.
Automation's opportunistic paste API retains its busy/no-admission semantics.
Ten focused paste tests pass, including 896 KiB Unicode delivery with concurrent
output and ordered following input, plus held-parser native admission. These
checks do not establish GUI latency or Windows/Linux runtime acceptance.

## Cancel abandoned clipboard exports

Native copies carry a per-request cancellation guard owned by the UI task.
Dropping that task now stops the background worker's parser-lock retries, even
if no later clipboard write changes the global generation. This avoids retaining
an engine or polling indefinitely after abandoning a copy. Cancellation of an
older request cannot invalidate a newer request in another pane. Focused tests
exercise unpolled-task destruction and cancellation under a held parser lock;
this is a lifecycle fix, not a measured latency improvement. An export already
formatting under the lock still finishes its bounded (16 MiB) operation.

## Explicit process shutdown on close

Current debug source reproduced the lingering-child observation at eight panes.
Confirmed tab and workspace closure now request process shutdown immediately,
rather than relying on UI entity destruction to drop the session. Pending tab
and split launches are canceled when their workspace closes. Workspace move and
detach remove only window ownership and preserve the live sessions.

The matched two-second post-close sample changed from **one original child still
alive** to **none**, leaving only the app and survivor shell. Normal app quit left
no original children in either run. [Before/after evidence and binary hashes](benchmarks/rpc-process-close-current.jsonl).
A focused terminal test confirms shutdown completes while the Session remains
owned and its output readable; the existing live-shell RPC smoke passed transfer
and shell-state checks. These are bounded macOS observations, not a cross-platform
teardown or memory acceptance result.

## v0.1.4 process-tree observation

The optional `--process-tree` harness mode records PID/parent/start-time, RSS and
CPU accounting for the app and its descendants, without capturing process
arguments. It reports CPU deltas only for identities present at both samples and
counts appeared/disappeared processes; CPU consumed by exited processes is unknown.
RSS sums may double-count shared pages and exclude GPU allocations and services
outside the process tree. These are not physical-memory totals.

One GitHub-built v0.1.4 macOS ARM64 run with eight idle fixture shells, two-second
settling and a two-second idle window recorded **114.27 MiB** summed RSS across
nine processes, including **18.92 MiB** of child RSS. Quantized CPU accounting
showed no increment in that brief window; this is not steady-state idle acceptance.
After closing the measured workspace while retaining a fresh survivor workspace,
the sample contained three processes and **99.97 MiB** summed RSS. **One original
child was still alive** two seconds after close; none remained after normal app
quit. Surface deregistration alone therefore did not establish prompt process
teardown in this release. Investigate retention on current source before claiming
this gate passes. [Raw process-tree sample](benchmarks/rpc-process-tree-release-v0.1.4-eight-panes.jsonl).

## v0.1.4 focused eight-pane idle observation

A GitHub-built v0.1.4 macOS ARM64 binary, installed and verified through mise,
was measured in one fresh instance with eight isolated idle fixture shells,
10 seconds of settling and a 60.019-second idle window. App RSS was **94.86 MiB**;
app CPU accumulated 0.01 seconds, approximately **0.0167% of one core**. Seven
split requests had **6.07 ms median / 6.76 ms maximum** CLI/RPC completion time.
[Raw sample and binary SHA-256](benchmarks/rpc-idle-release-v0.1.4-eight-panes.jsonl).

This is one observation on an uncontrolled desktop, not full idle acceptance or
a before/after comparison. CPU uses quantized macOS ps accounting. Child/helper
CPU, child RSS, GPU resources and input-to-display latency are excluded. The
fixture app shut down normally. The harness revision in the result is f78001c
plus the pane-count/repetition option change committed with this record; the
sampling path was unchanged.

The harness accepts `--panes 8 --iterations 1` for focused runs; defaults remain
1/8/32 panes with three iterations. Both options are bounded to avoid accidentally
launching an unbounded workload.

## Published v0.1.1 macOS split baseline

The harness also accepts `--idle-seconds` (0–60) and records app process CPU-time
deltas normalized to one core. A short instrumentation check used two-second
windows after one-second settling at 1/8/32 panes, three runs each. [Raw CPU samples](benchmarks/rpc-idle-release-v0.1.1.jsonl).
The first single-pane samples varied substantially (about 18.9%, 8.9%, 1.0%);
these short post-launch windows include startup work and are **not** steady-state
idle acceptance. Child/helper CPU and GPU time are excluded; macOS ps CPU time is
quantized. Desktop/HUD conditions were uncontrolled. Use the documented quiet
60-second protocol for acceptance rather than interpreting these samples as a
regression or rerunning until a desired value appears.

An additional `--measure-close` run creates a fresh survivor workspace, closes
the measured workspace, waits until only the survivor surface is registered, and
samples RSS after another one-second interval. At 1/8/32 panes, median pre-close
RSS was 107.81/126.19/147.16 MiB and post-close RSS was 108.88/122.13/126.77 MiB.
Each size used three fresh app instances. [Raw close samples](benchmarks/rpc-close-release-v0.1.1.jsonl).
Creating the survivor adds allocations, and allocators may retain released memory;
RSS changes are not a complete leak test. Surface deregistration does not prove
child-process exit or GPU reclamation. This records cleanup behavior for idle
fixture shells without claiming a stable memory ceiling.

Measured the GitHub-built, signed macOS ARM64 v0.1.1 executable installed through
mise, with three fresh app instances per pane count and a one-second settling
interval before sampling app RSS. Median RSS was **115.11 / 123.91 / 150.78 MiB**
at **1 / 8 / 32 panes**. Pooled split-request median/max were **8.71 / 20.98 ms**
while growing to 8 panes and **12.75 / 23.34 ms** while growing to 32 panes.
[Raw samples and binary SHA-256](benchmarks/rpc-splits-release-v0.1.1.jsonl).

These are app-only RSS samples, excluding child shells and unattributed GPU
resources. Timings include CLI startup, RPC and pane creation; they do not measure
input-to-pixel latency or frame pacing. The signed release includes its updater;
this differs from the earlier debug fixture and is not a before/after comparison.
Fixture shells are idle, so these results do not establish active-output memory
or cross-platform performance acceptance.

The existing split harness now accepts `--binary`, `--cli`, `--label` and
`--settle-seconds`, so release measurements can use downloaded GitHub artifacts
without local compilation. Results identify the executable by SHA-256 and keep
the harness revision separate from the binary label.

Browser reclamation remains an unmeasured acceptance gate. Inspection of WRY
0.57.3 and objc2 0.6.5 clarified that `InnerWebView::drop`'s standalone `retain()`
calls return temporary `Retained` guards, immediately released at the semicolon.
Those lines do **not** prove a permanent native retain; the earlier concern based
on their spelling alone was incorrect. Before enabling browser panes by default,
measure repeated create/close cycles including inspector windows and WebKit helper
processes. Inspector closure is requested before host release, but its memory
effect remains unmeasured.

The browser composition probe now records weak Objective-C references on creation.
**Sample native lifetime** reports cumulative created/reclaimed views and currently
live native views without keeping them alive. Release a view, let pending native
callbacks settle, then sample again. Dead weak entries are pruned; creation stops
at 64 still-live tracked views to bound diagnostic bookkeeping. Sampling is manual,
with no idle timer. Native-object death is distinct from process RSS, WebKit helper
termination, allocator reuse or GPU memory release; capture OS metrics alongside
these counters. The probe's weak handles include a currently mounted view.

Unmounted terminal views release both shaped glyph caches and their desktop-owned
cell snapshots. A shared empty-row placeholder retains viewport dimensions; engine
contents, scrollback and processes are unchanged. The next prepaint requests a
fresh engine frame. If that nonblocking request is busy, only the background is
shown until the scheduled retry succeeds. Focused ownership tests prove old cells
are no longer retained by the placeholder and engine text survives. Native refocus
latency/blank-frame behavior and RSS impact remain unmeasured acceptance work.

Opt-in render metrics include `snapshot_release_totals` with cumulative `frames`,
`cell_capacity_bytes` and `heap_text_payload_bytes`. Accounting runs once when a
retained desktop snapshot is released, and only when diagnostics are enabled.
Repeated release/rebuild cycles count repeatedly; shared engine/paint allocations
are not deduplicated. These counters describe dropped desktop references, not
allocator frees or RSS reductions. Compare them with process/engine measurements
when assessing actual memory reclamation.

`snapshot_recovery` records `placeholder_frames` (prepaints that still lack a fresh
snapshot), `completed`, `total_ns` and `max_ns`. Timing begins on the first such
prepaint and ends when a fresh snapshot is available. Hiding the pane again cancels
the interval, so time spent hidden is excluded. No clock sampling occurs for this
metric unless diagnostics are enabled. These are snapshot availability counters,
not measured input-to-photon latency or proof that every placeholder reached the
display. Pair them with native frame/interaction observations.


Experimental browser metadata uses one queued wakeup, independently coalesced
load/title/process state, a 100 ms minimum interval between UI publications, and
change-only redraw notification. Timers are event-driven and stop at idle. This
bounds rmux chrome updates, not native WebKit page animation or callback costs.
Focused reducer tests cover duplicate suppression and sticky failure/recovery;
native callback-storm latency/CPU measurements remain pending.

Selection-specific shaped glyphs and decorations are retained only for currently
selected rows with the current foreground setting. Clearing or shrinking a
selection drops unused variants on the next rendered frame, while active variants
remain reusable. Focused ownership tests pass; native drag latency and RSS impact
have not been measured for this change.

The desktop shaped-row cache checks immutable outer snapshot identity before
scanning row identities. Unchanged snapshots take a constant-time reuse path after
font/scale/metrics/background validation. A weak identity reference avoids retaining
old row data; changed snapshots retain per-row reuse. Desktop compilation and engine
snapshot tests pass; native repaint timing remains unmeasured.
An optional selection foreground override adds one lazily shaped text variant per
selected visible row. Variants persist for reuse within the bounded row cache and
clear when the override changes/disables or rows are released. It introduces a
selection-row scan only when configured; the default path does not shape variants.
Native selection-drag latency and additional memory are not yet measured.

Snapshot glyph extraction uses a 64-byte stack buffer for the common case instead
of a scratch heap allocation per dirty frame. Larger graphemes use a per-frame
reusable overflow allocation; it is not retained by hidden terminals. Focused
Unicode/snapshot ownership coverage passes. This is an allocation-path change,
not a measured throughput, latency or process-memory improvement.
Dirty-row extraction also delays cell-vector allocation until the first changed
cell. Identical dirty rewrites keep their existing row allocation; changed rows
copy the unchanged prefix. Snapshot, resize, selection and style-identity tests
cover this path. Identical heap-backed grapheme strings are reused from the previous cell, including
style-only changes; changed text still allocates. An oversized-glyph scratch buffer
may also allocate per frame. Pointer-sharing and changed-text ownership are covered
by the Unicode snapshot test.

Snapshot allocation baseline (`366e06d` plus harness, macOS arm64 debug): 200
iterations at each of 1/8/32 engines, 120×24 cells, retaining an external frame as
the UI does. Identical ASCII rewrites incurred **zero Rust allocation calls** in
snapshot extraction across every size. Late ASCII edits used four calls / 6,032
requested bytes per frame; style changes to an 81-byte grapheme used five calls /
6,113 requested bytes. [Raw data](benchmarks/snapshot-allocations-debug.jsonl).
Repeat with `mise run bench-snapshot-allocations`. The global allocator counts Rust
alloc/alloc_zeroed/realloc traffic only while extracting snapshots; setup, parsing
and validation are outside the interval. Realloc counts the full requested size,
not the growth delta. Requested bytes are allocation traffic, not retained memory;
Ghostty's internal allocator, PTYs, UI/GPU work and process RSS are excluded.
Instrumented debug timings are included in the raw data but do not establish
release latency, frame pacing or a before/after speedup.

Native RPC split baseline (`4642803` plus harness, macOS arm64 debug, locked desktop):
three fresh app instances per target size, with idle fixture shells and breadth-first
alternating splits. Across the 7 creation requests per 8-pane run, median/max were
5.620/7.379 ms; across 31 requests per 32-pane run, 5.292/7.196 ms. These are pooled
creation requests while growing toward each target, not repeated measurements at
fixed final pane counts. Median app RSS after 1/8/32 panes was
102,240/104,896/113,888 KiB (99.84/102.44/111.22 MiB).
[Raw samples](benchmarks/rpc-splits-debug.jsonl); repeat with `mise run bench-rpc-splits`.
Timings include a fresh rmuxctl process, socket RPC, PTY/first-frame preparation and
UI insertion, and exclude initial app/workspace startup. RSS is an immediate `ps`
sample of the app only; it excludes child-shell RSS and does not attribute GPU or
allocator resources. No steady-state settling interval is imposed. The desktop
was locked, so visible rendering, frame pacing and input-to-pixel latency were not
measured. This is neither a release/platform acceptance result nor a before/after
speedup claim. All fixture processes/apps are scoped to temporary configurations.

Incremental history-probe baseline (`9aa6f0f` plus harness, macOS arm64 debug):
five batches at each of 1/8/32 terminal engines, each containing 2,000 fixture
lines and a final Unicode needle. Match medians were 0.307/1.882/5.048 ms;
miss medians were 1.387/7.836/26.331 ms. Maximum 32-terminal miss batch was
26.611 ms. [Raw summaries](benchmarks/history-probe-debug.jsonl).
Repeat with `mise run bench-history-search`. The `late_match` fixture is late
in chronological export but newest in terminal history, so it exercises early
search termination, not a worst-case match location. Misses scan the retained
fixture to completion. Timings include search registration/query/scan/cleanup,
exclude fixture setup, preview exports, PTYs and UI rendering, and have no
concurrent output. They are not release, contention, maximum-retention or
input-to-pixel acceptance measurements.

Incremental Find feeds now use opportunistic parser locking. A busy feed retains
the refresh request, backs off for 1 ms and rechecks cancellation/supersession
instead of waiting inside the feed mutex acquisition. Completed searches still
sleep on their command channel. Query replacement also uses opportunistic locking,
requeueing the query/refresh/navigation only while its revision is still current.
Find navigation also attempts the parser lock without waiting and consumes a
queued movement only after selection succeeds. Supersession is checked again
after acquisition. Search cleanup retains its existing lock behavior; this is
not a claim that every search operation is nonblocking. A contention regression verifies that
the scan resumes without changing terminal selection or viewport.

Manual process-tree sampling baseline (`22dc885` plus harness/shared-index change,
macOS arm64, debug profile): at 1/8/32 shells, warm medians were
29.025/27.865/29.464 ms after sharing parent-child indexes across terminal roots,
versus 37.818/36.553/44.938 ms before. Each variant has one cold sample and five
warm samples per size; [raw summaries](benchmarks/process-trees-debug.jsonl).
Run `mise run bench-process-trees` to repeat. This includes two host process-table
snapshots, ownership checks and RSS/CPU sampling, but excludes UI frames and
fixture creation. Fixtures are idle shells with no children, exercising the
multi-root lookup cost, not deep/wide trees or the 4,096-process cap. Host activity
can affect these sequential runs; the one-shell change also shows timing variance.
This is not release-build, memory-allocation or cross-platform acceptance evidence.
Each parent-child index is now built once per refresh instead of once per root;
aggregation also streams samples without a temporary vector per tree.

Process-tree child workload (`fd3f0c1` plus child-fixture harness, macOS arm64,
debug profile): 1/8/32 shells with four idle child processes each measured warm
medians of 42.014/41.833/50.199 ms and maxima of 42.846/46.427/51.324 ms.
Each size uses one cold and five warm samples. The sampler also includes rmux's
test process, for 6/41/161 expected samples. All root totals were complete, with
five owned processes per shell and finite nonnegative CPU values after baseline.
[Raw summaries](benchmarks/process-tree-children-debug.jsonl); repeat with
`mise run bench-process-tree-children`. Fixture startup/readiness and teardown are
outside timings; parent shells terminate and reap their children on stdin EOF,
including assertion unwind. These shallow idle trees exercise descendant
attribution beyond the prior childless fixture. They do not cover deep trees,
process churn, cap saturation, UI responsiveness, or release-build performance.
Host load differs from the earlier baseline, so these numbers are not a controlled
measurement of the incremental cost of children.

Local file-loader baseline (`1794455` plus the benchmark harness, macOS arm64,
debug profile): 40 reads of a freshly generated 2,048-entry directory measured
2.486 ms p50 / 2.608 ms p95. A 128 KiB Unicode fixture, capped to a 64 KiB read
and 512 preview lines, measured 0.085 ms p50 / 0.095 ms p95 and retained 3,584
bytes of line text. [Raw samples summary](benchmarks/file-loaders-debug.jsonl).
Run `mise run bench-file-loaders` to repeat. This is a local, mostly warm-cache
baseline, excludes fixture creation, UI construction/rendering, allocator
overhead and clipboard writes, and is not a release-build or slow-filesystem
acceptance result. Retained text bytes are not total memory usage.
This historical baseline predates Copy Preview Text: the loader now also retains
the original newline-preserving prefix (at most 64 KiB), and the benchmark's
retained-text counter includes that string alongside the per-line strings.

New-tab PTY creation runs in a background task, with one pending launch per
pane and cached directory inheritance. PTY creation plus initial frame/color preparation run off the
UI thread for new tabs. Settings changes during launch trigger background
re-preparation before the view is attached. Subsequent focus/layout work may
still access the parser. Interactive splits also use background first-frame preparation.
All RPC splits now pass a prepared terminal through publication, including
background re-preparation if colors change during startup. The native SSH fixture
covers source preservation, failure rollback and teardown, not latency or visual
acceptance. All RPC workspaces and interactive SSH/custom-command workspaces now also prepare
first frames and colors off-thread, with a ready-terminal insertion path and settings
refresh before attachment. Interactive New Workspace now also prepares off-thread and inserts relative to its
captured origin without stealing newer focus. Initial app and group-mutation startup
paths remain separate work; native
startup latency and rapid create/close acceptance are still pending.

Find selection seeding uses a nonblocking parser-lock attempt and a streaming
4 KB export. Oversized/truncated or multiline selections fall back to the
remembered query. Streaming avoids the previous full-selection size probe
before enforcing the limit; this bounds emitted text rather than guaranteeing
a particular formatting latency.
Complete selection copying also uses this streaming formatter, avoiding a full
size-probe pass and a second copy from a temporary byte buffer. Its 16 MiB
default limit and rejection of partial selections remain in place; clipboard
export continues to run in the background. Superseded exports abandon parser
lock contention through a 1 ms retry backoff; this is not an idle timer or a
guarantee of cancellation latency. Formatting already in progress is not interrupted. A focused
[selection-export comparison](benchmarks/selection-export.md) records engine-only
timing improvements and its measurement limitations.

Prepaint now uses opportunistic snapshot access: a busy parser keeps the cached
frame and schedules a display-frame retry. `snapshot_busy` counts these misses
when diagnostics are enabled. No retry is scheduled after a successful snapshot.
Resize also attempts parser access without waiting: contention leaves dimensions
unchanged and retries the latest layout size next frame. `resize_busy` records
these deferrals. Actual PTY resizing and Ghostty reflow still execute synchronously
after acquiring access; this change removes lock waiting, not reflow cost. Other
engine interactions remain separate performance work.

Output metadata updates also use opportunistic access. Contention preserves
revision cursors and retries within the existing per-terminal task after 8 ms;
there is no retry timer while idle. `metadata_busy` counts misses. Program-status
details and render-delay state are read with the same metadata lock to avoid
immediate blocking reacquisition. Synchronized-output deadline callbacks use a
nonblocking render-delay query and retry only on contention.

For opt-in renderer counters, launch with `RMUX_RENDER_METRICS` set to an output
JSONL path, then press F12 to append a snapshot of all tabs in the active
workspace. File writes run in the background; another export is ignored while a
write is pending. `cache_releases` and `rows_released` count visibility-driven
cache drops, not bytes freed. Compare these with `rows_rebuilt`, `rows_reused`
and prepaint/paint timing deltas during repeated tab, workspace and zoom changes.
Use OS footprint/RSS sampling separately; these counters do not measure GPU
atlas retention or whole-application memory.
`cursor_glyphs_reused` and `cursor_glyphs_rebuilt` count block-cursor glyph cache
hits and shaping calls. They stay unchanged for hidden cursors, bar/underline/
hollow cursors and concealed or empty covered cells. Compare deltas across
repaints and font/color changes; these counts are not latency measurements.
Engine memory collection uses a nonblocking lock attempt. `ghostty_memory: null`
means that engine was busy or its memory query failed; it never means zero usage.

`snapshot_storage` accounts for the cached Rust frame: cell-vector capacity in
bytes, logical text bytes, inline/heap cell counts and heap text payload bytes.
Inline cells include empty cells. Collection scans existing snapshots only on
explicit export, without locking the terminal or creating a fresh frame. Hidden
tabs may retain older snapshots. Counts exclude allocator/Arc overhead, shaped
glyphs and GPU storage, and do not deduplicate shared allocations across frames;
they must not be summed as process RSS.

The goal is responsive input and bounded resource use under realistic multi-pane workloads. The figures below are **initial engineering targets**, not measurements or guarantees. P0 must identify reference hardware, OS versions, display settings and reproducible fixtures before approving budgets.

An initial [release engine baseline](benchmarks/README.md) is available for 1/4/16 terminals. It isolates parsing and snapshots; none of the end-to-end application gates below are satisfied by it alone.

The first [macOS release app observation](benchmarks/app-baseline.md) covers one and eight idle clean shells, memory attribution and basic shutdown. Its CPU results exceed the provisional idle target; measurement limitations and unresolved memory growth are recorded alongside the raw evidence.

A [PTY output-pressure baseline](benchmarks/pty-baseline.md) covers 1/4/16 sessions with a responsive child alongside up to 15 flooding children. It measures input calls and response snapshots, not GUI input-to-display latency.

[Opt-in renderer diagnostics](benchmarks/render-diagnostics.md) now include native
one/eight-pane captures with zero additional terminal paints over roughly 92
seconds and approximately 0.02% frontend CPU over nested 60-second samples.
These later observations are below the provisional idle target, but uncontrolled
desktop conditions prevent attributing the difference from the original run to
a particular change. Output-pressure GUI latency and cross-platform gates remain
open.

## Measurements

| Metric | Definition | Initial gate |
| --- | --- | --- |
| Input dispatch | OS key event received to encoded bytes admitted to the PTY writer | p99 under 2 ms under the defined local workload; measure queue/drain delay separately |
| Visible input latency | Input event to corresponding displayed pixels | Measure end to end; aim for no more than one display interval above the matched baseline at p95 |
| Frame latency | Visible state change to presentation | Target the refresh interval under representative output; 16.7 ms at 60 Hz, 8.3 ms at 120 Hz |
| Workspace switch | Selection event to correct visible frame | Next available frame for an already-live workspace; no process recreation |
| Idle CPU | App and required helpers over a quiet 60-second interval | Initial target below 0.5% of one core with blink/animations disabled; normalize OS conventions |
| Startup | Process launch to responsive window, and separately to usable shell | Record cold/warm medians and p95; set absolute budget after P0, controlling shell startup config |
| Memory | App, graphics allocations where measurable, helpers and workloads | Report fixed cost and incremental visible/hidden pane cost; set absolute budgets after P0 |
| Cleanup | Resource growth after repeated creation/destruction and warm-up | No sustained retained growth; explain cache plateaus, allocator retention and OS delayed reclamation |

Screen latency must not be inferred solely from an internal dispatch timer. Use high-speed capture or a validated external latency harness when available. Record presentation timestamps separately. Report p50, p95, p99, sample count and run-to-run variance; do not hide tails in averages.

## Workload matrix

| Scenario | Setup | What it should catch |
| --- | --- | --- |
| Quiet baseline | 1, 8 and 32 idle shells; 1 or 8 visible panes | Per-pane idle timers, redraws, watchers and memory cost |
| Output pressure | One producer flooding output while typing into another pane | Input starvation, oversized queues and main-thread parsing |
| Many sessions | 64 background sessions with a defined mixture of idle/output activity | Scheduler fairness and hidden-pane costs |
| Real applications | Editor, process monitor, git UI, build logs and agent CLI | Alternate screen, cursor, mouse and bursty redraw behavior |
| Split interaction | Drag dividers and move tabs during output | Full-tree layout work, stale sizes and dropped input |
| Text coverage | Unicode/CJK/emoji/combining fixtures at multiple DPI values | Shaping cost, cache growth and correctness regressions |
| Scrollback | Fixed byte/line budgets, repeated searches and resize reflow | Duplicated buffers, unbounded search and reflow stalls |
| Graphics | Large/replaced terminal images with explicit limits | Unbounded decoded-image/GPU caches |
| Browser | 0, 1 and 8 tabs; active, hidden and reclaimed states | Startup overhead, helper growth and terminal/browser contention |
| Remote | Delay, packet loss, disconnect/reconnect and host sleep | Retry storms, stale state and input ordering |
| Lifecycle | 100 open/close cycles, repeated app restore, one-hour soak | Leaked tasks, PTYs, descriptors, processes and graphics resources |
| Event consumers | Many subscribers including deliberately slow clients | Socket backpressure starving user input |
| Provider integrations | Quiet agents and activity bursts with indexing/hooks enabled | Polling, transcript scans and child-process overhead |

## Fair comparison

Use the same fonts, DPI, window dimensions, terminal cells, theme, scrollback budget and shell startup for rmux, Ghostty and cmux. Record exact application revisions and enabled features. Native cmux desktop comparison is on macOS; on other platforms compare rmux to available native terminals with matched behavior, not an imaginary cmux binary.

Report at least three memory totals: frontend only; frontend plus its browser/remote/helper descendants; total workload including shells and agents. On macOS use physical footprint in addition to RSS; on Linux report PSS where available; on Windows record private commit/working-set measures. These OS metrics are not directly interchangeable. GPU and shared-memory accounting limitations must remain visible.

An agent using gigabytes is a workload cost even if the terminal frontend is small. Hibernating that agent changes behavior and must be a separately labelled experiment. Browser reclamation also changes reload behavior and must not be quietly enabled only for benchmark runs.

## Regression policy

Store benchmark configuration and machine-readable results by build. Initially flag a repeatable regression greater than 10% in p95/p99 latency or steady memory, then calibrate against measured noise. Reject sustained leaks, terminal corruption, lost input and idle redraw loops regardless of average scores. Do not rerun noisy benchmarks until a desired number appears; investigate variance.

Memory control requires bounded scrollback, images, event retention, notifications, RPC sizes, logs and transcript caches. Backpressure must preserve byte ordering; dropping terminal output is not an acceptable optimization.

## Lessons from cmux

cmux documents git watcher churn and the tradeoff of disabling Git/PR updates. rmux should account for metadata work as part of idle CPU, share watchers where possible and suspend unnecessary refreshes. [Shell integration](https://github.com/manaflow-ai/cmux/blob/a76816e4f0788003758b9434ab702068a640fb5d/docs/shell-integration.md), [watcher investigation](https://github.com/manaflow-ai/cmux/blob/a76816e4f0788003758b9434ab702068a640fb5d/docs/performance/15067-bash-watcher-churn.md)

Renderer and lifecycle regressions need explicit stress coverage, including hidden surfaces, queued output and destruction during rendering. [Ghostty fork notes](https://github.com/manaflow-ai/cmux/blob/a76816e4f0788003758b9434ab702068a640fb5d/docs/ghostty-fork.md)

### Bounded browser selection formatting (2026-11-21)

The direct browser adapter now uses Ghostty's synchronous streaming writer for
clipboard text as well as selection-seeded search. On overflow, copying retains
its existing OUT_OF_SPACE error; search seeding returns no query. This avoids
`format_buf` retrying the entire selection with a discarding writer to compute
its required length. Output allocation stays bounded by the requested copy limit.

A local Node 24.22.0 / pinned Ghostty WASM comparison at source 367fc83 used an
80×24 terminal, 8 MiB scrollback, 10,000 lines of 79 `x` characters plus CRLF,
select-all and a 4096-byte output limit. One warm-up preceded ten sequential
samples per path on the same terminal/engine instance:

| Path | Median | Maximum |
| --- | ---: | ---: |
| Fixed buffer with full size retry | 1.916 ms | 2.363 ms |
| Streaming, stop on overflow | 0.025 ms | 0.031 ms |

This is one local engine-boundary observation, not browser frame/input latency
or a general speedup guarantee. Existing real-WASM checks cover copy contents,
wrapped/unwrapped text, oversized-copy rejection, selection gestures, Unicode
search seeds, exact-limit seeds, multiline/oversized rejection and disposal.

### Browser engine lifecycle capacity (2026-11-21)

`mise run bench-memory` in `tools/web-probe` now measures both output-only and
search/copy workloads at 1, 4 and 16 panes over five create/write/close cycles.
Each pane is 80×24 with a 4 MiB scrollback limit and receives 164,890 bytes
(2,000 log lines). Search scans for `Sample`, selects and copies a match, and
exports the selection as a search seed. Alternating cycles explicitly close
search or destroy the terminal with its search handle still live. Each
pane-count/workload combination starts with a fresh engine instance.

On an Apple M4 Max, Node 24.22.0, engine/adapter source 04df7ba, retained linear
memory capacity after the fifth close was:

| Panes | Output only | Search and copy |
| --- | ---: | ---: |
| 1 | 3.50 MiB | 6.625 MiB |
| 4 | 10.5625 MiB | 19.25 MiB |
| 16 | 38.8125 MiB | 69.8125 MiB |

Every cycle ended with zero registered sessions. Output-only capacity stayed
constant after cycle one. Search/copy capacity increased by at most 128 KiB
across subsequent cycles and was constant for cycles three through five.
WASM linear memory does not shrink when allocations are freed; this measures
retained allocator capacity, not live allocation bytes. Stable capacity over
five cycles is not proof of no leaks. This harness excludes GPUI entities, JS
heap, fonts, GPU resources and browser overhead, so it does not validate UI
pane destruction or whole-app memory targets. Results and the exact engine
WASM hash are in [the raw measurements](benchmarks/browser-engine-memory-search-copy.jsonl).

### Native startup milestones (2026-11-21)

The split harness now accepts `--measure-startup`. Before launching the native
app it starts a monotonic clock, then records socket-path availability, the first
successful surface-list response containing a terminal, and the first text-read
response containing a marker printed by an isolated fixture shell. Polling is
bounded to ten seconds; normal app quit remains part of each run. The default
fixture is unchanged when this option is absent.

Three sequential fresh-process runs of debug source ccbf2a6 on macOS ARM64:

| Milestone | Run 1 | Run 2 | Run 3 | Median |
| --- | ---: | ---: | ---: | ---: |
| Socket available | 655.7 ms | 228.5 ms | 216.3 ms | 228.5 ms |
| First terminal registered | 733.1 ms | 232.3 ms | 219.5 ms | 232.3 ms |
| Fixture shell output readable | 736.7 ms | 235.7 ms | 222.6 ms | 235.7 ms |

[Raw samples and binary hash](benchmarks/startup-current-debug.jsonl). Reproduce
with `mise exec -- python3 scripts/bench-rpc-splits.py --panes 1 --iterations 3
--measure-startup`. The harness revision in the raw record is ccbf2a6 plus the
startup instrumentation committed with these results.

These are debug-process readiness observations, not cold/warm release statistics.
OS caches were not controlled. Socket detection has 20 ms polling granularity;
RPC milestones include CLI subprocess and request overhead. Parsed output does
not prove a presented frame, interactive window, IME readiness or normal user
shell startup. Three samples do not establish tail percentiles. Release startup
and input-to-display acceptance remain open.

The same harness also measured the installed GitHub-built v0.1.4 release, using
its bundled `rmuxctl`, on the same macOS ARM64 host:

| Milestone | Run 1 | Run 2 | Run 3 | Median |
| --- | ---: | ---: | ---: | ---: |
| Socket available | 261.6 ms | 227.7 ms | 209.5 ms | 227.7 ms |
| First terminal registered | 267.5 ms | 231.4 ms | 221.5 ms | 231.4 ms |
| Fixture shell output readable | 303.3 ms | 234.7 ms | 224.7 ms | 234.7 ms |

[Release raw samples and binary hash](benchmarks/startup-release-v0.1.4.jsonl).
All three runs exited normally. Reproduce with the command above plus
`--binary <v0.1.4-app>/Contents/MacOS/rmux
--cli <v0.1.4-app>/Contents/MacOS/rmuxctl --label release-v0.1.4`.
This establishes a shipped-binary readiness baseline with the same polling and
fixture limitations. It is not a release-versus-debug speed comparison: the
product revisions differ, and OS caches/background load were not controlled.
Cold-start and first-presented-frame acceptance still need dedicated measurement.


## Browser render-state reclamation cost

The Node/Ghostty WASM boundary benchmark now includes `reclaimed-snapshot`:
release the render adapter, recreate it and serialize a full populated 80×24
viewport, without new terminal output. Each pane count uses a fresh process,
20 warmup cycles and 100 measured cycles. Timings cover all panes in the row.

| Panes | Median | p95 | Engine linear-memory capacity, before → after |
| --- | ---: | ---: | ---: |
| 1 | 0.325 ms | 0.528 ms | 1,441,792 → 1,441,792 bytes |
| 4 | 1.285 ms | 1.562 ms | 2,949,120 → 2,949,120 bytes |
| 8 | 2.601 ms | 3.195 ms | 4,980,736 → 4,980,736 bytes |
| 16 | 5.264 ms | 6.078 ms | 8,978,432 → 8,978,432 bytes |

[Raw results and environment](benchmarks/browser-render-reclamation.jsonl).
Reproduce from `tools/web-probe` with
`mise exec -- node --expose-gc scripts/bench-native-vt.mjs 8 reclaimed-snapshot`.
The full `bench-native-vt` task also includes this scenario at 1/4/16 panes.
Unknown scenario names now fail instead of silently running the full-output case.

This measures the cost paid when discarded render state is needed again. It
excludes Rust deserialization, shaping, GPU work and browser presentation, so it
is not workspace-switch latency. Flat linear-memory capacity supports allocation
reuse during this workload, not a live-allocation or browser-RSS reduction claim.
Snapshots still transfer every visible row after reclamation; sixteen panes
produce roughly 1.8 MB of JSON per full reconstruction. Native/browser end-to-end
frame and memory acceptance remain open.


## Native queued focus under output pressure

`mise run bench-queued-focus` measures the new Session queue-admission path in a
release build while 1/4/8/16 terminals flood output. It samples 128 focus-in/out
pairs per size and waits for fixture cleanup before the next size. Focus-out
includes a held-key name in the same queue message. The optional status-details
probe uses try-lock and records misses separately.

| Flooding panes | Focus-in p99 | Focus-out p99 | Status probe busy / 128 |
| --- | ---: | ---: | ---: |
| 1 | 2.875 µs | 0.375 µs | 89 |
| 4 | 4.875 µs | 0.583 µs | 22 |
| 8 | 7.666 µs | 0.709 µs | 5 |
| 16 | 6.792 µs | 0.709 µs | 6 |

[Raw results and binary/source hashes](benchmarks/pty-queued-focus.jsonl).
All queue admissions succeeded; output continued and fixture cleanup completed.
The legacy `bench-focus` mode is retained for its synchronous call path, rather
than silently changing the meaning of prior measurements. These new timings end
at queue admission: parser work and PTY delivery occur later. They exclude GPUI
focus dispatch, other callbacks, frame presentation and actual terminal input
response. This is one uncontrolled local run, not an end-to-end acceptance gate.

## Plain-key dispatch without parser contention

The PTY reader publishes the report-all keyboard flag after each completed
output batch. `Session::key_with_text` reads that atomic flag instead of locking
the parser to decide between native text composition and terminal key events.
Report-all presses retain committed text in the bounded input queue and encode
on the existing writer thread. Disabling report-all before a queued press is
encoded falls back to its committed bytes, preserving the handled character.
Native dead-key, Option-as-text and composition guards remain in the UI host.

The contention fixture now exercises both ordinary text and report-all admission
while another thread owns the parser. A real PTY fixture enables report-all,
receives the shifted-A sequence and disables the mode again. These are functional
and lock-avoidance checks; they do not measure end-to-end key-to-pixel latency.
The reader adds one keyboard-mode query per parsed output batch. A current PTY
pressure baseline is recorded below; its isolated throughput impact still needs
a matched comparison, and native IME interaction still needs acceptance.

### Queued keyboard pressure measurement

`mise run bench-queued-keyboard` measures 128 dispatches per mode with 1/4/8/16
continuously outputting PTYs. Unlike the older keyboard mode's synchronous
release call, this mode uses the native UI's queued press/release path. Unix
children use raw/no-echo input and drain received bytes to avoid filling an
unread canonical input buffer. Every workload is reaped before the next size.

macOS arm64 release run, terminal source 212d230:

| Busy panes | Native text routing p99 (µs) | Report-all text admission p99 (µs) | Report-all Up admission p99 (µs) |
| --- | ---: | ---: | ---: |
| 1 | 0.125 | 0.250 | 3.459 |
| 4 | 0.125 | 0.375 | 5.875 |
| 8 | 0.166 | 0.292 | 17.209 |
| 16 | 0.333 | 0.709 | 7.666 |

All admissions succeeded, parsed output advanced in every case, and workload
cleanup completed. Dispatches run sequentially (Up, release, text), so these
columns have different preceding work and should not be read as equivalent
independent key costs. There is no matched before/after throughput comparison.
The 128-sample results exclude writer completion, native composition, rendering
and presentation, and do not establish a key-to-pixel latency target.
[Raw results, platform and source/binary hashes](benchmarks/pty-queued-keyboard.jsonl).

### PTY pressure after queued input changes

November 21 macOS arm64 release run, source 374b4c5 with the benchmark cleanup
change identified by its source hash. One responding PTY plus continuous output
in every other pane, 160×48 cells, 128 requests per pane count. The harness now
waits for all shutdown handles before starting the next count, preventing
asynchronous cleanup from overlapping workloads.

| Total panes | Aggregate flood MiB/s | Input admission p99 | Response snapshot p99 | Snapshot batch p99 |
| --- | ---: | ---: | ---: | ---: |
| 1 | — | 14.3 µs | 1.26 ms | 0.81 ms |
| 4 | 298.98 | 6.2 µs | 0.70 ms | 0.68 ms |
| 8 | 118.42 | 8.1 µs | 1.41 ms | 1.39 ms |
| 16 | 103.57 | 11.8 µs | 5.94 ms | 5.91 ms |

All 512 responses arrived, every flood producer advanced and all workload
shutdowns completed within the five-second bound. Snapshot observation includes
sampling the busy panes before checking the response. This single uncontrolled
run excludes the GUI, shaping and presentation; it is neither key-to-pixel
latency nor a matched before/after regression comparison. The throughput fall
at larger pane counts warrants profiling, not an attribution from this run.
[Raw results and provenance](benchmarks/pty-post-queued-input.jsonl).

### Repeated-cell style reuse

Native snapshots now reuse the resolved style/color tuple for consecutive
identical raw cells within one row. This removes repeated style queries and
color calculations for blank padding and repeated glyphs without allocating a
cache or retaining it across frames. Full raw-cell equality includes packed
background colors; row-local lifetime avoids page-local style-ID collisions.

Three alternating 16-pane runs per release binary, 128 requests each, yielded
median per-run snapshot-batch p99 of 4.77 ms before and 4.20 ms after. Median
aggregate flood throughput was 102.47 versus 102.13 MiB/s, effectively unchanged
in this small sample. Snapshot p99 ranges overlapped (4.65–5.71 ms before,
4.01–4.75 ms after), so these are observations rather than a guaranteed speedup.
All runs completed and reaped their workloads. The comparison excludes GUI
rendering/presentation and does not control other host activity.
[Alternating runs and binary/source hashes](benchmarks/pty-row-style-cache.jsonl).

The same row-local raw-cell equality now also reuses character-width metadata,
skipping another engine query for repeated cells. Grapheme text remains a
per-cell lookup: equal packed base characters can have different combining
extensions. The regression fixture covers acute/grave variants and wide tails.
This follow-up has correctness coverage but no separate timing claim; the
comparison above measures the earlier style-only change.

### Clearing scrollback under output load

The desktop Clear Scrollback action admits a command to the existing bounded
writer queue without taking the parser lock. The worker clears history under
that lock and emits a completion wakeup, including on a quiet terminal. This
avoids UI-thread contention; it does not make the engine operation itself
instantaneous. Queue saturation is reported instead of blocking. Focused tests
cover parser-lock contention, visible-screen preservation, input ordering and
the wakeup, but native click-to-redraw timing has not been measured.
