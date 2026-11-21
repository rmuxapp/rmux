//! Workspace ownership transitions, shared by native and browser applications.
use crate::{focus_trail::PreviousSelection, workspace_groups::{self, Groups}};

/// Commit selection without focusing a terminal. Hosts can restore a remembered
/// pane or transfer a tab afterward without briefly activating the wrong session.
/// Re-selecting preserves MRU history but still reveals a collapsed group.
pub fn select<T, W: Copy + Eq>(
    entries: &[T], active: &mut usize, groups: &mut Groups<W>,
    previous: &mut PreviousSelection<W>, index: usize, key: impl Fn(&T) -> W,
) -> bool {
    let Some(destination) = entries.get(index).map(&key) else { return false; };
    previous.record(entries.get(*active).map(&key), Some(destination));
    *active = index;
    groups.reveal(destination);
    true
}

pub struct ClosedWorkspace<T> {
    /// Keep ownership alive until the host finishes its subscriptions/UI cleanup.
    pub removed: T,
    pub was_active: bool,
}

/// Remove one workspace, promote a group anchor in its original sidebar slot,
/// and preserve selection by identity. Hosts confirm process termination before
/// calling this and decide whether an empty window closes or shows an empty view.
/// Invalid indices leave all state untouched. Empty collections use active = 0.
pub fn close<T, W: Copy + Eq>(
    entries: &mut Vec<T>,
    active: &mut usize,
    groups: &mut Groups<W>,
    previous: &mut PreviousSelection<W>,
    index: usize,
    metadata: impl Fn(&T) -> (W, bool),
) -> Option<ClosedWorkspace<T>> {
    let removed_id = metadata(entries.get(index)?).0;
    let selected = entries.get(*active).map(|entry| metadata(entry).0);
    let was_active = selected == Some(removed_id);
    let identities: Vec<_> = entries.iter().map(&metadata).collect();
    let successor = if was_active {
        let order = groups.ordered_indices(&identities, true);
        workspace_groups::neighbor_after_removal(&order, index).map(|next| identities[next].0)
    } else { None };
    let slot = groups.containing(removed_id).and_then(|group| {
        let anchor = *group.members.first()?;
        let index = identities.iter().position(|(id, _)| *id == anchor)?;
        Some((group.id, anchor, index))
    });
    let removed = entries.remove(index);
    groups.remove(removed_id);
    previous.remove(removed_id);
    let promoted = slot.and_then(|(id, _, _)| groups.entries().iter().find(|group| group.id == id))
        .and_then(|group| group.members.first()).copied();
    if let (Some((_, old_anchor, old_index)), Some(anchor)) = (slot, promoted) {
        workspace_groups::preserve_anchor_slot(entries, |entry| metadata(entry).0, old_anchor, old_index, anchor);
    }
    let destination = if was_active {
        slot.filter(|(_, anchor, _)| *anchor == removed_id).and(promoted).or(successor)
    } else { selected };
    *active = destination.and_then(|id| entries.iter().position(|entry| metadata(entry).0 == id))
        .unwrap_or_else(|| active.saturating_sub(usize::from(*active > index)).min(entries.len().saturating_sub(1)));
    Some(ClosedWorkspace { removed, was_active })
}

/// Materialize the complete logical order before dissolving a group, including
/// collapsed members. Keep workspace-owned pin flags and selected identity.
/// This moves existing entries; it never clones/restarts their sessions.
pub fn ungroup<T, W: Copy + Eq + std::hash::Hash>(
    entries: &mut [T], active: &mut usize, groups: &mut Groups<W>, id: u64,
    metadata: impl Fn(&T) -> (W, bool),
) -> bool {
    if !groups.entries().iter().any(|group| group.id == id) { return false; }
    let selected = entries.get(*active).map(|entry| metadata(entry).0);
    let identities: Vec<_> = entries.iter().map(&metadata).collect();
    let ranks: std::collections::HashMap<_, _> = groups.ordered_indices(&identities, true)
        .into_iter().enumerate().map(|(rank, index)| (identities[index].0, rank)).collect();
    groups.ungroup(id);
    entries.sort_by_key(|entry| ranks[&metadata(entry).0]);
    if let Some(index) = selected.and_then(|id| entries.iter().position(|entry| metadata(entry).0 == id)) {
        *active = index;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_reveals_groups_preserves_toggle_and_ignores_stale_indices() {
        let entries = [1, 2, 3];
        let mut active = 0;
        let mut groups = Groups::default();
        let group = groups.create("group".into(), vec![2, 3], false).unwrap();
        groups.get_mut(group).unwrap().collapsed = true;
        let mut previous = PreviousSelection::default();
        assert!(!select(&entries, &mut active, &mut groups, &mut previous, 99, |id| *id));
        assert_eq!(active, 0);
        assert_eq!(previous.target(), None);
        assert!(groups.entries()[0].collapsed);
        assert!(select(&entries, &mut active, &mut groups, &mut previous, 2, |id| *id));
        assert_eq!(previous.target(), Some(1));
        assert!(!groups.entries()[0].collapsed);
        groups.get_mut(group).unwrap().collapsed = true;
        assert!(select(&entries, &mut active, &mut groups, &mut previous, 2, |id| *id));
        assert_eq!(previous.target(), Some(1));
        assert!(!groups.entries()[0].collapsed);
        let back = entries.iter().position(|id| Some(*id) == previous.target()).unwrap();
        select(&entries, &mut active, &mut groups, &mut previous, back, |id| *id);
        assert_eq!(active, 0);
        assert_eq!(previous.target(), Some(3));
    }

    #[test]
    fn ungroup_preserves_hidden_member_order_selection_and_individual_pins() {
        // Member order intentionally differs from entity storage order.
        let mut entries = vec![(1, false), (2, false), (3, true), (4, false)];
        let mut active = 3;
        let mut groups = Groups::default();
        let id = groups.create("group".into(), vec![1, 4, 3], false).unwrap();
        groups.get_mut(id).unwrap().collapsed = true;
        assert!(ungroup(&mut entries, &mut active, &mut groups, id, |entry| *entry));
        assert_eq!(entries, vec![(1, false), (4, false), (3, true), (2, false)]);
        assert_eq!(entries[active].0, 4);
        assert!(groups.entries().is_empty());
        let visible: Vec<_> = groups.ordered_indices(&entries, false).into_iter().map(|index| entries[index].0).collect();
        assert_eq!(visible, vec![3, 1, 4, 2]); // Individual pin survives dissolution.
        let before = entries.clone();
        assert!(!ungroup(&mut entries, &mut active, &mut groups, id, |entry| *entry));
        assert_eq!(entries, before);
        assert_eq!(entries[active].0, 4);
    }

    #[test]
    fn closing_anchor_promotes_member_without_moving_group_or_losing_other_focus() {
        // Group member storage is deliberately separated by an unrelated row.
        for (selected, expected) in [(0, 3), (1, 2), (2, 3)] {
            let mut entries = vec![(1, false), (2, false), (3, false), (4, false)];
            let mut groups = Groups::default();
            groups.create("group".into(), vec![1, 3], false).unwrap();
            let mut previous = PreviousSelection::default();
            previous.record(Some(1), Some(2));
            let mut active = selected;
            let closed = close(&mut entries, &mut active, &mut groups, &mut previous, 0, |entry| *entry).unwrap();
            assert_eq!(closed.removed.0, 1);
            assert_eq!(closed.was_active, selected == 0);
            assert_eq!(entries.iter().map(|entry| entry.0).collect::<Vec<_>>(), vec![3, 2, 4]);
            assert_eq!(entries[active].0, expected);
            assert_eq!(groups.entries()[0].members, vec![3]);
            assert_eq!(previous.target(), None);
        }
    }

    #[test]
    fn close_uses_pinned_order_and_leaves_invalid_requests_untouched() {
        let mut entries = vec![(1, false), (2, true), (3, false)];
        let mut active = 1;
        let mut groups = Groups::default();
        let mut previous = PreviousSelection::default();
        previous.record(Some(3), Some(2));
        assert!(close(&mut entries, &mut active, &mut groups, &mut previous, 9, |entry| *entry).is_none());
        assert_eq!(active, 1);
        assert_eq!(previous.target(), Some(3));
        close(&mut entries, &mut active, &mut groups, &mut previous, 1, |entry| *entry).unwrap();
        assert_eq!(entries[active].0, 1); // Pinned row precedes storage row 0.
        assert_eq!(previous.target(), Some(3));
        close(&mut entries, &mut active, &mut groups, &mut previous, 0, |entry| *entry).unwrap();
        assert_eq!(entries[active].0, 3);
        close(&mut entries, &mut active, &mut groups, &mut previous, 0, |entry| *entry).unwrap();
        assert!(entries.is_empty());
        assert_eq!(active, 0);
        assert_eq!(previous.target(), None);
    }
}
