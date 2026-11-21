//! Ordered pane tabs and selection, independent of UI entities or sessions.
/// Resolve insertion before a target after removing the source. A target equal
/// to the original length appends the tab to the strip.
pub fn drop_index(from: usize, target: usize) -> usize {
    target.saturating_sub(usize::from(from < target))
}
/// User tab titles share the same bounded normalization on every host.
/// None restores the terminal's automatic title.
pub fn custom_title(text: &str) -> Option<String> {
    let title: String = text.trim().chars().filter(|c| !c.is_control()).take(128).collect();
    (!title.is_empty()).then_some(title)
}
#[derive(Clone, Copy)]
pub enum CloseScope { Others, Right, Finished }
impl CloseScope {
    pub fn includes(self, index: usize, active: usize, finished: bool) -> bool {
        match self {
            Self::Others => index != active,
            Self::Right => index > active,
            Self::Finished => index != active && finished,
        }
    }
}
/// Revalidate captured identities after a possible confirmation delay. New tabs
/// and the kept tab survive; descending indices avoid shifting later removals.
pub fn close_indices<T: Copy + Eq>(current: &[T], keep: T, targets: &[T]) -> Vec<usize> {
    if !current.contains(&keep) { return Vec::new(); }
    current.iter().enumerate().rev()
        .filter_map(|(index, id)| (*id != keep && targets.contains(id)).then_some(index)).collect()
}
#[derive(Clone, Debug)]
pub struct Tabs<T> {
    items: Vec<T>,
    active: usize,
}
impl<T> Tabs<T> {
    pub fn new(items: Vec<T>, active: usize) -> Self {
        let active = active.min(items.len().saturating_sub(1));
        Self { items, active }
    }
    /// Zero for an empty pane; use `get` when resolving the active item.
    pub fn active_index(&self) -> usize { self.active }
    pub fn active(&self) -> Option<&T> { self.items.get(self.active) }
    /// Capture identities before any asynchronous host confirmation. Hosts supply
    /// session identity and completion status; selection policy stays shared.
    pub fn close_targets<I>(&self, scope: CloseScope, mut identity: impl FnMut(&T) -> I,
        mut finished: impl FnMut(&T) -> bool) -> Option<(I, Vec<I>)> {
        let keep = identity(self.active()?);
        let targets = self.items.iter().enumerate()
            .filter(|(index, item)| scope.includes(*index, self.active,
                matches!(scope, CloseScope::Finished) && finished(item)))
            .map(|(_, item)| identity(item)).collect();
        Some((keep, targets))
    }
    pub fn select(&mut self, index: usize) -> bool {
        if index >= self.items.len() { return false; }
        self.active = index;
        true
    }
    pub fn cycle_target(&self, delta: isize) -> Option<usize> {
        crate::layout::cycle_index(self.active, self.items.len(), delta)
    }
    /// Reordering stops at the edge; navigation wraps, movement does not.
    pub fn move_active(&mut self, delta: isize) -> Option<(usize, usize)> {
        let from = self.active;
        let to = from.checked_add_signed(delta)?;
        if to >= self.items.len() || to == from { return None; }
        self.swap(from, to);
        Some((from, to))
    }
    /// Append transferred tabs and resolve the incoming active tab. Keep the
    /// existing selection until the host finishes subscriptions/cache cleanup.
    pub fn append(&mut self, items: Vec<T>, active: usize) -> Option<usize> {
        if items.is_empty() { return None; }
        let selected = self.items.len() + active.min(items.len() - 1);
        self.items.extend(items);
        Some(selected)
    }
    pub fn push(&mut self, item: T) { self.items.push(item); }
    /// Keep the selected tab when inserting before it. The host may then select
    /// the inserted tab explicitly, after completing subscriptions and focus.
    pub fn insert(&mut self, index: usize, item: T) {
        assert!(index <= self.items.len());
        if !self.items.is_empty() && index <= self.active { self.active += 1; }
        self.items.insert(index, item);
    }
    /// Removing the active tab chooses its right neighbor, or the last tab.
    pub fn remove(&mut self, index: usize) -> T {
        let item = self.items.remove(index);
        if index < self.active { self.active -= 1; }
        self.active = self.active.min(self.items.len().saturating_sub(1));
        item
    }
    pub fn swap(&mut self, a: usize, b: usize) {
        self.items.swap(a,b);
        if self.active == a { self.active = b; }
        else if self.active == b { self.active = a; }
    }
    pub fn take_all(&mut self) -> Vec<T> {
        self.active = 0;
        std::mem::take(&mut self.items)
    }
}
// Read-only slice access cannot bypass selection reconciliation.
impl<T> std::ops::Deref for Tabs<T> {
    type Target = [T];
    fn deref(&self) -> &[T] { &self.items }
}
impl<'a,T> IntoIterator for &'a Tabs<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a,T>;
    fn into_iter(self) -> Self::IntoIter { self.items.iter() }
}
impl<T> IntoIterator for Tabs<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter { self.items.into_iter() }
}

#[cfg(test)]
mod tests {
    use super::Tabs;
    #[test]
    fn captured_close_targets_survive_selection_changes_and_new_tabs() {
        let mut tabs = Tabs::new(vec![10, 20, 30, 40], 1);
        let (keep, targets) = tabs.close_targets(super::CloseScope::Right, |id| *id,
            |_| panic!("right scope does not inspect process status")).unwrap();
        tabs.swap(0, 3);
        tabs.push(50);
        tabs.select(4);
        for index in super::close_indices(&tabs, keep, &targets) { tabs.remove(index); }
        assert_eq!(&*tabs, &[20, 10, 50]);
        assert_eq!(tabs.active(), Some(&50));
        let (_, targets) = tabs.close_targets(super::CloseScope::Finished, |id| *id, |id| *id != 10).unwrap();
        assert_eq!(targets, vec![20]);
        assert!(Tabs::<u32>::new(vec![], 0).close_targets(super::CloseScope::Others, |id| *id, |_| false).is_none());
    }
    #[test]
    fn joining_tabs_preserves_order_and_incoming_selection() {
        let mut source = Tabs::new(vec!["source-a", "source-b"], 1);
        let mut target = Tabs::new(vec!["target"], 0);
        let active = source.active_index();
        let selected = target.append(source.take_all(), active).unwrap();
        assert!(source.is_empty());
        assert_eq!(target.active(), Some(&"target"));
        target.select(selected);
        assert_eq!(&*target, &["target", "source-a", "source-b"]);
        assert_eq!(target.active(), Some(&"source-b"));
        assert_eq!(target.append(vec![], 99), None);
        assert_eq!(target.active(), Some(&"source-b"));
    }
    #[test]
    fn moving_keeps_the_same_tab_selected_without_wrapping() {
        let mut tabs=Tabs::new(vec!["a","b","c"],0);
        assert_eq!(tabs.move_active(-1),None);
        assert_eq!(tabs.move_active(1),Some((0,1)));
        assert_eq!(&*tabs,&["b","a","c"]);
        assert_eq!(tabs.active(),Some(&"a"));
        assert_eq!(tabs.move_active(1),Some((1,2)));
        assert_eq!(tabs.move_active(1),None);
        assert_eq!(tabs.move_active(isize::MIN),None);
        assert_eq!(tabs.active(),Some(&"a"));
    }
    #[test]
    fn removal_preserves_survivors_and_selects_neighbors() {
        let mut tabs=Tabs::new(vec!["a","b","c","d"],2);
        tabs.remove(0);
        assert_eq!(tabs[tabs.active_index()],"c");
        tabs.remove(1);
        assert_eq!(tabs[tabs.active_index()],"d");
        tabs.remove(1);
        assert_eq!(tabs[tabs.active_index()],"b");
        tabs.remove(0);
        assert_eq!(tabs.active_index(),0);
        assert_eq!(tabs.cycle_target(1),None);
        assert!(!tabs.select(0));
        tabs.push("new");
        assert_eq!(tabs[tabs.active_index()],"new");
    }
    #[test]
    fn transfer_reorder_restore_and_wrap_keep_selection_valid() {
        let mut tabs=Tabs::new(vec!["a","b"],99);
        assert_eq!(tabs.active_index(),1);
        tabs.insert(0,"incoming");
        assert_eq!(tabs[tabs.active_index()],"b");
        tabs.swap(2,0);
        assert_eq!(tabs[tabs.active_index()],"b");
        assert_eq!(tabs.cycle_target(-1),Some(2));
        assert!(!tabs.select(100));
        assert_eq!(tabs.active_index(),0);
        assert_eq!(tabs.take_all(),vec!["b","a","incoming"]);
        assert_eq!(tabs.active_index(),0);
        tabs.insert(0,"restored");
        assert_eq!(tabs.active_index(),0);
    }
}
