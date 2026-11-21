/// Session-local search history: at most eight queries and 16 KiB per terminal.
#[derive(Clone, Default)]
pub struct QueryHistory {
    entries: Vec<String>,
    cursor: Option<usize>,
    draft: String,
}
impl QueryHistory {
    pub fn remember(&mut self, query: &str) {
        self.cursor = None;
        self.draft.clear();
        if query.is_empty() || query.len() > 4096 { return; }
        self.entries.retain(|entry| entry != query);
        self.entries.insert(0, query.to_owned());
        while self.entries.len() > 8 || self.entries.iter().map(String::len).sum::<usize>() > 16 * 1024 {
            self.entries.pop();
        }
    }
    pub fn edited(&mut self, query: &str) {
        if self.cursor.is_some_and(|index| self.entries[index] != query) {
            self.cursor = None;
            self.draft.clear();
        }
    }
    pub fn recall(&mut self, older: bool, current: &str) -> Option<String> {
        if self.entries.is_empty() || current.len() > 4096 { return None; }
        if older {
            let next = match self.cursor {
                Some(index) => (index + 1).min(self.entries.len() - 1),
                None => {
                    self.draft = current.to_owned();
                    usize::from(self.entries[0] == current).min(self.entries.len() - 1)
                }
            };
            self.cursor = Some(next);
            Some(self.entries[next].clone())
        } else {
            match self.cursor? {
                0 => { self.cursor = None; Some(std::mem::take(&mut self.draft)) }
                index => { self.cursor = Some(index - 1); Some(self.entries[index - 1].clone()) }
            }
        }
    }
}

#[cfg(test)]
mod history_tests {
    use super::QueryHistory;
    #[test]
    fn recall_preserves_draft_and_editing_starts_a_new_navigation() {
        let mut history = QueryHistory::default();
        history.remember("older 世界");
        history.remember("newer");
        assert_eq!(history.recall(true, "draft"), Some("newer".into()));
        history.edited("newer");
        assert_eq!(history.recall(true, "newer"), Some("older 世界".into()));
        assert_eq!(history.recall(false, "older 世界"), Some("newer".into()));
        assert_eq!(history.recall(false, "newer"), Some("draft".into()));
        assert_eq!(history.recall(false, "draft"), None);
        assert_eq!(history.recall(true, "draft"), Some("newer".into()));
        history.edited("changed");
        assert_eq!(history.recall(false, "changed"), None);
        assert_eq!(history.recall(true, "changed"), Some("newer".into()));
        assert_eq!(history.recall(false, "newer"), Some("changed".into()));
    }
    #[test]
    fn query_history_is_deduplicated_and_byte_bounded() {
        let mut history = QueryHistory::default();
        for index in 0..10 { history.remember(&format!("query {index}")); }
        assert_eq!(history.entries.len(), 8);
        history.remember("query 3");
        assert_eq!(history.entries[0], "query 3");
        assert_eq!(history.entries.len(), 8);
        for index in 0..8 { history.remember(&format!("{index}{}", "界".repeat(1365))); }
        assert_eq!(history.entries.len(), 4);
        assert_eq!(history.entries.iter().map(String::len).sum::<usize>(), 16384);
        let before = history.entries.clone();
        history.remember("");
        history.remember(&"x".repeat(4097));
        assert_eq!(history.entries, before);
    }
}
