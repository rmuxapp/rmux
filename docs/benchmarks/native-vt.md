# Direct libghostty-vt WASM adapter benchmark

Measured 2026-11-21 using `mise run bench-native-vt` in tools/web-probe.
The pinned desktop engine is libghostty-vt 1.0.1, built ReleaseSmall. Each
case runs in its own Node process: 80×24 cells per pane, 20 warmups and
100 measured updates. Raw environment and memory data are in
[native-vt-before.jsonl](native-vt-before.jsonl) and
[native-vt-after.jsonl](native-vt-after.jsonl).

The baseline is the direct adapter at 609a7a6. The optimized adapter caches
its DataView while checking for WASM memory growth, reads ordinary codepoints
from the ABI-described packed cell, and reuses consecutive styles within a row.
Combining graphemes retain the UTF-8 buffer path; background-only cells bypass
style reuse. Style IDs never persist across rows/pages or snapshots.

| Panes | Update | Before median | After median | After p95 |
|---:|---|---:|---:|---:|
| 1 | clean-snapshot | 0.003 ms | 0.002 ms | 0.007 ms |
| 1 | one-row | 0.078 ms | 0.022 ms | 0.074 ms |
| 1 | full-viewport | 1.376 ms | 0.310 ms | 0.438 ms |
| 4 | clean-snapshot | 0.008 ms | 0.006 ms | 0.010 ms |
| 4 | one-row | 0.310 ms | 0.080 ms | 0.161 ms |
| 4 | full-viewport | 6.060 ms | 1.282 ms | 1.563 ms |
| 16 | clean-snapshot | 0.032 ms | 0.028 ms | 0.035 ms |
| 16 | one-row | 1.057 ms | 0.296 ms | 0.391 ms |
| 16 | full-viewport | 24.418 ms | 5.650 ms | 6.594 ms |

Dense 16-pane median cost fell about 77%, from 24.418 to 5.650 ms.
Payload bytes and dirty-row counts are unchanged in all nine scenarios.

Scope: synchronous engine writes, cell extraction and JSON serialization.
Excludes Rust deserialization, shaping, GPU rendering, browser presentation and
input-to-display latency. Whole-process Node memory is not browser/per-pane
memory or a transient-allocation peak. These are synthetic workloads, not a
claim of end-to-end native performance. Historical ghostty-web results use a
different engine and wire payload; do not compare them as equivalent workloads.
