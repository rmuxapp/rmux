/// Lower scores rank first. Callers normalize case once for each title and query.
/// Each query word may match anywhere; contiguous matches beat subsequences.
pub fn score(title: &str, words: &[&str]) -> Option<(usize, usize, usize)> {
    let mut fuzzy = 0;
    let mut gaps = 0;
    let mut starts = 0;
    for word in words {
        if let Some(start) = title.find(word) {
            // Use the same scalar positions as the subsequence path. UTF-8
            // byte lengths must not penalize names with non-ASCII prefixes.
            starts += title[..start].chars().count();
            continue;
        }
        fuzzy += 1;
        let mut chars = title.chars().enumerate();
        let mut previous = None;
        for wanted in word.chars() {
            let (position, _) = chars.find(|(_, ch)| *ch == wanted)?;
            if let Some(previous) = previous {
                gaps += position - previous - 1;
            } else {
                starts += position;
            }
            previous = Some(position);
        }
    }
    Some((fuzzy, gaps, starts))
}

/// Rank tab titles and directories independently for each word. Prefer title
/// matches of equal quality, without allowing a fuzzy word to cross fields.
pub fn tab_score(number: usize, title: &str, directory: &str, words: &[&str]) -> Option<(usize, usize, usize, usize)> {
    let mut total = (0, 0, 0, 0);
    for word in words {
        // Displayed tab numbers are exact tokens, not fuzzy substrings: 3
        // should target tab 3, rather than also matching tab 13 or tab 30.
        if !word.is_empty() && word.bytes().all(|byte| byte.is_ascii_digit())
            && (word.len() == 1 || !word.starts_with('0'))
            && word.parse::<usize>().ok() == Some(number) { continue; }
        let title_score = score(title, &[*word]).map(|(fuzzy, gaps, start)| (fuzzy, gaps, 1, start));
        let directory_score = score(directory, &[*word]).map(|(fuzzy, gaps, start)| (fuzzy, gaps, 2, start));
        let best = match (title_score, directory_score) {
            (Some(a), Some(b)) => a.min(b),
            (Some(a), None) | (None, Some(a)) => a,
            (None, None) => return None,
        };
        total.0 += best.0;
        total.1 += best.1;
        total.2 += best.2;
        total.3 += best.3;
    }
    Some(total)
}

/// Search over an immutable palette snapshot. Title-only appended queries can
/// narrow previous matches; exact shortcut tokens must reconsider candidates.
pub struct RankedSearch {
    titles: Vec<String>,
    document: bool,
    shortcuts: Vec<String>,
    details: Vec<String>,
    query: String,
    complete_candidates: bool,
    ranked: Vec<((usize, usize, usize), usize)>,
    pub matches: Vec<usize>,
}
impl RankedSearch {
    pub fn new(titles: Vec<String>) -> Self {
        Self { matches: (0..titles.len()).collect(), titles, document: false, shortcuts: Vec::new(), details: Vec::new(), query: String::new(), complete_candidates: true, ranked: Vec::new() }
    }
    /// Literal word filtering for document rows. Preserve reading order and
    /// use the same contiguous matches as preview highlighting.
    pub fn document(titles: Vec<String>) -> Self {
        Self { document: true, ..Self::new(titles) }
    }
    /// Shortcut aliases are exact tokens, never fuzzy continuations of a title.
    pub fn with_shortcuts(titles: Vec<String>, shortcuts: Vec<String>) -> Self {
        assert_eq!(titles.len(), shortcuts.len());
        if shortcuts.iter().all(String::is_empty) { return Self::new(titles); }
        Self { shortcuts, ..Self::new(titles) }
    }
    /// Keep descriptive paths/metadata searchable without treating scattered
    /// characters there as command abbreviations. Each word stays in one field.
    pub fn with_details(titles: Vec<String>, details: Vec<String>, shortcuts: Vec<String>) -> Self {
        assert_eq!(titles.len(), details.len());
        Self { details, ..Self::with_shortcuts(titles, shortcuts) }
    }
    /// Query-dependent filters may reintroduce candidates even when text is
    /// appended (e.g. a shortcut query becomes a command-name query).
    pub fn update_filtered(&mut self, query: &str, mut include: impl FnMut(usize) -> bool) {
        let normalized = query.to_lowercase();
        let words: Vec<_> = normalized.split_whitespace().collect();
        self.ranked.clear();
        if words.is_empty() {
            // Every empty-query score ties; retain source order directly.
            // Reapply the filter even for repeated empty queries: availability
            // can change independently of the text (e.g. workspace selection).
            self.matches.clear();
            self.matches.extend((0..self.titles.len()).filter(|index| include(*index)));
            self.query = normalized;
            self.complete_candidates = false;
            return;
        }
        self.ranked.extend(self.titles.iter().enumerate().filter_map(|(index, title)| {
            if !include(index) { return None; }
            item_score(title, self.details.get(index).map(String::as_str), self.shortcuts.get(index).map(String::as_str), &words).map(|score| (score, index))
        }));
        self.ranked.sort_unstable();
        self.matches.clear();
        self.matches.extend(self.ranked.iter().map(|(_, index)| *index));
        // A subsequent ordinary update must reconsider all candidates.
        self.query = normalized;
        self.complete_candidates = false;
    }
    pub fn update(&mut self, query: &str) {
        let normalized = query.to_lowercase();
        if self.complete_candidates && normalized == self.query { return; }
        let words: Vec<_> = normalized.split_whitespace().collect();
        if self.document {
            self.matches.clear();
            self.matches.extend(self.titles.iter().enumerate().filter_map(|(index, title)|
                words.iter().all(|word| title.contains(word)).then_some(index)));
        } else if words.is_empty() {
            self.matches.clear();
            self.matches.extend(0..self.titles.len());
        } else {
            self.ranked.clear();
            if self.shortcuts.is_empty() && self.complete_candidates && !self.query.is_empty() && normalized.starts_with(&self.query) {
                self.ranked.extend(self.matches.iter().filter_map(|index|
                    item_score(&self.titles[*index], self.details.get(*index).map(String::as_str), None, &words).map(|score| (score, *index))));
            } else {
                self.ranked.extend(self.titles.iter().enumerate().filter_map(|(index, title)|
                    item_score(title, self.details.get(index).map(String::as_str), self.shortcuts.get(index).map(String::as_str), &words).map(|score| (score, index))));
            }
            self.ranked.sort_unstable();
            self.matches.clear();
            self.matches.extend(self.ranked.iter().map(|(_, index)| *index));
        }
        self.query = normalized;
        self.complete_candidates = true;
    }
}

fn item_score(title: &str, detail: Option<&str>, shortcut: Option<&str>, words: &[&str]) -> Option<(usize, usize, usize)> {
    let Some(detail) = detail else { return command_score(title, shortcut, words); };
    let mut total = (0, 0, 0);
    for word in words {
        let name = command_score(title, shortcut, &[*word]);
        let description = detail.find(word).map(|start|
            (0, 0, title.chars().count() + 1 + detail[..start].chars().count()));
        let best = match (name, description) {
            (Some(a), Some(b)) => a.min(b),
            (Some(a), None) | (None, Some(a)) => a,
            (None, None) => return None,
        };
        total.0 += best.0;
        total.1 += best.1;
        total.2 += best.2;
    }
    Some(total)
}

fn command_score(title: &str, shortcuts: Option<&str>, words: &[&str]) -> Option<(usize, usize, usize)> {
    let Some(shortcuts) = shortcuts.filter(|text| !text.is_empty()) else { return score(title, words); };
    let mut total = (0, 0, 0);
    for word in words {
        let name = score(title, &[*word]);
        let binding = shortcuts.split_whitespace().position(|token| token == *word).map(|position| (0, 0, position));
        let best = match (name, binding) {
            (Some(a), Some(b)) => a.min(b),
            (Some(a), None) | (None, Some(a)) => a,
            (None, None) => return None,
        };
        total.0 += best.0;
        total.1 += best.1;
        total.2 += best.2;
    }
    Some(total)
}

#[cfg(test)]
mod tests {
    use super::{score, tab_score, RankedSearch};

    #[test]
    fn command_details_match_literal_words_without_path_noise() {
        let titles = vec!["next pane".into(), "switch to terminal: notes".into(), "build project".into()];
        let details = vec![String::new(), "/private/nested/example/text/pane".into(), "/work/東京 café".into()];
        let mut search = RankedSearch::with_details(titles.clone(), details.clone(), vec![String::new(); 3]);
        search.update("next pane");
        assert_eq!(search.matches, [0]);
        search.update("bld café");
        assert_eq!(search.matches, [2]);
        search.update("notes example");
        assert_eq!(search.matches, [1]);
        search.update("notesexample");
        assert!(search.matches.is_empty());
        // Incremental filtering must use the same field rules as a fresh query.
        for query in ["n", "ne", "next", "next p", "next pane", "", "東京", "CAFÉ"] {
            search.update(query);
            let mut fresh = RankedSearch::with_details(titles.clone(), details.clone(), vec![String::new(); 3]);
            fresh.update(query);
            assert_eq!(search.matches, fresh.matches, "{query}");
        }
        let mut shortcuts = RankedSearch::with_details(titles, details, vec!["cmd ]".into(), String::new(), String::new()]);
        shortcuts.update_filtered("cmd pane", |_| true);
        assert_eq!(shortcuts.matches, [0]);
        shortcuts.update("café");
        assert_eq!(shortcuts.matches, [2]);
    }

    #[test]
    fn document_search_keeps_source_order_and_literal_unicode_matches() {
        let mut search = RankedSearch::document(vec![
            "prefix needle 世界".into(), "needle 世界".into(),
            "n e e d l e 世界".into(), "needle unrelated".into(),
        ]);
        search.update("NEEDLE");
        assert_eq!(search.matches, [0, 1, 3]);
        search.update("NEEDLE 世界");
        assert_eq!(search.matches, [0, 1]);
        search.update("NEEDLE 世界");
        assert_eq!(search.matches, [0, 1]);
        search.update("needle");
        assert_eq!(search.matches, [0, 1, 3]);
        search.update("missing");
        assert!(search.matches.is_empty());
        search.update("");
        assert_eq!(search.matches, [0, 1, 2, 3]);
        // Ordinary command search still prefers the earlier contiguous match.
        let mut commands = RankedSearch::new(vec!["prefix needle".into(), "needle".into()]);
        commands.update("needle");
        assert_eq!(commands.matches, [1, 0]);
    }

    #[test]
    fn clearing_filtered_search_preserves_order_and_refreshes_availability() {
        let mut search = RankedSearch::new(vec!["zebra".into(), "alpha".into(), "beta".into()]);
        search.update("beta");
        assert_eq!(search.matches, vec![2]);
        search.update_filtered(" ", |index| index != 1);
        assert_eq!(search.matches, vec![0, 2]);
        search.update_filtered(" ", |index| index == 1);
        assert_eq!(search.matches, vec![1]);
        search.update("");
        assert_eq!(search.matches, vec![0, 1, 2]);
    }
    #[test]
    fn shortcut_tokens_do_not_create_fuzzy_command_names() {
        let mut search = RankedSearch::with_shortcuts(
            vec!["move tab left".into(), "new workspace group".into()],
            vec!["⌃⇧pgup ctrl control shift pgup page up".into(), String::new()]);
        search.update("group");
        assert_eq!(search.matches, [1]);
        search.update("mv tb");
        assert_eq!(search.matches, [0]);
        // Exact shortcut tokens can introduce results as typing continues.
        search.update("c");
        assert_eq!(search.matches, [1]);
        search.update("ctrl");
        assert_eq!(search.matches, [0]);
        search.update("ctrl move");
        assert_eq!(search.matches, [0]);
        search.update_filtered("group", |_| true);
        assert_eq!(search.matches, [1]);
        search.update("⌃⇧pgup");
        assert_eq!(search.matches, [0]);
        search.update("");
        assert_eq!(search.matches, [0, 1]);
    }

    #[test]
    fn query_dependent_filters_can_reintroduce_results_and_return_to_normal_search() {
        let mut search = RankedSearch::new(vec!["ctrl tab next pane".into(), "ctrl move tab".into()]);
        search.update_filtered("ctrl tab", |index| index == 0);
        assert_eq!(search.matches, [0]);
        search.update_filtered("ctrl tab move", |_| true);
        assert_eq!(search.matches, [1]);
        search.update_filtered("", |index| index == 0);
        assert_eq!(search.matches, [0]);
        search.update("");
        assert_eq!(search.matches, [0, 1]);
    }
    #[test]
    fn unicode_prefixes_use_character_positions_for_contiguous_and_fuzzy_words() {
        assert_eq!(score("界 build", &["build"]), score("x build", &["build"]));
        assert_eq!(score("界 build", &["bld"]), score("x build", &["bld"]));
        let mut search = RankedSearch::new(vec!["界 build".into(), "xx build".into()]);
        search.update("build");
        assert_eq!(search.matches, [0, 1]);
        search.update("BUILD");
        assert_eq!(search.matches, [0, 1]);
        search.update("");
        assert_eq!(search.matches, [0, 1]);
    }

    #[test]
    fn incremental_search_matches_full_ranking_through_edits() {
        let titles: Vec<String> = ["split below", "split right", "switch workspace 東京", "café build", "c a f é", "σ shell", "ς shell"]
            .into_iter().map(str::to_owned).collect();
        let mut search = RankedSearch::new(titles.clone());
        for query in ["s", "sp", "spl", "splt", "splt b", "splt blw", "splt", "", "CAF", "CAFÉ", "café b", "c", "東", "東京", "東京 z", "東京", " ", "  s", "  sh", "Σ", "Σ ", "", "unmatched", "unmatched more", "split"] {
            search.update(query);
            let normalized = query.to_lowercase();
            let words: Vec<_> = normalized.split_whitespace().collect();
            let mut full: Vec<_> = titles.iter().enumerate().filter_map(|(index, title)| score(title, &words).map(|score| (score, index))).collect();
            full.sort_unstable();
            assert_eq!(search.matches, full.into_iter().map(|(_, index)| index).collect::<Vec<_>>(), "query: {query}");
        }
    }

    #[test]
    fn tab_numbers_are_exact_tokens_and_combine_with_metadata() {
        assert!(tab_score(3, "server", "/work", &["3", "srv"]).is_some());
        assert!(tab_score(13, "server", "/work", &["3"]).is_none());
        assert!(tab_score(3, "server", "/work", &["03"]).is_none());
        for word in ["+3", "٣", "３", "0003", "184467440737095516160"] {
            assert!(tab_score(3, "server", "/work", &[word]).is_none(), "{word}");
        }
        assert_eq!(tab_score(usize::MAX, "", "", &[&usize::MAX.to_string()]), Some((0, 0, 0, 0)));
        assert_eq!(tab_score(0, "", "", &["0"]), Some((0, 0, 0, 0)));
        // Noncanonical numbers can still match literal title text.
        assert!(tab_score(3, "server03", "", &["03"]).is_some());
        assert!(tab_score(3, "server", "/work", &["3"]) < tab_score(1, "server3", "", &["3"]));
        assert!(tab_score(1, "server3", "", &["3"]).is_some());
    }

    #[test]
    fn tab_search_combines_fields_without_crossing_them() {
        assert!(tab_score(1, "build server", "/work/rmux", &["bld", "rmx"]).is_some());
        assert!(tab_score(1, "alpha", "/beta", &["ab"]).is_none());
        assert!(tab_score(1, "東京", "/work/café", &["東", "café"]).is_some());
        assert!(tab_score(1, "rmux", "/other", &["rmux"]) < tab_score(1, "other", "/rmux", &["rmux"]));
        assert!(tab_score(1, "r m u x", "", &["rmux"]) > tab_score(1, "other", "/rmux", &["rmux"]));
        assert_eq!(tab_score(1, "anything", "/anywhere", &[]), Some((0, 0, 0, 0)));
    }

    #[test]
    fn abbreviations_rank_after_contiguous_matches() {
        let words = ["splt", "blw"];
        assert!(score("split below", &words).is_some());
        assert!(score("split right", &words).is_none());
        assert!(score("splt blw", &words) < score("split below", &words));
        assert!(
            score("split below", &["split"]).unwrap() < score("s p l i t", &["split"]).unwrap()
        );
    }

    #[test]
    fn words_match_independently_and_unicode_is_not_byte_sliced() {
        assert!(score("switch to workspace: 東京 café", &["café", "東"]).is_some());
        assert!(score("東京作業", &["東業"]).is_some());
        assert!(score("東京作業", &["業東"]).is_none());
        assert!(score("tab", &["tt"]).is_none());
        assert_eq!(score("anything", &[]), Some((0, 0, 0)));
    }
}

/// Map case-folded word matches back to displayed UTF-8 ranges. The caller uses
/// bounded UI labels or captured preview lines, never unbounded scrollback.
pub fn highlights(text: &str, query: &str) -> Vec<std::ops::Range<usize>> {
    match_highlights(text, query, false)
}

/// Highlight the contiguous or greedy subsequence match used by palette ranking.
/// Literal document searches should continue to use `highlights`.
pub fn fuzzy_highlights(text: &str, query: &str) -> Vec<std::ops::Range<usize>> {
    match_highlights(text, query, true)
}

fn match_highlights(text: &str, query: &str, fuzzy: bool) -> Vec<std::ops::Range<usize>> {
    if query.trim().is_empty() { return Vec::new(); }
    // Whole-string lowering preserves contextual final sigma, matching the
    // ranked search. Lowering each scalar separately always produces σ.
    let folded = text.to_lowercase();
    let mut folded_offset = 0;
    let mut boundaries = Vec::new();
    for (start, ch) in text.char_indices() {
        let folded_start = folded_offset;
        // Contextual σ/ς have equal UTF-8 widths; expanding mappings such as
        // İ still occupy the sum of their lowercase scalar widths.
        folded_offset += ch.to_lowercase().map(char::len_utf8).sum::<usize>();
        boundaries.push((folded_start..folded_offset, start..start + ch.len_utf8()));
    }
    let mut marked = vec![false; boundaries.len()];
    let mut mark = |matched: std::ops::Range<usize>| {
        let first = boundaries.partition_point(|(range, _)| range.end <= matched.start);
        let last = boundaries.partition_point(|(range, _)| range.start < matched.end);
        marked[first..last].fill(true);
    };
    // Only fuzzy words need temporary ranges, so an incomplete subsequence
    // can be discarded. Reuse that storage across words in this label.
    let mut matches = Vec::new();
    for word in query.split_whitespace() {
        let word = word.to_lowercase();
        let mut contiguous = false;
        for (start, matched) in folded.match_indices(&word) {
            contiguous = true;
            mark(start..start + matched.len());
        }
        if !contiguous && fuzzy {
            matches.clear();
            let mut chars = folded.char_indices();
            for wanted in word.chars() {
                let Some((start, ch)) = chars.find(|(_, ch)| *ch == wanted) else {
                    // A partial subsequence is not a match for this word.
                    matches.clear();
                    break;
                };
                matches.push(start..start + ch.len_utf8());
            }
            for matched in &matches { mark(matched.clone()); }
        }
    }
    let mut ranges: Vec<std::ops::Range<usize>> = Vec::new();
    for ((_, source), marked) in boundaries.into_iter().zip(marked) {
        if !marked { continue; }
        if let Some(previous) = ranges.last_mut() && previous.end == source.start {
            previous.end = source.end;
        } else { ranges.push(source); }
    }
    ranges
}

#[cfg(test)]
mod highlight_tests {
    use super::highlights as preview_highlights;
    #[test]
    fn fuzzy_labels_show_complete_ranked_subsequences_only() {
        let text = "Split Below";
        let ranges = super::fuzzy_highlights(text, "splt blw");
        assert_eq!(ranges.iter().map(|range| &text[range.clone()]).collect::<Vec<_>>(), ["Spl", "t", "B", "l", "w"]);
        assert!(preview_highlights(text, "splt blw").is_empty());
        assert!(super::fuzzy_highlights(text, "spltz").is_empty());
        assert_eq!(super::fuzzy_highlights(text, "splt blwz"), vec![0..3, 4..5]);
        // Ranking greedily takes the first r (in work), even for the rmx word.
        assert_eq!(super::fuzzy_highlights("/work/rmux", "wrk rmx"), vec![1..2, 3..5, 7..8, 9..10]);
        assert_eq!(super::fuzzy_highlights("s p l t splt", "splt"), vec![8..12]);
        assert_eq!(super::fuzzy_highlights("東京作業", "東業"), vec![0..3, 9..12]);
        assert_eq!(super::fuzzy_highlights("İ ΟΣ", "iς"), vec![0..2, 5..7]);
    }
    #[test]
    fn highlights_agree_with_search_for_contextual_greek_lowercase() {
        let text = "ΟΣ build";
        assert!(super::score(&text.to_lowercase(), &["ος"]).is_some());
        assert_eq!(preview_highlights(text, "ΟΣ"), vec![0..4]);
        assert_eq!(preview_highlights("İ ΟΣ ΣΟ", "ος σο"), vec![3..7, 8..12]);
    }
    #[test]
    fn preview_highlights_merge_words_and_preserve_expanding_unicode() {
        let text = "…İstanbul ÉCOLE 世界 needle";
        let ranges = preview_highlights(text, "i École 世界 need needle");
        assert_eq!(ranges.iter().map(|range| &text[range.clone()]).collect::<Vec<_>>(), ["İ", "ÉCOLE", "世界", "needle"]);
        assert!(preview_highlights(text, "   ").is_empty());
        assert!(preview_highlights(text, "absent").is_empty());
        assert_eq!(preview_highlights("abcabc", "abc bc"), vec![0..6]);
        // A match within an expanded scalar highlights the whole source scalar.
        assert_eq!(preview_highlights("İ", "\u{307}"), vec![0..2]);
    }
}
