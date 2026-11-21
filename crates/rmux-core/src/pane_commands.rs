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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::Axis;

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
