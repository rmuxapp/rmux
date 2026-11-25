//! Opt-in, bounded counters. No timers, frame logs or automatic file writes.
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

static NEXT_PANE: AtomicU64 = AtomicU64::new(1);

#[derive(Default)]
pub struct ExportState(pub bool);
impl gpui::Global for ExportState {}

pub fn destination() -> Option<PathBuf> {
    std::env::var_os("RMUX_RENDER_METRICS")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
}

#[derive(Default)]
pub struct Timing {
    count: u64,
    total_ns: u128,
    max_ns: u128,
}
impl Timing {
    pub fn record(&mut self, elapsed: Duration) {
        let ns = elapsed.as_nanos();
        self.count += 1;
        self.total_ns += ns;
        self.max_ns = self.max_ns.max(ns);
    }
}

/// Cumulative logical storage whose desktop references were dropped. Engine or
/// paint-frame references may still own allocations; these are not RSS savings.
#[derive(Default, serde::Serialize)]
pub struct SnapshotReleases {
    frames: u64,
    cell_capacity_bytes: u64,
    heap_text_payload_bytes: u64,
}
impl SnapshotReleases {
    pub fn record(&mut self, frame: &rmux_terminal::Frame) {
        let storage = frame.storage();
        self.frames = self.frames.saturating_add(1);
        self.cell_capacity_bytes = self.cell_capacity_bytes.saturating_add(storage.cell_capacity_bytes as u64);
        self.heap_text_payload_bytes = self.heap_text_payload_bytes.saturating_add(storage.heap_text_payload_bytes as u64);
    }
}

/// Time from the first placeholder prepaint to a fresh snapshot, excluding
/// intervals canceled by hiding the pane again. This is not input-to-photon time.
#[derive(Default, serde::Serialize)]
pub struct SnapshotRecovery {
    placeholder_frames: u64,
    completed: u64,
    total_ns: u128,
    max_ns: u128,
    #[serde(skip)]
    started: Option<Instant>,
}
impl SnapshotRecovery {
    pub fn observe(&mut self, placeholder: bool, now: Instant) {
        if placeholder {
            self.placeholder_frames = self.placeholder_frames.saturating_add(1);
            self.started.get_or_insert(now);
        } else if let Some(started) = self.started.take() {
            let elapsed = now.saturating_duration_since(started).as_nanos();
            self.completed = self.completed.saturating_add(1);
            self.total_ns = self.total_ns.saturating_add(elapsed);
            self.max_ns = self.max_ns.max(elapsed);
        }
    }
    pub fn hide(&mut self) { self.started = None; }
}

pub struct Metrics {
    id: u64,
    started: Instant,
    pub notifications: u64,
    pub hidden_output_invalidations_skipped: u64,
    pub snapshot_busy: u64,
    pub metadata_busy: u64,
    pub resize_busy: u64,
    pub rows_reused: u64,
    pub rows_rebuilt: u64,
    pub cursor_glyphs_reused: u64,
    pub cursor_glyphs_rebuilt: u64,
    pub cache_releases: u64,
    pub rows_released: u64,
    pub snapshot_releases: SnapshotReleases,
    pub snapshot_recovery: SnapshotRecovery,
    pub prepaint: Timing,
    pub paint: Timing,
}
impl Metrics {
    /// Bounded, read-only counters for automation sampling. No engine lock,
    /// snapshot construction, or file IO is needed to observe render activity.
    pub fn counters(&self, output_bytes: u64) -> serde_json::Value {
        serde_json::json!({
            "parsed_output_bytes": output_bytes,
            "output_notifications": self.notifications,
            "hidden_output_invalidations_skipped": self.hidden_output_invalidations_skipped,
            "prepaint_calls": self.prepaint.count,
            "prepaint_total_ns": self.prepaint.total_ns,
            "paint_calls": self.paint.count,
            "paint_total_ns": self.paint.total_ns,
            "rows_reused": self.rows_reused,
            "rows_rebuilt": self.rows_rebuilt,
            "cache_releases": self.cache_releases,
            "snapshot_busy": self.snapshot_busy,
            "metadata_busy": self.metadata_busy,
            "resize_busy": self.resize_busy,
        })
    }
    pub fn enabled() -> Option<Self> {
        destination().map(|_| Self::new())
    }
    fn new() -> Self {
        Self {
            id: NEXT_PANE.fetch_add(1, Ordering::Relaxed),
            started: Instant::now(),
            notifications: 0,
            hidden_output_invalidations_skipped: 0,
            snapshot_busy: 0,
            metadata_busy: 0,
            resize_busy: 0,
            rows_reused: 0,
            rows_rebuilt: 0,
            cursor_glyphs_reused: 0,
            cursor_glyphs_rebuilt: 0,
            cache_releases: 0,
            rows_released: 0,
            snapshot_releases: SnapshotReleases::default(),
            snapshot_recovery: SnapshotRecovery::default(),
            prepaint: Timing::default(),
            paint: Timing::default(),
        }
    }
    pub fn snapshot(
        &self,
        timestamp_ms: u128,
        output_bytes: u64,
        focused: bool,
        active: bool,
        frame: &rmux_terminal::Frame,
        memory: Option<rmux_terminal::MemoryUsage>,
    ) -> String {
        let columns = frame.columns;
        let rows = frame.rows.len();
        let storage = frame.storage();
        let snapshot_storage = serde_json::json!({
            "cell_capacity_bytes": storage.cell_capacity_bytes,
            "text_bytes": storage.text_bytes,
            "inline_cells": storage.inline_cells,
            "heap_cells": storage.heap_cells,
            "heap_text_payload_bytes": storage.heap_text_payload_bytes,
        });
        let memory_json = memory
            .map(|m| {
                serde_json::json!({
                    "primary_resident_bytes": m.primary_resident_bytes,
                    "alternate_resident_bytes": m.alternate_resident_bytes,
                    "virtual_bytes": m.virtual_bytes,
                    "compressed_bytes": m.compressed_bytes,
                    "image_bytes": m.image_bytes,
                    "compression_supported": m.compression_supported,
                })
            })
            .unwrap_or(serde_json::Value::Null);
        let snapshot_release_totals = serde_json::to_value(&self.snapshot_releases).expect("serialize snapshot counters");
        let snapshot_recovery = serde_json::to_value(&self.snapshot_recovery).expect("serialize recovery counters");
        let snapshot_busy = self.snapshot_busy;
        let metadata_busy = self.metadata_busy;
        let resize_busy = self.resize_busy;
        let cache_releases = self.cache_releases;
        let rows_released = self.rows_released;
        let cursor_glyphs_reused = self.cursor_glyphs_reused;
        let cursor_glyphs_rebuilt = self.cursor_glyphs_rebuilt;
        let mut output = format!(
            "{{\"timestamp_ms\":{timestamp_ms},\"pid\":{},\"pane_id\":{},\"elapsed_ms\":{},\"columns\":{columns},\"rows\":{rows},\"pane_focused\":{focused},\"window_active\":{active},\"parsed_output_bytes\":{output_bytes},\"output_notifications\":{},\"rows_reused\":{},\"rows_rebuilt\":{},\"prepaint_calls\":{},\"prepaint_total_ns\":{},\"prepaint_max_ns\":{},\"paint_calls\":{},\"paint_total_ns\":{},\"paint_max_ns\":{},\"ghostty_memory\":{memory_json},\"cache_releases\":{cache_releases},\"rows_released\":{rows_released},\"cursor_glyphs_reused\":{cursor_glyphs_reused},\"cursor_glyphs_rebuilt\":{cursor_glyphs_rebuilt}}}\n",
            std::process::id(),
            self.id,
            self.started.elapsed().as_millis(),
            self.notifications,
            self.rows_reused,
            self.rows_rebuilt,
            self.prepaint.count,
            self.prepaint.total_ns,
            self.prepaint.max_ns,
            self.paint.count,
            self.paint.total_ns,
            self.paint.max_ns
        );
        // Append the bounded snapshot accounting object before the closing brace.
        output.truncate(output.len() - 2);
        output.push_str(&format!(",\"resize_busy\":{resize_busy},\"metadata_busy\":{metadata_busy},\"snapshot_busy\":{snapshot_busy},\"snapshot_storage\":{snapshot_storage},\"snapshot_release_totals\":{snapshot_release_totals},\"snapshot_recovery\":{snapshot_recovery}}}\n"));
        output
    }
}

#[cfg(test)]
mod tests {
    use super::{SnapshotReleases, SnapshotRecovery};
    #[test]
    fn automation_sampling_preserves_cumulative_render_counters() {
        let mut metrics = super::Metrics::new();
        metrics.prepaint.record(std::time::Duration::from_micros(20));
        metrics.paint.record(std::time::Duration::from_micros(10));
        metrics.rows_rebuilt = 3;
        let first = metrics.counters(1024);
        assert_eq!(metrics.counters(1024), first);
        metrics.paint.record(std::time::Duration::from_micros(5));
        let second = metrics.counters(2048);
        assert_eq!(second["paint_calls"], 2);
        assert_eq!(second["paint_total_ns"], 15000);
        assert_eq!(second["prepaint_calls"], 1);
        assert_eq!(second["rows_rebuilt"], 3);
        assert_eq!(second["parsed_output_bytes"], 2048);
    }
    #[test]
    fn snapshot_recovery_excludes_hidden_time_and_counts_placeholder_frames() {
        use std::time::{Instant, Duration};
        let start = Instant::now();
        let mut recovery = SnapshotRecovery::default();
        recovery.observe(false, start);
        recovery.observe(true, start);
        recovery.observe(true, start + Duration::from_millis(5));
        recovery.observe(false, start + Duration::from_millis(10));
        assert_eq!(recovery.placeholder_frames, 2);
        assert_eq!(recovery.completed, 1);
        assert_eq!(recovery.total_ns, 10_000_000);
        recovery.observe(true, start + Duration::from_millis(20));
        recovery.hide();
        recovery.observe(false, start + Duration::from_secs(30));
        assert_eq!(recovery.completed, 1);
        assert_eq!(recovery.max_ns, 10_000_000);
        let value = serde_json::to_value(&recovery).unwrap();
        assert_eq!(value["placeholder_frames"], 3);
        assert!(value.get("started").is_none());
    }
    #[test]
    fn snapshot_release_accounting_is_cumulative_and_serializable() -> anyhow::Result<()> {
        let mut terminal = rmux_terminal::Terminal::new(40, 3)?;
        terminal.write("retained 世界".as_bytes());
        let frame = terminal.frame()?;
        let storage = frame.storage();
        assert!(storage.cell_capacity_bytes > 0);
        let mut releases = SnapshotReleases::default();
        releases.record(&frame);
        releases.record(&frame);
        let value = serde_json::to_value(&releases)?;
        assert_eq!(value["frames"], 2);
        assert_eq!(value["cell_capacity_bytes"], (2 * storage.cell_capacity_bytes) as u64);
        assert_eq!(value["heap_text_payload_bytes"], (2 * storage.heap_text_payload_bytes) as u64);
        assert_eq!(terminal.frame()?.text(), frame.text());
        Ok(())
    }
}
