# Browser engine/JSON boundary baseline

Measured on Apple M4 Max, macOS arm64, Node 24.22.0, ghostty-web 0.4.0.

Run `mise run bench-browser-engine` in `tools/web-probe` after `mise run build`.
Each matrix entry runs in a fresh Node process with 20 warmup and 100 measured
iterations. Every pane is 80×24. Timing covers writing ANSI output and producing
the sparse JSON snapshot for all panes. Validation and JSON parsing happen outside
the timed region. These are not browser input latency or frame-time measurements.

| Panes | Update | Median ms | p95 ms | JSON bytes/update |
| ---: | --- | ---: | ---: | ---: |
| 1 | clean-snapshot | 0.017 | 0.046 | 189 |
| 1 | one-row | 0.045 | 0.064 | 19,305 |
| 1 | full-viewport | 0.476 | 0.572 | 458,988 |
| 4 | clean-snapshot | 0.075 | 0.081 | 757 |
| 4 | one-row | 0.148 | 0.177 | 77,221 |
| 4 | full-viewport | 1.953 | 2.214 | 1,835,953 |
| 16 | clean-snapshot | 0.278 | 0.302 | 3,028 |
| 16 | one-row | 0.574 | 0.690 | 308,884 |
| 16 | full-viewport | 7.930 | 9.135 | 7,343,812 |

The dense 16-pane case transfers about 7.34 MB per update before Rust parses it.
Repeated field names and default style values dominate the cell representation.
Next: compact the wire format while preserving dirty-row updates and unchanged
row identities. Re-run this same workload to compare payload and serialization cost.

Clean-snapshot means an explicit API call with no changed rows. The UI does not
poll this API while idle, so it is not an idle CPU measurement. One-row changes
only row zero; full-viewport rewrites all rows without adding scrollback.

The JSONL includes whole-process memory before and after forced garbage collection.
That includes Node, loaded WASM and benchmark data; it is neither per-pane memory
nor a transient allocation peak. Fresh processes avoid cross-case heap reuse.
GPU rendering, font shaping, Rust deserialization, browser scheduling, input-to-paint
latency, actual browser memory and long-session retention still need measurement.

Raw results: [browser-engine-json.jsonl](browser-engine-json.jsonl).
