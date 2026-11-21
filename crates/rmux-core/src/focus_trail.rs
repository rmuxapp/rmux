/// Bounded navigation history, separate from the last-terminal MRU toggle.
pub struct FocusTrail<T> {
    entries: Vec<T>,
    cursor: usize,
}

impl<T> Default for FocusTrail<T> {
    fn default() -> Self { Self { entries: Vec::new(), cursor: 0 } }
}

impl<T: Copy + Eq> FocusTrail<T> {
    pub fn record(&mut self, target: T) {
        if self.entries.get(self.cursor) == Some(&target) { return; }
        if !self.entries.is_empty() { self.entries.truncate(self.cursor + 1); }
        self.entries.push(target);
        if self.entries.len() > 64 { self.entries.remove(0); }
        self.cursor = self.entries.len() - 1;
    }

    pub fn step(&mut self, forward: bool, live: impl Fn(T) -> bool) -> Option<T> {
        let current = self.entries.get(self.cursor).copied();
        let mut index = self.cursor;
        loop {
            index = if forward { index.checked_add(1)? } else { index.checked_sub(1)? };
            let target = *self.entries.get(index)?;
            if Some(target) != current && live(target) {
                self.cursor = index;
                return Some(target);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FocusTrail;
    #[test]
    fn back_forward_and_new_visit_branch() {
        let mut trail = FocusTrail::default();
        for id in [1, 2, 3] { trail.record(id); }
        assert_eq!(trail.step(false, |_| true), Some(2));
        trail.record(2); // Focus notification from navigation must keep forward history.
        assert_eq!(trail.step(true, |_| true), Some(3));
        assert_eq!(trail.step(false, |_| true), Some(2));
        trail.record(4);
        assert_eq!(trail.step(true, |_| true), None);
        assert_eq!(trail.step(false, |_| true), Some(2));
        assert_eq!(trail.step(false, |_| true), Some(1));
        assert_eq!(trail.step(false, |_| true), None);
    }
    #[test]
    fn closed_targets_are_skipped_and_history_is_bounded() {
        let mut trail = FocusTrail::default();
        assert_eq!(trail.step(false, |_| true), None);
        for id in 0..100 { trail.record(id); }
        assert_eq!(trail.entries.len(), 64);
        assert_eq!(trail.step(false, |id| id < 97), Some(96));
        assert_eq!(trail.step(true, |id| id == 99), Some(99));
        assert_eq!(trail.step(false, |_| false), None);
        assert_eq!(trail.step(true, |_| true), None);
    }
}
