//! Pane command transitions shared by desktop and browser application hosts.
use crate::layout::{Direction, PaneTree};

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
