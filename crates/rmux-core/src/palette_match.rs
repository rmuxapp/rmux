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

/// Search over an immutable palette snapshot. Appended normalized queries can
/// only remove matches; arbitrary edits must consider every title again.
pub struct RankedSearch {
    titles: Vec<String>,
    query: String,
    ranked: Vec<((usize, usize, usize), usize)>,
    pub matches: Vec<usize>,
}
impl RankedSearch {
    pub fn new(titles: Vec<String>) -> Self {
        Self { matches: (0..titles.len()).collect(), titles, query: String::new(), ranked: Vec::new() }
    }
    pub fn update(&mut self, query: &str) {
        let normalized = query.to_lowercase();
        if normalized == self.query { return; }
        let words: Vec<_> = normalized.split_whitespace().collect();
        if words.is_empty() {
            self.matches.clear();
            self.matches.extend(0..self.titles.len());
        } else {
            self.ranked.clear();
            if !self.query.is_empty() && normalized.starts_with(&self.query) {
                self.ranked.extend(self.matches.iter().filter_map(|index|
                    score(&self.titles[*index], &words).map(|score| (score, *index))));
            } else {
                self.ranked.extend(self.titles.iter().enumerate().filter_map(|(index, title)|
                    score(title, &words).map(|score| (score, index))));
            }
            self.ranked.sort_unstable();
            self.matches.clear();
            self.matches.extend(self.ranked.iter().map(|(_, index)| *index));
        }
        self.query = normalized;
    }
}

#[cfg(test)]
mod tests {
    use super::{score, tab_score, RankedSearch};

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
