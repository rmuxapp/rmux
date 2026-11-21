/// Shared with saved-layout validation so live splits remain restorable.
pub const MAX_SPLIT_DEPTH: usize = 32;

/// Wrap focus through a stable pane order; empty collections have no target.
pub fn cycle_index(current: usize, len: usize, direction: isize) -> Option<usize> {
    if len == 0 { return None; }
    Some(((current % len) as i128 + direction as i128).rem_euclid(len as i128) as usize)
}

#[cfg(test)]
mod cycling_tests {
    #[test]
    fn focus_wraps_in_both_directions() {
        use super::cycle_index;
        assert_eq!(cycle_index(0, 0, 1), None);
        assert_eq!(cycle_index(0, 1, -1), Some(0));
        assert_eq!(cycle_index(0, 3, -1), Some(2));
        assert_eq!(cycle_index(2, 3, 1), Some(0));
        assert_eq!(cycle_index(1, 3, 1), Some(2));
        assert_eq!(cycle_index(usize::MAX, 3, isize::MAX), Some(((usize::MAX % 3) as i128 + isize::MAX as i128).rem_euclid(3) as usize));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Axis {
    Columns,
    Rows,
}

#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

/// Prefer a pane aligned with the current pane, then the closest facing edge
/// and perpendicular center. Input rectangles are [x, y, width, height].
pub fn focus_neighbor(rects: &[[f32; 4]], current: usize, direction: Direction) -> Option<usize> {
    let source = *rects.get(current)?;
    if source[2] <= 0. || source[3] <= 0. {
        return None;
    }
    let vertical = matches!(direction, Direction::Up | Direction::Down);
    let backwards = matches!(direction, Direction::Left | Direction::Up);
    let primary = usize::from(vertical);
    let secondary = 1 - primary;
    let center = |rect: [f32; 4], axis: usize| rect[axis] + rect[axis + 2] / 2.;
    rects
        .iter()
        .enumerate()
        .filter_map(|(index, &rect)| {
            if index == current || rect[2] <= 0. || rect[3] <= 0. {
                return None;
            }
            let delta = center(rect, primary) - center(source, primary);
            if (backwards && delta >= 0.) || (!backwards && delta <= 0.) {
                return None;
            }
            let overlaps = rect[secondary] < source[secondary] + source[secondary + 2]
                && source[secondary] < rect[secondary] + rect[secondary + 2];
            let gap = if backwards {
                source[primary] - rect[primary] - rect[primary + 2]
            } else {
                rect[primary] - source[primary] - source[primary + 2]
            };
            Some((
                index,
                [
                    if overlaps { 0. } else { 1. },
                    gap.max(0.),
                    (center(rect, secondary) - center(source, secondary)).abs(),
                    delta.abs(),
                ],
            ))
        })
        .min_by(|(a, left), (b, right)| {
            left.iter()
                .zip(right)
                .map(|(a, b)| a.total_cmp(b))
                .find(|order| !order.is_eq())
                .unwrap_or_else(|| a.cmp(b))
        })
        .map(|(index, _)| index)
}

/// Layout owns pane references, never terminal processes. Moving a leaf preserves
/// the same session; removing a branch collapses it around its surviving panes.
#[derive(Debug, PartialEq)]
pub enum PaneTree<T> {
    Pane(T),
    Split {
        axis: Axis,
        children: Vec<Self>,
        weights: Vec<f32>,
    },
}
impl<T: Clone + PartialEq> PaneTree<T> {
    /// Resolve pane regions from current weights, independent of rendering or
    /// terminal headers/overlays. Leaf order matches serialization and cycling.
    pub fn neighbor(&self, target: &T, direction: Direction) -> Option<T> {
        fn visit<'a, T>(tree: &'a PaneTree<T>, bounds: [f32; 4], out: &mut Vec<(&'a T, [f32; 4])>) {
            match tree {
                PaneTree::Pane(pane) => out.push((pane, bounds)),
                PaneTree::Split { axis, children, weights } => {
                    let total: f64 = weights.iter().map(|weight| f64::from(*weight)).sum();
                    if children.len() != weights.len() || !total.is_finite() || total <= 0. { return; }
                    let dimension = if *axis == Axis::Columns { 0 } else { 1 };
                    let mut offset = 0.;
                    for (child, weight) in children.iter().zip(weights) {
                        let share = f64::from(*weight) / total;
                        let mut region = bounds;
                        region[dimension] += (f64::from(bounds[dimension + 2]) * offset) as f32;
                        region[dimension + 2] = (f64::from(bounds[dimension + 2]) * share) as f32;
                        visit(child, region, out);
                        offset += share;
                    }
                }
            }
        }
        let mut panes = Vec::new();
        visit(self, [0., 0., 1., 1.], &mut panes);
        let current = panes.iter().position(|(pane, _)| *pane == target)?;
        let bounds: Vec<_> = panes.iter().map(|(_, bounds)| *bounds).collect();
        focus_neighbor(&bounds, current, direction).map(|index| panes[index].0.clone())
    }

    /// Joining two roots adds one split level without changing either subtree.
    pub fn can_merge(&self, other: &Self) -> bool {
        fn fits<T>(tree: &PaneTree<T>, depth: usize) -> bool {
            if depth > MAX_SPLIT_DEPTH { return false; }
            match tree {
                PaneTree::Pane(_) => true,
                PaneTree::Split { children, .. } => children.iter().all(|child| fits(child, depth + 1)),
            }
        }
        fits(self, 1) && fits(other, 1)
    }
    /// Position in the depth-first order used by layout serialization.
    pub fn pane_index(&self, target: &T) -> Option<usize> {
        fn visit<T: PartialEq>(tree: &PaneTree<T>, target: &T, index: &mut usize) -> Option<usize> {
            match tree {
                PaneTree::Pane(pane) => {
                    let current = *index;
                    *index += 1;
                    (pane == target).then_some(current)
                }
                PaneTree::Split { children, .. } => children.iter().find_map(|child| visit(child, target, index)),
            }
        }
        visit(self, target, &mut 0)
    }
    /// Move the nearest matching divider by 5% of its neighboring pair. Prefer
    /// the requested side; at an outer edge, move the opposite shared divider.
    pub fn resize_direction(&mut self, target: &T, direction: Direction) -> bool {
        let (axis, positive) = match direction {
            Direction::Left => (Axis::Columns, false), Direction::Right => (Axis::Columns, true),
            Direction::Up => (Axis::Rows, false), Direction::Down => (Axis::Rows, true),
        };
        self.resize_ancestor(target, axis, positive).unwrap_or(false)
    }
    fn resize_ancestor(&mut self, target: &T, requested: Axis, positive: bool) -> Option<bool> {
        let Self::Split { axis, children, weights } = self else { return None; };
        let index = children.iter().position(|child| child.pane_index(target).is_some())?;
        if let Some(changed) = children[index].resize_ancestor(target, requested, positive) { return Some(changed); }
        if *axis != requested || children.len() < 2 { return None; }
        let left = if positive { index.min(children.len() - 2) } else { index.saturating_sub(1) };
        let total = weights[left] + weights[left + 1];
        let pair = resized_pair(weights[left], weights[left + 1], total * if positive { 0.05 } else { -0.05 }, total * 0.1);
        if pair == (weights[left], weights[left + 1]) { return Some(false); }
        weights[left] = pair.0;
        weights[left + 1] = pair.1;
        Some(true)
    }

    /// Give siblings equal space at every split without changing topology.
    pub fn equalize(&mut self) -> bool {
        let mut changed = false;
        if let Self::Split {
            children, weights, ..
        } = self
        {
            changed = weights.len() != children.len() || weights.iter().any(|weight| *weight != 1.);
            if changed { weights.clear(); weights.resize(children.len(), 1.); }
            for child in children { changed |= child.equalize(); }
        }
        changed
    }
    /// Exchange leaf identities without changing split geometry. Missing targets
    /// leave the entire tree untouched.
    pub fn swap(&mut self, first: &T, second: &T) -> bool {
        fn contains<T: PartialEq>(tree: &PaneTree<T>, target: &T) -> bool {
            match tree {
                PaneTree::Pane(pane) => pane == target,
                PaneTree::Split { children, .. } => {
                    children.iter().any(|child| contains(child, target))
                }
            }
        }
        fn exchange<T: Clone + PartialEq>(tree: &mut PaneTree<T>, first: &T, second: &T) {
            match tree {
                PaneTree::Pane(pane) if pane == first => *pane = second.clone(),
                PaneTree::Pane(pane) if pane == second => *pane = first.clone(),
                PaneTree::Split { children, .. } => {
                    for child in children {
                        exchange(child, first, second);
                    }
                }
                _ => {}
            }
        }
        if first == second || !contains(self, first) || !contains(self, second) {
            return false;
        }
        exchange(self, first, second);
        true
    }

    pub fn can_split(&self, target: &T) -> bool {
        fn depth<T: PartialEq>(tree: &PaneTree<T>, target: &T, level: usize) -> Option<usize> {
            match tree {
                PaneTree::Pane(pane) => (pane == target).then_some(level),
                PaneTree::Split { children, .. } => children.iter().find_map(|child| depth(child, target, level + 1)),
            }
        }
        depth(self, target, 0).is_some_and(|depth| depth < MAX_SPLIT_DEPTH)
    }

    #[cfg(test)]
    pub fn split(&mut self, target: &T, new: T, axis: Axis) -> bool {
        self.split_positioned(target, new, axis, false)
    }

    pub fn split_positioned(&mut self, target: &T, new: T, axis: Axis, before: bool) -> bool {
        if !self.can_split(target) { return false; }
        self.split_unchecked(target, new, axis, before)
    }

    fn split_unchecked(&mut self, target: &T, new: T, axis: Axis, before: bool) -> bool {
        match self {
            Self::Pane(pane) if pane == target => {
                *self = Self::Split {
                    axis,
                    children: if before { vec![Self::Pane(new), Self::Pane(pane.clone())] }
                        else { vec![Self::Pane(pane.clone()), Self::Pane(new)] },
                    weights: vec![1., 1.],
                };
                true
            }
            Self::Split { children, .. } => children
                .iter_mut()
                .any(|child| child.split_unchecked(target, new.clone(), axis, before)),
            _ => false,
        }
    }

    pub fn remove(self, target: &T) -> Option<Self> {
        match self {
            Self::Pane(pane) => (pane != *target).then_some(Self::Pane(pane)),
            Self::Split {
                axis,
                children,
                weights,
            } => {
                let (mut children, weights): (Vec<_>, Vec<_>) = children
                    .into_iter()
                    .zip(weights)
                    .filter_map(|(child, weight)| child.remove(target).map(|child| (child, weight)))
                    .unzip();
                match children.len() {
                    0 => None,
                    1 => children.pop(),
                    _ => Some(Self::Split {
                        axis,
                        children,
                        weights,
                    }),
                }
            }
        }
    }

    pub fn at_mut(&mut self, path: &[usize]) -> Option<&mut Self> {
        let Some((&index, tail)) = path.split_first() else {
            return Some(self);
        };
        match self {
            Self::Split { children, .. } => children.get_mut(index)?.at_mut(tail),
            Self::Pane(_) => None,
        }
    }
}

/// Adjust only the two neighbors of a divider. Delta and minimum are in weight
/// units, converted by the UI from the split's content size at drag start.
pub fn resized_pair(left: f32, right: f32, delta: f32, minimum: f32) -> (f32, f32) {
    let total = left + right;
    // Preserve narrower existing shares instead of jumping against the gesture.
    // The minimum is in relative weight units, so no absolute floor is valid.
    let minimum = minimum.max(0.).min(total / 2.).min(left).min(right);
    let next = (left + delta).clamp(minimum, total - minimum);
    if next == left || !next.is_finite() || next <= 0. || total - next <= 0. {
        return (left, right);
    }
    (next, total - next)
}

#[cfg(test)]
mod tests {
    #[test]
    fn pointer_resize_preserves_narrow_origin_and_weight_scale() {
        assert_eq!(resized_pair(0.01, 0.99, 0., 0.1), (0.01, 0.99));
        assert_eq!(resized_pair(0.01, 0.99, -0.2, 0.1), (0.01, 0.99));
        assert_eq!(resized_pair(0.99, 0.01, 0.2, 0.1), (0.99, 0.01));
        let normal = resized_pair(0.01, 0.99, 0.03, 0.1);
        let scaled = resized_pair(0.00001, 0.00099, 0.00003, 0.0001);
        assert!((normal.0 - 0.04).abs() < 1e-6);
        assert!((scaled.0 * 1000. - normal.0).abs() < 1e-6);
        assert!((scaled.1 * 1000. - normal.1).abs() < 1e-6);
    }

    #[test]
    fn keyboard_resize_does_not_reverse_narrow_restored_dividers() {
        let mut tree = PaneTree::Split { axis: Axis::Columns, weights: vec![0.01, 0.99], children: vec![PaneTree::Pane(1), PaneTree::Pane(2)] };
        assert!(!tree.resize_direction(&1, Direction::Left));
        let mut tiny = PaneTree::Split { axis: Axis::Columns, weights: vec![0.00001, 0.00099], children: vec![PaneTree::Pane(1), PaneTree::Pane(2)] };
        assert!(!tiny.resize_direction(&1, Direction::Left));
        assert!(tiny.resize_direction(&1, Direction::Right));
        assert!(tree.resize_direction(&1, Direction::Right));
        let PaneTree::Split { weights, .. } = &tree else { panic!() };
        assert!((weights[0] - 0.06).abs() < 0.00001);
        assert!((weights[1] - 0.94).abs() < 0.00001);
        let mut nested = PaneTree::Split { axis: Axis::Columns, weights: vec![1., 1.], children: vec![tree, PaneTree::Pane(3)] };
        assert!(!nested.resize_direction(&1, Direction::Left));
        let PaneTree::Split { weights, .. } = &nested else { panic!() };
        assert_eq!(weights, &[1., 1.], "a clamped local divider must not resize its ancestor");
    }

    #[test]
    fn directional_resize_uses_nearest_axis_and_preserves_other_weights() {
        let mut tree = PaneTree::Split { axis: Axis::Columns, weights: vec![1., 1., 2.], children: vec![
            PaneTree::Split { axis: Axis::Rows, weights: vec![1., 1.], children: vec![PaneTree::Pane(1), PaneTree::Pane(2)] },
            PaneTree::Pane(3), PaneTree::Pane(4),
        ] };
        assert!(tree.resize_direction(&2, Direction::Down));
        assert!(tree.resize_direction(&2, Direction::Right));
        let PaneTree::Split { weights, children, .. } = &tree else { panic!() };
        assert_eq!(weights, &[1.1, 0.9, 2.]);
        let PaneTree::Split { weights, .. } = &children[0] else { panic!() };
        assert_eq!(weights, &[1.1, 0.9]);
        assert_eq!(tree.pane_index(&2), Some(1));
        assert!(!tree.resize_direction(&99, Direction::Left));
        for _ in 0..100 { tree.resize_direction(&1, Direction::Up); }
        assert!(!tree.resize_direction(&1, Direction::Up));
        let PaneTree::Split { weights, children, .. } = &tree else { panic!() };
        assert_eq!(weights, &[1.1, 0.9, 2.]);
        let PaneTree::Split { weights, .. } = &children[0] else { panic!() };
        assert_eq!(weights, &[0.2, 1.8]);
    }

    #[test]
    fn positioned_splits_preserve_nested_order_and_existing_weights() {
        for axis in [Axis::Rows, Axis::Columns] {
            for before in [false, true] {
                let mut tree = PaneTree::Split { axis: Axis::Columns,
                    children: vec![PaneTree::Pane(1), PaneTree::Pane(2)], weights: vec![3., 7.] };
                assert!(tree.split_positioned(&2, 3, axis, before));
                assert_eq!(tree.pane_index(&1), Some(0));
                assert_eq!(tree.pane_index(&3), Some(if before { 1 } else { 2 }));
                assert_eq!(tree.pane_index(&2), Some(if before { 2 } else { 1 }));
                let PaneTree::Split { weights, children, .. } = &tree else { panic!() };
                assert_eq!(weights, &[3., 7.]);
                let PaneTree::Split { axis: actual, weights, .. } = &children[1] else { panic!() };
                assert_eq!(*actual, axis);
                assert_eq!(weights, &[1., 1.]);
                let saved = format!("{tree:?}");
                assert!(!tree.split_positioned(&99, 4, axis, before));
                assert_eq!(format!("{tree:?}"), saved);
            }
        }
    }

    #[test]
    fn merge_depth_accounts_for_both_preserved_subtrees() {
        let leaf = PaneTree::Pane(0);
        let mut deep = PaneTree::Pane(1);
        assert!(leaf.can_merge(&deep));
        for index in 2..=MAX_SPLIT_DEPTH + 1 {
            deep = PaneTree::Split { axis: Axis::Rows, children: vec![deep, PaneTree::Pane(index)], weights: vec![1., 2.] };
        }
        assert!(!leaf.can_merge(&deep));
        assert!(!deep.can_merge(&leaf));
        let deep = deep.remove(&(MAX_SPLIT_DEPTH + 1)).unwrap();
        assert!(leaf.can_merge(&deep));
    }
    #[test]
    fn split_limit_tracks_target_depth_not_total_panes() {
        let mut tree = PaneTree::Pane(0);
        for next in 1..=MAX_SPLIT_DEPTH {
            assert!(tree.can_split(&(next - 1)));
            assert!(tree.split(&(next - 1), next, Axis::Columns));
        }
        assert!(!tree.can_split(&MAX_SPLIT_DEPTH));
        let before = format!("{tree:?}");
        assert!(!tree.split(&MAX_SPLIT_DEPTH, 100, Axis::Rows));
        assert_eq!(format!("{tree:?}"), before, "rejected split must leave all geometry unchanged");
        assert!(!tree.can_split(&999));
        // A shallow sibling still has room even when another branch is deep.
        assert!(tree.can_split(&0));
    }

    #[test]
    fn saved_pane_index_follows_nested_layout_after_swap_and_removal() {
        use super::{Axis, PaneTree};
        let mut tree = PaneTree::Split {
            axis: Axis::Columns,
            weights: vec![1., 2.],
            children: vec![PaneTree::Pane(10), PaneTree::Split {
                axis: Axis::Rows,
                weights: vec![1., 1.],
                children: vec![PaneTree::Pane(20), PaneTree::Pane(30)],
            }],
        };
        assert_eq!(tree.pane_index(&30), Some(2));
        assert!(tree.swap(&10, &30));
        assert_eq!(tree.pane_index(&30), Some(0));
        assert_eq!(tree.pane_index(&10), Some(2));
        let tree = tree.remove(&20).unwrap();
        assert_eq!(tree.pane_index(&10), Some(1));
        assert_eq!(tree.pane_index(&20), None);
        assert_eq!(PaneTree::Pane(10).pane_index(&10), Some(0));
    }
    #[test]
    fn equalize_preserves_nested_topology_and_leaf_order() {
        use super::{Axis, PaneTree};
        let mut tree = PaneTree::Split {
            axis: Axis::Columns,
            weights: vec![2., 7.],
            children: vec![
                PaneTree::Pane(1),
                PaneTree::Split {
                    axis: Axis::Rows,
                    weights: vec![9., 3.],
                    children: vec![PaneTree::Pane(2), PaneTree::Pane(3)],
                },
            ],
        };
        tree.equalize();
        assert_eq!(
            tree,
            PaneTree::Split {
                axis: Axis::Columns,
                weights: vec![1., 1.],
                children: vec![
                    PaneTree::Pane(1),
                    PaneTree::Split {
                        axis: Axis::Rows,
                        weights: vec![1., 1.],
                        children: vec![PaneTree::Pane(2), PaneTree::Pane(3)],
                    }
                ],
            }
        );
    }
    use super::*;
    #[test]
    fn tree_neighbors_follow_weights_and_swaps_without_rendered_bounds() {
        let mut tree = PaneTree::Split {
            axis: Axis::Columns, weights: vec![1., 1.],
            children: vec![PaneTree::Pane(1), PaneTree::Split {
                axis: Axis::Rows, weights: vec![8., 2.],
                children: vec![PaneTree::Pane(2), PaneTree::Pane(3)],
            }],
        };
        assert_eq!(tree.neighbor(&1, Direction::Right), Some(2));
        assert_eq!(tree.neighbor(&2, Direction::Down), Some(3));
        assert_eq!(tree.neighbor(&3, Direction::Left), Some(1));
        assert_eq!(tree.neighbor(&1, Direction::Left), None);
        assert_eq!(tree.neighbor(&99, Direction::Right), None);
        if let PaneTree::Split { children, .. } = &mut tree {
            if let PaneTree::Split { weights, .. } = &mut children[1] { *weights = vec![2., 8.]; }
        }
        assert_eq!(tree.neighbor(&1, Direction::Right), Some(3));
        assert!(tree.swap(&2, &3));
        assert_eq!(tree.neighbor(&1, Direction::Right), Some(2));
        if let PaneTree::Split { weights, .. } = &mut tree { *weights = vec![1e30, 1e30]; }
        assert_eq!(tree.neighbor(&1, Direction::Right), Some(2));
        let tree = tree.remove(&2).unwrap();
        assert_eq!(tree.neighbor(&1, Direction::Right), Some(3));
        assert_eq!(PaneTree::Pane(1).neighbor(&1, Direction::Down), None);
    }

    #[test]
    fn swap_preserves_nested_geometry_and_rejects_missing_targets() {
        let mut tree = PaneTree::Pane(1);
        tree.split(&1, 2, Axis::Columns);
        tree.split(&2, 3, Axis::Rows);
        if let PaneTree::Split { weights, .. } = &mut tree {
            *weights = vec![2., 5.];
        }
        assert!(tree.swap(&1, &3));
        assert_eq!(
            tree,
            PaneTree::Split {
                axis: Axis::Columns,
                weights: vec![2., 5.],
                children: vec![
                    PaneTree::Pane(3),
                    PaneTree::Split {
                        axis: Axis::Rows,
                        weights: vec![1., 1.],
                        children: vec![PaneTree::Pane(2), PaneTree::Pane(1)]
                    }
                ]
            }
        );
        assert!(!tree.swap(&3, &99));
        assert!(!tree.swap(&3, &3));
        assert!(tree.swap(&3, &1));
        let PaneTree::Split { children, .. } = tree else {
            panic!("lost split")
        };
        assert_eq!(children[0], PaneTree::Pane(1));
    }
    #[test]
    fn directional_focus_follows_resized_nested_geometry() {
        let panes = [
            [0., 0., 300., 600.],
            [305., 0., 500., 200.],
            [305., 205., 500., 395.],
        ];
        assert_eq!(focus_neighbor(&panes, 0, Direction::Right), Some(2));
        assert_eq!(focus_neighbor(&panes, 2, Direction::Up), Some(1));
        assert_eq!(focus_neighbor(&panes, 1, Direction::Down), Some(2));
        assert_eq!(focus_neighbor(&panes, 2, Direction::Left), Some(0));
        assert_eq!(focus_neighbor(&panes, 0, Direction::Left), None);
        assert_eq!(focus_neighbor(&panes, 1, Direction::Right), None);
        assert_eq!(focus_neighbor(&[[0.; 4]], 0, Direction::Right), None);
    }
    #[test]
    fn nested_splits_and_close_preserve_surviving_panes() {
        let mut tree = PaneTree::Pane(1);
        assert!(tree.split(&1, 2, Axis::Columns));
        assert!(tree.split(&1, 3, Axis::Rows));
        assert!(!tree.split(&9, 4, Axis::Columns));
        let tree = tree.remove(&1).unwrap();
        assert_eq!(
            tree,
            PaneTree::Split {
                axis: Axis::Columns,
                children: vec![PaneTree::Pane(3), PaneTree::Pane(2)],
                weights: vec![1., 1.]
            }
        );
        let tree = tree.remove(&3).unwrap();
        assert_eq!(tree, PaneTree::Pane(2));
        assert!(tree.remove(&2).is_none());
    }

    #[test]
    fn resize_clamps_and_close_preserves_sibling_proportions() {
        assert_eq!(resized_pair(1., 1., 0.5, 0.2), (1.5, 0.5));
        assert_eq!(resized_pair(1., 1., -9., 0.25), (0.25, 1.75));
        assert_eq!(resized_pair(1., 1., 9., 8.), (1., 1.));
        let tree = PaneTree::Split {
            axis: Axis::Columns,
            children: vec![PaneTree::Pane(1), PaneTree::Pane(2), PaneTree::Pane(3)],
            weights: vec![1., 2., 3.],
        }
        .remove(&2)
        .unwrap();
        assert_eq!(
            tree,
            PaneTree::Split {
                axis: Axis::Columns,
                children: vec![PaneTree::Pane(1), PaneTree::Pane(3)],
                weights: vec![1., 3.],
            }
        );
    }
}
