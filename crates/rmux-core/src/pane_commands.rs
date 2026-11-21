//! Pane command transitions shared by desktop and browser application hosts.
use crate::layout::{Direction, PaneTree};

/// Shared layout ownership. Hosts retain session/view handles and process guards;
/// the split tree and zoom target belong to the application model.
pub struct State<T, Z> {
    pub layout: Option<PaneTree<T>>,
    pub zoomed: Option<Z>,
}
impl<T, Z> Default for State<T, Z> {
    fn default() -> Self { Self { layout: None, zoomed: None } }
}
impl<T: Clone + PartialEq, Z> State<T, Z> {
    /// Accept layout ownership before a host registers the new session/view.
    /// Failed insertions preserve both the existing tree and zoom target.
    pub fn insert(&mut self, target: Option<&T>, pane: T, axis: crate::layout::Axis, before: bool) -> Option<usize> {
        let ordinal = insert(&mut self.layout, target, pane, axis, before)?;
        self.zoomed = None;
        Some(ordinal)
    }
    /// Release a pane without touching process lifetime or input focus.
    /// A stale identity leaves the layout (including zoom) unchanged.
    pub fn remove(&mut self, pane: &T) -> Option<usize> {
        let ordinal = self.layout.as_ref()?.pane_index(pane)?;
        self.layout = self.layout.take().and_then(|tree| tree.remove(pane));
        self.zoomed = None;
        Some(ordinal)
    }
    pub fn swap_target(&self, focused: &T, direction: Option<Direction>) -> Option<T> {
        swap_target(self.layout.as_ref()?, focused, direction, self.zoomed.is_some())
    }
    /// Commit only the layout change; hosts keep focus/session ownership attached
    /// to identity and update any traversal-order view list afterward.
    pub fn swap(&mut self, first: &T, second: &T) -> bool {
        if !self.layout.as_mut().is_some_and(|tree| tree.swap(first, second)) { return false; }
        self.zoomed = None;
        true
    }
    pub fn swap_direction(&mut self, focused: &T, direction: Option<Direction>) -> bool {
        let Some(target) = self.swap_target(focused, direction) else { return false; };
        self.swap(focused, &target)
    }
    pub fn resize(&mut self, focused: &T, direction: Direction) -> bool {
        resize(&mut self.layout, focused, direction, self.zoomed.is_some())
    }
    pub fn focus_changed(&mut self, focused: Z) {
        if self.zoomed.is_some() { self.zoomed = Some(focused); }
    }
    pub fn equalize(&mut self) { equalize(&mut self.layout, &mut self.zoomed); }
    pub fn toggle_zoom(&mut self, focused: Option<Z>) {
        self.zoomed = toggle_zoom(self.zoomed.take(), focused, self.layout.as_ref().map_or(0, PaneTree::pane_count));
    }
}

/// Zoom only a focused pane in a split layout. Invoking again exits zoom.
pub fn toggle_zoom<T>(current: Option<T>, focused: Option<T>, pane_count: usize) -> Option<T> {
    if current.is_some() || pane_count < 2 {
        None
    } else {
        focused
    }
}

/// A zoomed pane cannot resize the splits hidden behind it.
pub fn resize<T: Clone + PartialEq>(
    tree: &mut Option<PaneTree<T>>,
    focused: &T,
    direction: Direction,
    zoomed: bool,
) -> bool {
    !zoomed
        && tree
            .as_mut()
            .is_some_and(|tree| tree.resize_direction(focused, direction))
}

/// Equalizing reveals the full layout, even if its weights were already equal.
pub fn equalize<T: Clone + PartialEq, Z>(tree: &mut Option<PaneTree<T>>, zoomed: &mut Option<Z>) {
    if let Some(tree) = tree {
        tree.equalize();
        *zoomed = None;
    }
}

/// Commit a new pane to the layout and return its visual ordinal. Rejected
/// targets, duplicate identities and depth limits leave ownership unchanged.
/// Hosts create their session first, then register it only after this succeeds.
pub fn insert<T: Clone + PartialEq>(tree: &mut Option<PaneTree<T>>, target: Option<&T>, pane: T, axis: crate::layout::Axis, before: bool) -> Option<usize> {
    match tree {
        None => {
            if target.is_some() { return None; }
            *tree = Some(PaneTree::Pane(pane));
            Some(0)
        }
        Some(tree) => {
            let target = target?;
            if tree.pane_index(&pane).is_some() { return None; }
            let index = tree.pane_index(target)? + usize::from(!before);
            tree.split_positioned(target, pane, axis, before).then_some(index)
        }
    }
}

/// Cycle in visual layout order, independent of host entity storage order.
pub fn cycle<T: Clone + PartialEq>(tree: &PaneTree<T>, focused: Option<&T>, direction: isize) -> Option<T> {
    let current = focused.and_then(|pane| tree.pane_index(pane)).unwrap_or(0);
    tree.pane_at(crate::layout::cycle_index(current, tree.pane_count(), direction)?).cloned()
}

/// Resolve the desktop swap policy. Directional swaps are unavailable while
/// zoomed; cycling to the next pane can still swap and then reveal the layout.
pub fn swap_target<T: Clone + PartialEq>(tree: &PaneTree<T>, focused: &T, direction: Option<Direction>, zoomed: bool) -> Option<T> {
    if tree.pane_count() < 2 || tree.pane_index(focused).is_none() { return None; }
    match direction {
        Some(_) if zoomed => None,
        Some(direction) => tree.neighbor(focused, direction),
        None => cycle(tree, Some(focused), 1),
    }
}

/// Layout activity that makes transferring pane ownership unsafe. Session-specific
/// guards (pending processes, editors) remain the host's responsibility.
#[derive(Clone, Copy, Debug, Default)]
pub struct JoinState {
    pub zoomed: bool,
    pub resizing: bool,
    pub splitting: bool,
}

/// Resolve a join before either host releases terminal ownership. A missing or
/// single pane is a no-op; a busy layout returns a shared user-facing error.
pub fn join_target<T: Clone + PartialEq>(
    tree: &PaneTree<T>, focused: &T, direction: Option<Direction>, state: JoinState,
) -> Result<Option<T>, &'static str> {
    if tree.pane_count() < 2 || tree.pane_index(focused).is_none() {
        return Ok(None);
    }
    if state.zoomed || state.resizing || state.splitting {
        return Err("Finish resizing or splitting, and unzoom before joining panes.");
    }
    Ok(match direction {
        Some(direction) => tree.neighbor(focused, direction),
        None => cycle(tree, Some(focused), 1),
    })
}

/// Decide focus before removing ownership. Closing an inactive pane preserves
/// focus; closing the active pane selects the following pane, or the previous
/// pane when the last leaf closes. The final pane has no successor.
pub fn focus_after_close<T: Clone + PartialEq>(tree: &PaneTree<T>, removed: &T, focused: Option<&T>) -> Option<T> {
    if let Some(focused) = focused.filter(|pane| *pane != removed && tree.pane_index(pane).is_some()) {
        return Some(focused.clone());
    }
    let index = tree.pane_index(removed)?;
    tree.pane_at(index + 1).or_else(|| index.checked_sub(1).and_then(|index| tree.pane_at(index))).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::Axis;

    #[test]
    fn swap_commit_preserves_zoom_on_rejection_and_reveals_on_success() {
        let mut state = State::<i32, i32>::default();
        state.insert(None, 1, Axis::Columns, false);
        state.insert(Some(&1), 2, Axis::Columns, false);
        state.toggle_zoom(Some(1));
        assert!(!state.swap_direction(&1, Some(Direction::Right)));
        assert!(!state.swap(&1, &99));
        assert_eq!(state.zoomed, Some(1));
        assert_eq!(state.layout.as_ref().unwrap().pane_at(0), Some(&1));
        assert!(state.swap_direction(&1, None));
        assert_eq!(state.zoomed, None);
        assert_eq!(state.layout.as_ref().unwrap().pane_at(1), Some(&1));
        assert!(state.swap_direction(&1, Some(Direction::Left)));
        assert_eq!(state.layout.as_ref().unwrap().pane_at(0), Some(&1));
        assert!(!state.swap_direction(&1, Some(Direction::Left)));
    }

    #[test]
    fn pane_ownership_rejects_stale_requests_without_disturbing_zoom() {
        let mut state = State::<i32, i32>::default();
        assert_eq!(state.insert(None, 1, Axis::Columns, false), Some(0));
        assert_eq!(state.insert(Some(&1), 2, Axis::Columns, false), Some(1));
        state.toggle_zoom(Some(2));
        assert_eq!(state.insert(Some(&99), 3, Axis::Rows, false), None);
        assert_eq!(state.insert(Some(&1), 2, Axis::Rows, false), None);
        assert_eq!(state.remove(&99), None);
        assert_eq!(state.zoomed, Some(2));
        assert_eq!(state.layout.as_ref().unwrap().pane_count(), 2);
        assert_eq!(state.insert(Some(&2), 3, Axis::Rows, true), Some(1));
        assert_eq!(state.zoomed, None);
        assert_eq!(state.layout.as_ref().unwrap().pane_at(2), Some(&2));
        state.toggle_zoom(Some(3));
        assert_eq!(state.remove(&3), Some(1));
        assert_eq!(state.zoomed, None);
        assert_eq!(state.layout.as_ref().unwrap().pane_at(1), Some(&2));
        state.remove(&1);
        state.remove(&2);
        assert!(state.layout.is_none());
        assert_eq!(state.insert(None, 4, Axis::Columns, false), Some(0));
    }

    #[test]
    fn owned_layout_keeps_zoom_focus_and_resize_policy_together() {
        let mut state = State { layout: Some(PaneTree::Split {
            axis: Axis::Columns, weights: vec![2., 1.],
            children: vec![PaneTree::Pane(1), PaneTree::Pane(2)],
        }), zoomed: None };
        state.toggle_zoom(Some(1));
        state.focus_changed(2);
        assert_eq!(state.zoomed, Some(2));
        assert!(!state.resize(&2, Direction::Left));
        state.equalize();
        assert_eq!(state.zoomed, None);
        if let Some(PaneTree::Split { weights, .. }) = &state.layout {
            assert_eq!(weights, &vec![1., 1.]);
        } else { panic!("split layout was lost"); }
        state.focus_changed(1);
        assert_eq!(state.zoomed, None);
        state.layout = Some(PaneTree::Pane(1));
        state.toggle_zoom(Some(1));
        assert_eq!(state.zoomed, None);
    }

    #[test]
    fn join_resolves_visual_order_and_rejects_busy_layouts() {
        let tree = PaneTree::Split { axis: Axis::Columns, weights: vec![1., 1.], children: vec![
            PaneTree::Pane(30), PaneTree::Split { axis: Axis::Rows, weights: vec![1., 1.],
                children: vec![PaneTree::Pane(10), PaneTree::Pane(20)] },
        ] };
        assert_eq!(join_target(&tree, &20, None, JoinState::default()), Ok(Some(30)));
        assert_eq!(join_target(&tree, &10, Some(Direction::Down), JoinState::default()), Ok(Some(20)));
        assert_eq!(join_target(&tree, &30, Some(Direction::Left), JoinState::default()), Ok(None));
        assert_eq!(join_target(&tree, &99, None, JoinState::default()), Ok(None));
        assert_eq!(join_target(&PaneTree::Pane(30), &30, None, JoinState::default()), Ok(None));
        for state in [
            JoinState { zoomed: true, ..Default::default() },
            JoinState { resizing: true, ..Default::default() },
            JoinState { splitting: true, ..Default::default() },
        ] {
            assert!(join_target(&tree, &10, None, state).is_err());
            assert!(join_target(&tree, &10, Some(Direction::Down), state).is_err());
        }
        assert_eq!(tree.pane_count(), 3);
    }

    #[test]
    fn swapping_keeps_identity_and_updates_navigation() {
        let mut tree = PaneTree::Split { axis: Axis::Columns, weights: vec![1., 1.],
            children: vec![PaneTree::Pane(10), PaneTree::Pane(20)] };
        assert_eq!(swap_target(&tree, &10, Some(Direction::Left), false), None);
        assert_eq!(swap_target(&tree, &10, Some(Direction::Right), true), None);
        assert_eq!(swap_target(&tree, &10, None, true), Some(20));
        let next = swap_target(&tree, &10, Some(Direction::Right), false).unwrap();
        assert!(tree.swap(&10, &next));
        assert_eq!(tree.pane_at(1), Some(&10));
        assert_eq!(cycle(&tree, Some(&10), 1), Some(20));
        assert_eq!(focus_after_close(&tree, &10, Some(&10)), Some(20));
        assert_eq!(swap_target(&PaneTree::Pane(10), &10, None, false), None);
    }

    #[test]
    fn insert_is_atomic_and_reports_visual_order() {
        let mut tree = None;
        assert_eq!(insert(&mut tree, Some(&9), 1, Axis::Columns, false), None);
        assert!(tree.is_none());
        assert_eq!(insert(&mut tree, None, 30, Axis::Columns, false), Some(0));
        assert_eq!(insert(&mut tree, Some(&30), 10, Axis::Rows, true), Some(0));
        assert_eq!(insert(&mut tree, Some(&30), 20, Axis::Columns, false), Some(2));
        assert_eq!(insert(&mut tree, Some(&99), 40, Axis::Columns, false), None);
        assert_eq!(insert(&mut tree, Some(&30), 10, Axis::Rows, false), None);
        assert_eq!(insert(&mut tree, None, 40, Axis::Rows, false), None);
        let tree = tree.unwrap();
        assert_eq!(tree.pane_count(), 3);
        assert_eq!((tree.pane_at(0), tree.pane_at(1), tree.pane_at(2)), (Some(&10), Some(&30), Some(&20)));
    }

    #[test]
    fn rejected_deep_insert_preserves_the_layout() {
        let mut tree = Some(PaneTree::Pane(0));
        for pane in 1..=crate::layout::MAX_SPLIT_DEPTH {
            assert!(insert(&mut tree, Some(&0), pane, Axis::Columns, false).is_some());
        }
        let count = tree.as_ref().unwrap().pane_count();
        assert_eq!(insert(&mut tree, Some(&0), count, Axis::Rows, false), None);
        assert_eq!(tree.as_ref().unwrap().pane_count(), count);
        assert!(tree.as_ref().unwrap().pane_index(&count).is_none());
    }

    #[test]
    fn nested_layout_controls_cycle_and_close_focus() {
        let tree = PaneTree::Split { axis: Axis::Columns, weights: vec![1., 1.], children: vec![
            PaneTree::Pane(30), PaneTree::Split { axis: Axis::Rows, weights: vec![1., 1.],
                children: vec![PaneTree::Pane(10), PaneTree::Pane(20)] },
        ] };
        assert_eq!(tree.pane_count(), 3);
        assert_eq!(cycle(&tree, Some(&30), 1), Some(10));
        assert_eq!(cycle(&tree, Some(&30), -1), Some(20));
        assert_eq!(cycle(&tree, Some(&20), 1), Some(30));
        assert_eq!(focus_after_close(&tree, &10, Some(&10)), Some(20));
        assert_eq!(focus_after_close(&tree, &20, Some(&20)), Some(10));
        assert_eq!(focus_after_close(&tree, &10, Some(&30)), Some(30));
        assert_eq!(focus_after_close(&tree, &10, None), Some(20));
        assert_eq!(focus_after_close(&PaneTree::Pane(10), &10, Some(&10)), None);
        assert_eq!(tree.pane_at(3), None);
    }

    #[test]
    fn zoom_requires_multiple_panes_and_focus() {
        assert_eq!(toggle_zoom(None, Some(1), 1), None);
        assert_eq!(toggle_zoom::<usize>(None, None, 2), None);
        assert_eq!(toggle_zoom(None, Some(2), 2), Some(2));
        assert_eq!(toggle_zoom(Some(2), Some(1), 2), None);
    }

    #[test]
    fn zoom_blocks_resize_and_equalize_reveals_layout() {
        let mut tree = Some(PaneTree::Split {
            axis: Axis::Columns,
            children: vec![PaneTree::Pane(1), PaneTree::Pane(2)],
            weights: vec![1., 1.],
        });
        let mut zoomed = Some(1);
        assert!(!resize(&mut tree, &1, Direction::Right, true));
        if let Some(PaneTree::Split { weights, .. }) = &tree {
            assert_eq!(weights, &[1., 1.]);
        }
        equalize(&mut tree, &mut zoomed);
        assert_eq!(zoomed, None);
        assert!(resize(&mut tree, &1, Direction::Right, false));
        zoomed = Some(2);
        equalize(&mut tree, &mut zoomed);
        assert_eq!(zoomed, None);
        if let Some(PaneTree::Split { weights, .. }) = &tree {
            assert_eq!(weights, &[1., 1.]);
        }
    }
}
