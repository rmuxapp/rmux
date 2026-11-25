//! Explicit, bounded searches of terminal snapshots; never scans on keystrokes.
use std::sync::atomic::{AtomicBool, Ordering};
use gpui::EntityId;

pub const MAX_TERMINALS: usize = 256;
pub const MAX_BYTES: usize = 64 * 1024;
static RUNNING: AtomicBool = AtomicBool::new(false);
pub struct Permit;
impl Permit {
    pub fn acquire() -> Option<Self> {
        RUNNING.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).ok().map(|_| Self)
    }
}
impl Drop for Permit { fn drop(&mut self) { RUNNING.store(false, Ordering::Release); } }
pub struct Job {
    pub window: Option<gpui::AnyWindowHandle>,
    pub workspace: EntityId,
    pub terminal: EntityId,
    pub name: String,
    pub group: String,
    pub read: rmux_terminal::session::HistoryExport,
    pub probe: rmux_terminal::session::HistoryProbe,
}
pub struct Hit { pub window: Option<gpui::AnyWindowHandle>, pub workspace: EntityId, pub terminal: EntityId, pub name: String, pub group: String, pub snippet: String, pub preview: bool }
#[derive(Default)]
pub struct Report { pub hits: Vec<Hit>, pub scanned: usize, pub unavailable: usize, pub limited: usize }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress { pub previews: usize, pub total: usize, pub matches: usize, pub deep_remaining: usize }
pub fn validate(query: &str) -> Result<(), &'static str> {
    if query.trim().is_empty() || query.len() > 4096 || query.chars().any(char::is_control) {
        Err("Enter 1–4096 bytes of search text without control characters")
    } else { Ok(()) }
}
fn matching_line(text: &str, query: &str) -> Option<String> {
    // Match Ghostty Find: ASCII case-insensitive, other Unicode bytes exact.
    text.lines().find_map(|line| {
        let offset = line.to_ascii_lowercase().find(query)?;
        // ASCII folding preserves byte offsets. Keep context before the literal
        // match, rather than before an unrelated occurrence of its first word.
        let start = line[..offset].char_indices().rev().nth(31).map_or(0, |(index, _)| index);
        let mut chars = line[start..].chars();
        let mut snippet = String::new();
        if start > 0 { snippet.push('…'); }
        snippet.extend(chars.by_ref().take(1024).map(|ch| if ch.is_control() { ' ' } else { ch }));
        if chars.next().is_some() { snippet.push('…'); }
        Some(snippet)
    })
}
#[cfg(test)]
fn scan(jobs: Vec<Job>, query: &str, canceled: &AtomicBool) -> Report {
    scan_with_progress(jobs, query, canceled, |_| {})
}
pub fn scan_with_progress(jobs: Vec<Job>, query: &str, canceled: &AtomicBool, mut publish: impl FnMut(Progress)) -> Report {
    let mut report = Report::default();
    let mut progress = Progress { previews: 0, total: jobs.len().min(MAX_TERMINALS), matches: 0, deep_remaining: 0 };
    publish(progress);
    let query = query.to_ascii_lowercase();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut pending = Vec::new();
    let mut hits = Vec::new();
    // Collect inexpensive snapshots before spending the shared budget on deep
    // scans. Retain only identity and the weak engine probe, not snapshot text.
    for (index, job) in jobs.into_iter().take(MAX_TERMINALS).enumerate() {
        if canceled.load(Ordering::Relaxed) { break; }
        if std::time::Instant::now() >= deadline { report.limited += 1; continue; }
        let Job { window, workspace, terminal, name, group, read, probe } = job;
        let mut hit = Hit { window, workspace, terminal, name, group, snippet: String::new(), preview: false };
        match read() {
            Ok(Some((text, limited))) => {
                report.scanned += 1;
                if let Some(snippet) = matching_line(&text, &query) {
                    hit.snippet = snippet;
                    hit.preview = true;
                    hits.push((index, hit));
                } else if limited { pending.push((index, hit, probe)); }
            }
            Ok(None) => { report.scanned += 1; pending.push((index, hit, probe)); }
            _ => report.unavailable += 1,
        }
        progress.previews += 1;
        progress.matches = hits.len();
        progress.deep_remaining = pending.len();
        publish(progress);
    }
    for (index, mut hit, probe) in pending {
        if canceled.load(Ordering::Relaxed) { break; }
        progress.deep_remaining = progress.deep_remaining.saturating_sub(1);
        if std::time::Instant::now() >= deadline { report.limited += 1; publish(progress); continue; }
        let terminal_deadline = deadline.min(std::time::Instant::now() + std::time::Duration::from_millis(250));
        match probe(&query, canceled, terminal_deadline) {
            Ok(Some(true)) => {
                hit.snippet = "Match in retained history · Enter opens Find".into();
                hits.push((index, hit));
            }
            Ok(Some(false)) => {},
            Ok(None) => report.limited += 1,
            Err(_) => report.unavailable += 1,
        }
        progress.matches = hits.len();
        publish(progress);
    }
    hits.sort_unstable_by_key(|(index, _)| *index);
    report.hits = hits.into_iter().map(|(_, hit)| hit).collect();
    report
}
#[cfg(test)]
mod tests {
    #[test]
    fn excerpts_center_on_literal_phrase_with_ascii_case_rules() {
        let line = format!("needle unrelated {} NEEDLE 世界 tail", "x".repeat(1500));
        let excerpt = super::matching_line(&line, "needle 世界").unwrap();
        assert!(excerpt.starts_with('…'));
        assert!(excerpt.contains("NEEDLE 世界 tail"));
        assert!(!excerpt.contains("unrelated"));
        let line = format!("é {} É needle\tend", "界".repeat(1500));
        let excerpt = super::matching_line(&line, "É needle").unwrap();
        assert!(excerpt.contains("É needle end"));
        assert!(excerpt.chars().count() <= 1026);
        assert_eq!(super::matching_line("é needle", "É needle"), None);
    }
    #[test]
    fn quick_matches_precede_deep_work_but_results_keep_terminal_order() {
        use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
        let reads = Arc::new(AtomicUsize::new(0));
        let jobs = (0..3).map(|index| {
            let read_count = reads.clone();
            let probe_count = reads.clone();
            super::Job { window: None, workspace: gpui::EntityId::from(0x100000001), terminal: gpui::EntityId::from(0x100000002), name: index.to_string(), group: "Workspace".into(),
                read: Box::new(move || {
                    read_count.fetch_add(1, Ordering::Relaxed);
                    Ok(Some((if index == 2 { "needle" } else { "prefix" }.into(), true)))
                }),
                probe: Box::new(move |_, _, _| {
                    assert_eq!(probe_count.load(Ordering::Relaxed), 3, "deep work must not delay later previews");
                    assert_ne!(index, 2, "a preview hit needs no deep scan");
                    Ok(Some(true))
                }),
            }
        }).collect();
        let mut updates = Vec::new();
        let report = super::scan_with_progress(jobs, "needle", &super::AtomicBool::new(false), |progress| updates.push(progress));
        assert_eq!(updates.first().unwrap().previews, 0);
        assert!(updates.contains(&super::Progress { previews: 3, total: 3, matches: 1, deep_remaining: 2 }));
        assert_eq!(updates.last(), Some(&super::Progress { previews: 3, total: 3, matches: 3, deep_remaining: 0 }));
        assert_eq!(report.hits.iter().map(|hit| hit.name.as_str()).collect::<Vec<_>>(), ["0", "1", "2"]);
        assert_eq!(report.limited, 0);
    }
    #[test]
    fn truncated_misses_probe_history_and_report_unfinished_scans() {
        let jobs = (0..3).map(|index| super::Job {
            window: None, workspace: gpui::EntityId::from(0x100000001), terminal: gpui::EntityId::from(0x100000002), name: index.to_string(), group: "Workspace".into(),
            read: Box::new(|| Ok(Some(("ordinary prefix".into(), true)))),
            probe: Box::new(move |query, _, _| {
                assert_eq!(query, "needle");
                Ok(match index { 0 => Some(true), 1 => Some(false), _ => None })
            }),
        }).collect();
        let report = super::scan(jobs, "NEEDLE", &super::AtomicBool::new(false));
        assert_eq!((report.scanned, report.limited, report.unavailable, report.hits.len()), (3, 1, 0, 1));
        assert_eq!(report.hits[0].name, "0");
        assert!(report.hits[0].snippet.contains("retained history"));
    }
    #[test]
    fn snapshots_report_partial_busy_and_canceled_work() {
        use std::sync::{Arc, atomic::{AtomicUsize, AtomicBool, Ordering}};
        let calls = Arc::new(AtomicUsize::new(0));
        let jobs = |count| (0..count).map(|index| {
            let calls = calls.clone();
            super::Job { window: None, workspace: gpui::EntityId::from(0x100000001), terminal: gpui::EntityId::from(0x100000002), name: index.to_string(), group: "Workspace".into(),
                probe: Box::new(|_, _, _| Ok(None)),
                read: Box::new(move || { calls.fetch_add(1, Ordering::Relaxed); match index {
                    0 => Ok(Some(("NEEDLE 世界".into(), true))), 1 => Ok(None), _ => anyhow::bail!("closed"),
                } }) }
        }).collect();
        let report = super::scan(jobs(3), "needle", &AtomicBool::new(false));
        assert_eq!((report.scanned, report.limited, report.unavailable, report.hits.len()), (2, 1, 1, 1));
        assert_eq!(report.hits[0].name, "0");
        calls.store(0, Ordering::Relaxed);
        assert!(super::scan(jobs(3), "needle", &AtomicBool::new(true)).hits.is_empty());
        assert_eq!(calls.load(Ordering::Relaxed), 0);
        super::scan(jobs(300), "needle", &AtomicBool::new(false));
        assert_eq!(calls.load(Ordering::Relaxed), super::MAX_TERMINALS);
    }
    #[test]
    fn literal_search_matches_find_case_rules_and_bounds_snippets() {
        assert!(super::matching_line("before\nBUILD 世界\nafter", "build 世界").unwrap().contains("BUILD 世界"));
        assert!(super::matching_line("CAFÉ", "café").is_none());
        assert!(super::matching_line("a.b", "a.b").is_some());
        assert!(super::matching_line("axb", "a.b").is_none());
        let line = format!("{}needle{}", "x".repeat(3000), "y".repeat(3000));
        let snippet = super::matching_line(&line, "needle").unwrap();
        assert!(snippet.contains("needle") && snippet.chars().count() <= 1026);
        for query in ["", "  ", "a\nb", "\0"] { assert!(super::validate(query).is_err()); }
        assert!(super::validate(&"x".repeat(4097)).is_err());
    }
}
