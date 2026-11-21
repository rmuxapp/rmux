//! Workspace ownership transitions, shared by native and browser applications.
use crate::{focus_trail::PreviousSelection, workspace_groups::{self, Groups}};

/// Application workspace ownership shared by native and browser hosts.
/// Entries contain platform views; selection and group state belong to this model.
/// Fields remain accessible for restoration and metadata operations during migration.
pub struct State<T, W> {
    pub entries: Vec<T>,
    pub active: usize,
    pub groups: Groups<W>,
    pub previous: PreviousSelection<W>,
}
impl<T, W> Default for State<T, W> {
    fn default() -> Self {
        Self { entries: Vec::new(), active: 0, groups: Groups::default(), previous: PreviousSelection::default() }
    }
}
/// Resolve a destination without changing focus or selection.
#[derive(Clone, Copy)]
pub enum Navigation { Cycle(isize), Number(usize), Previous }

#[derive(Clone, Copy)]
pub enum GroupEdit { Remove, SetAnchor, TogglePin }

impl<T, W: Copy + Eq + std::hash::Hash> State<T, W> {
    pub fn order(&self, include_collapsed: bool, metadata: impl Fn(&T) -> (W, bool)) -> Vec<usize> {
        let identities: Vec<_> = self.entries.iter().map(metadata).collect();
        self.groups.ordered_indices(&identities, include_collapsed)
    }
    pub fn navigation_target(&self, request: Navigation, metadata: impl Fn(&T) -> (W, bool)) -> Option<usize> {
        match request {
            Navigation::Previous => {
                let previous = self.previous.target()?;
                self.entries.iter().position(|entry| metadata(entry).0 == previous)
            }
            Navigation::Cycle(delta) => workspace_groups::cycle_target(&self.order(true, metadata), self.active, delta),
            Navigation::Number(number) => workspace_groups::numbered_target(&self.order(true, metadata), number),
        }
    }
    pub fn sidebar_row(&self, index: usize, metadata: impl Fn(&T) -> (W, bool)) -> Option<usize> {
        let visible = self.groups.visible_workspace(metadata(self.entries.get(index)?).0);
        self.order(false, &metadata).iter().position(|index| metadata(&self.entries[*index]).0 == visible)
    }
    pub fn group_anchor_slot(&self, member: W, key: impl Fn(&T) -> W) -> Option<(u64, W, usize)> {
        let group = self.groups.containing(member)?;
        let anchor = *group.members.first()?;
        let index = self.entries.iter().position(|entry| key(entry) == anchor)?;
        Some((group.id, anchor, index))
    }
    pub fn preserve_group_slot(&mut self, slot: Option<(u64, W, usize)>, selected: Option<W>, key: impl Fn(&T) -> W) {
        if let Some((id, previous, position)) = slot {
            if let Some(anchor) = self.groups.entries().iter().find(|group| group.id == id)
                .and_then(|group| group.members.first()).copied() {
                workspace_groups::preserve_anchor_slot(&mut self.entries, &key, previous, position, anchor);
            }
        }
        if let Some(index) = selected.and_then(|selected| self.entries.iter().position(|entry| key(entry) == selected)) {
            self.active = index;
        }
    }
    pub fn move_to_group(&mut self, index: usize, destination: u64, key: impl Fn(&T) -> W) -> anyhow::Result<()> {
        let member = self.entries.get(index).map(&key).ok_or_else(|| anyhow::anyhow!("workspace no longer exists"))?;
        let selected = self.entries.get(self.active).map(&key);
        let slot = self.group_anchor_slot(member, &key);
        self.groups.add(destination, member, None)?;
        self.preserve_group_slot(slot, selected, key);
        self.groups.get_mut(destination).unwrap().collapsed = false;
        Ok(())
    }
    pub fn edit_group(&mut self, index: usize, edit: GroupEdit, key: impl Fn(&T) -> W) -> anyhow::Result<bool> {
        let Some(member) = self.entries.get(index).map(&key) else { return Ok(false); };
        let Some(group) = self.groups.containing(member) else { return Ok(false); };
        let id = group.id;
        let slot = self.group_anchor_slot(member, &key);
        let selected = self.entries.get(self.active).map(&key);
        match edit {
            GroupEdit::Remove => self.groups.remove(member),
            GroupEdit::SetAnchor => self.groups.set_anchor(id, member)?,
            GroupEdit::TogglePin => {
                let group = self.groups.get_mut(id).unwrap();
                group.pinned = !group.pinned;
            }
        }
        self.preserve_group_slot(slot, selected, key);
        Ok(true)
    }
    pub fn next_group_name(&self) -> String {
        (1..).map(|number| format!("Group {number}"))
            .find(|name| !self.groups.entries().iter().any(|group| &group.name == name)).unwrap()
    }
    /// Extract one workspace into a new group without moving the old group's
    /// promoted anchor in the sidebar or changing the selected workspace identity.
    pub fn create_group(&mut self, index: usize, generated: bool, key: impl Fn(&T) -> W) -> anyhow::Result<u64> {
        let member = self.entries.get(index).map(&key).ok_or_else(|| anyhow::anyhow!("workspace no longer exists"))?;
        let selected = self.entries.get(self.active).map(&key);
        let slot = self.group_anchor_slot(member, &key);
        let id = self.groups.create(self.next_group_name(), vec![member], generated)?;
        self.preserve_group_slot(slot, selected, key);
        Ok(id)
    }
    pub fn select(&mut self, index: usize, key: impl Fn(&T) -> W) -> bool {
        select(&self.entries, &mut self.active, &mut self.groups, &mut self.previous, index, key)
    }
    pub fn insert(&mut self, entry: T, request: Insertion<W>, metadata: impl Fn(&T) -> (W, bool)) -> anyhow::Result<usize> {
        insert(&mut self.entries, &mut self.active, &mut self.groups, entry, request, metadata)
    }
    pub fn close(&mut self, index: usize, metadata: impl Fn(&T) -> (W, bool)) -> Option<ClosedWorkspace<T>> {
        close(&mut self.entries, &mut self.active, &mut self.groups, &mut self.previous, index, metadata)
    }
    pub fn ungroup(&mut self, id: u64, metadata: impl Fn(&T) -> (W, bool)) -> bool {
        ungroup(&mut self.entries, &mut self.active, &mut self.groups, id, metadata)
    }
}

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

pub struct Insertion<W> {
    pub origin: Option<W>,
    pub inherit_group: bool,
    pub placement: workspace_groups::Placement,
}

/// Attach an already-created workspace without selecting it. Preserve the old
/// active identity until the host explicitly selects the new entry. Validate
/// identity/origin before mutating membership or entry ownership.
pub fn insert<T, W: Copy + Eq>(
    entries: &mut Vec<T>, active: &mut usize, groups: &mut Groups<W>,
    entry: T, request: Insertion<W>, metadata: impl Fn(&T) -> (W, bool),
) -> anyhow::Result<usize> {
    let member = metadata(&entry).0;
    let identities: Vec<_> = entries.iter().map(&metadata).collect();
    anyhow::ensure!(!identities.iter().any(|(id, _)| *id == member), "workspace already exists");
    let origin = match request.origin {
        Some(id) => identities.iter().position(|(candidate, _)| *candidate == id)
            .ok_or_else(|| anyhow::anyhow!("workspace origin no longer exists"))?,
        None => { anyhow::ensure!(entries.is_empty(), "workspace origin is required"); 0 }
    };
    let parent = request.origin.filter(|_| request.inherit_group)
        .and_then(|id| groups.containing(id)).map(|group| group.id);
    let index = if let Some(parent) = parent {
        groups.add_new(parent, member, request.origin, request.placement)?;
        groups.get_mut(parent).unwrap().collapsed = false;
        entries.len()
    } else {
        groups.new_workspace_index(&identities, origin, request.placement)
    };
    if !entries.is_empty() && index <= *active { *active += 1; }
    entries.insert(index, entry);
    Ok(index)
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
    fn moving_anchor_between_groups_preserves_selection_and_reveals_destination() {
        let mut state = State { entries: vec![1, 2, 3, 4], active: 2, ..Default::default() };
        let source = state.groups.create("source".into(), vec![1, 4], false).unwrap();
        let destination = state.groups.create("destination".into(), vec![2], false).unwrap();
        state.groups.get_mut(destination).unwrap().collapsed = true;
        assert!(state.move_to_group(0, 999, |id| *id).is_err());
        assert_eq!(state.entries, vec![1, 2, 3, 4]);
        assert_eq!(state.groups.containing(1).unwrap().id, source);
        assert!(state.groups.get_mut(destination).unwrap().collapsed);
        state.move_to_group(0, destination, |id| *id).unwrap();
        assert_eq!(state.entries, vec![4, 2, 3, 1]);
        assert_eq!(state.entries[state.active], 3);
        assert_eq!(state.groups.containing(4).unwrap().id, source);
        assert_eq!(state.groups.containing(1).unwrap().members, vec![2, 1]);
        assert!(!state.groups.containing(1).unwrap().collapsed);
        state.move_to_group(3, destination, |id| *id).unwrap();
        assert_eq!(state.entries, vec![4, 2, 3, 1]);
        assert_eq!(state.groups.containing(1).unwrap().members, vec![2, 1]);
    }

    #[test]
    fn group_edits_preserve_selection_and_promoted_anchor_slot() {
        let mut state = State { entries: vec![1, 2, 3, 4], active: 3, ..Default::default() };
        let group = state.groups.create("group".into(), vec![1, 4], false).unwrap();
        assert!(state.edit_group(3, GroupEdit::SetAnchor, |id| *id).unwrap());
        assert_eq!(state.entries, vec![4, 2, 3, 1]);
        assert_eq!(state.entries[state.active], 4);
        assert_eq!(state.groups.containing(4).unwrap().members, vec![4, 1]);
        assert!(state.edit_group(0, GroupEdit::TogglePin, |id| *id).unwrap());
        assert!(state.groups.containing(4).unwrap().pinned);
        assert!(state.edit_group(0, GroupEdit::Remove, |id| *id).unwrap());
        assert_eq!(state.entries, vec![1, 2, 3, 4]);
        assert_eq!(state.entries[state.active], 4);
        assert!(state.groups.containing(4).is_none());
        assert_eq!(state.groups.containing(1).unwrap().id, group);
        assert!(state.groups.containing(1).unwrap().pinned);
        assert!(!state.edit_group(3, GroupEdit::Remove, |id| *id).unwrap());
        assert!(!state.edit_group(99, GroupEdit::SetAnchor, |id| *id).unwrap());
    }

    #[test]
    fn extracting_group_anchor_preserves_old_slot_selection_and_unique_names() {
        let mut state = State { entries: vec![1, 2, 3, 4], active: 0, ..Default::default() };
        let old = state.groups.create("Group 1".into(), vec![1, 4], false).unwrap();
        let new = state.create_group(0, false, |id| *id).unwrap();
        assert_eq!(state.entries, vec![4, 2, 3, 1]);
        assert_eq!(state.entries[state.active], 1);
        assert_eq!(state.groups.containing(4).unwrap().id, old);
        let group = state.groups.containing(1).unwrap();
        assert_eq!(group.id, new);
        assert_eq!(group.name, "Group 2");
        assert!(!group.generated_anchor);
        assert_eq!(state.next_group_name(), "Group 3");
        assert!(state.create_group(99, true, |id| *id).is_err());
        assert_eq!(state.next_group_name(), "Group 3");
        state.create_group(1, true, |id| *id).unwrap();
        assert!(state.groups.containing(2).unwrap().generated_anchor);
        assert_eq!(state.entries[state.active], 1);
    }

    #[test]
    fn state_navigation_tracks_group_order_visibility_and_closed_history() {
        let mut state = State { entries: vec![(1, false), (2, true), (3, false), (4, false)], ..Default::default() };
        let group = state.groups.create("group".into(), vec![1, 4], false).unwrap();
        state.groups.get_mut(group).unwrap().collapsed = true;
        assert_eq!(state.order(true, |entry| *entry), vec![1, 0, 3, 2]);
        assert_eq!(state.order(false, |entry| *entry), vec![1, 0, 2]);
        assert_eq!(state.sidebar_row(3, |entry| *entry), Some(1));
        assert_eq!(state.sidebar_row(99, |entry| *entry), None);
        assert_eq!(state.navigation_target(Navigation::Number(1), |entry| *entry), Some(1));
        assert_eq!(state.navigation_target(Navigation::Number(9), |entry| *entry), Some(2));
        assert_eq!(state.navigation_target(Navigation::Cycle(1), |entry| *entry), Some(3));
        assert_eq!(state.navigation_target(Navigation::Cycle(-1), |entry| *entry), Some(1));
        // Target lookup alone must not reveal a collapsed group or change MRU.
        assert!(state.groups.entries().iter().find(|entry| entry.id == group).unwrap().collapsed);
        assert!(state.select(3, |entry| entry.0));
        assert!(!state.groups.entries().iter().find(|entry| entry.id == group).unwrap().collapsed);
        assert_eq!(state.navigation_target(Navigation::Previous, |entry| *entry), Some(0));
        state.close(0, |entry| *entry).unwrap();
        assert_eq!(state.navigation_target(Navigation::Previous, |entry| *entry), None);
        assert_eq!(state.entries[state.active].0, 4);
        let empty: State<(i32, bool), i32> = State::default();
        assert_eq!(empty.navigation_target(Navigation::Cycle(1), |entry| *entry), None);
        assert_eq!(empty.navigation_target(Navigation::Number(9), |entry| *entry), None);
    }

    #[test]
    fn insertion_preserves_outgoing_identity_and_group_member_order() {
        use workspace_groups::Placement;
        let mut entries = vec![(1, false), (2, false)];
        let mut active = 1;
        let mut groups = Groups::default();
        let group = groups.create("group".into(), vec![1, 2], false).unwrap();
        groups.get_mut(group).unwrap().collapsed = true;
        let index = insert(&mut entries, &mut active, &mut groups, (3, false), Insertion {
            origin: Some(2), inherit_group: true, placement: Placement::AfterCurrent,
        }, |entry| *entry).unwrap();
        assert_eq!(index, 2);
        assert_eq!(entries[active].0, 2);
        assert_eq!(groups.entries()[0].members, vec![1, 2, 3]);
        assert!(!groups.entries()[0].collapsed);
        let index = insert(&mut entries, &mut active, &mut groups, (4, false), Insertion {
            origin: Some(2), inherit_group: false, placement: Placement::Top,
        }, |entry| *entry).unwrap();
        assert_eq!(index, 0);
        assert_eq!(entries[active].0, 2);
        let before = entries.clone();
        for (member, origin) in [(4, Some(2)), (5, Some(99)), (5, None)] {
            assert!(insert(&mut entries, &mut active, &mut groups, (member, false), Insertion {
                origin, inherit_group: true, placement: Placement::End,
            }, |entry| *entry).is_err());
            assert_eq!(entries, before);
            assert_eq!(entries[active].0, 2);
            assert_eq!(groups.entries()[0].members, vec![1, 2, 3]);
        }
        let mut empty = vec![];
        let mut first = 0;
        assert_eq!(insert(&mut empty, &mut first, &mut Groups::default(), (9, false), Insertion {
            origin: None, inherit_group: true, placement: Placement::AfterCurrent,
        }, |entry| *entry).unwrap(), 0);
        assert_eq!(empty[first].0, 9);
    }

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
