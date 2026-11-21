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

/// Cycle in visual layout order, independent of host entity storage order.
pub fn cycle<T: Clone + PartialEq>(tree: &PaneTree<T>, focused: Option<&T>, direction: isize) -> Option<T> {
    let current = focused.and_then(|pane| tree.pane_index(pane)).unwrap_or(0);
    tree.pane_at(crate::layout::cycle_index(current, tree.pane_count(), direction)?).cloned()
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
