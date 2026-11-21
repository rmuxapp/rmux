//! Ordered pane tabs and selection, independent of UI entities or sessions.
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
