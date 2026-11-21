//! Standalone optimized benchmark; no GPUI window or terminal engine required.
#[allow(dead_code)]
#[path = "../crates/rmux-core/src/palette_match.rs"]
mod palette_match;
use std::{hint::black_box, time::Instant};
const QUERIES: &[&str] = &["", "s", "sw", "swi", "switch", "switch b", "switch bu", "switch build", "switch", "s", "", "東京", "東京 b"];
fn full(titles: &[String], query: &str) -> Vec<usize> {
    let query = query.to_lowercase();
    let words: Vec<_> = query.split_whitespace().collect();
    let mut ranked: Vec<_> = titles.iter().enumerate().filter_map(|(index, title)|
        palette_match::score(title, &words).map(|score| (score, index))).collect();
    ranked.sort_unstable();
    ranked.into_iter().map(|(_, index)| index).collect()
}
fn measure(titles: &[String], incremental: bool) -> f64 {
    let mut search = palette_match::RankedSearch::new(titles.to_vec());
    // Exclude opening the palette and initial allocations from typing timings.
    for query in QUERIES { search.update(query); black_box(full(titles, query)); }
    let start = Instant::now();
    for _ in 0..32 {
        for query in QUERIES {
            if incremental { search.update(black_box(query)); black_box(&search.matches); }
            else { black_box(full(black_box(titles), black_box(query))); }
        }
    }
    start.elapsed().as_secs_f64() * 1e6 / (32 * QUERIES.len()) as f64
}
fn main() {
    for count in [128, 1024, 8192] {
        let titles: Vec<_> = (0..count).map(|i| match i % 4 {
            0 => format!("switch build terminal {i} /work/project-{i}"),
            1 => format!("move workspace {i} /work/frontend"),
            2 => format!("switch server terminal {i} /work/api"),
            _ => format!("東京 build {i} /work/café"),
        }).collect();
        let mut search = palette_match::RankedSearch::new(titles.clone());
        for query in QUERIES {
            search.update(query);
            assert_eq!(search.matches, full(&titles, query));
        }
        for sample in 0..3 {
            let (baseline, incremental) = if sample % 2 == 0 {
                (measure(&titles, false), measure(&titles, true))
            } else {
                let incremental = measure(&titles, true);
                (measure(&titles, false), incremental)
            };
            println!("{{\"candidates\":{count},\"sample\":{sample},\"full_us_per_query\":{baseline:.3},\"incremental_us_per_query\":{incremental:.3},\"ratio\":{:.3}}}", incremental / baseline);
        }
    }
}
