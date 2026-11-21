//! Window-local group ownership, independent of terminal/process lifetime.
//! The first member is the anchor; removing it promotes the next member.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Placement {
    #[default]
    AfterCurrent,
    Top,
    End,
}

/// Position of a new ungrouped row; callers pass a group anchor for grouped origins.
pub fn insertion_index(len: usize, current: usize, placement: Placement) -> usize {
    match placement {
        Placement::Top => 0,
        Placement::End => len,
        Placement::AfterCurrent => current.saturating_add(1).min(len),
    }
}

/// Pick the next logical row, falling back to the previous row at the end.
/// Storage indices need not match the pinned/grouped navigation order.
pub fn neighbor_after_removal<T: Copy + Eq>(order: &[T], removed: T) -> Option<T> {
    let index = order.iter().position(|item| *item == removed)?;
    order.get(index + 1).or_else(|| index.checked_sub(1).and_then(|index| order.get(index))).copied()
}

/// Keep a group's row in place when its anchor changes. Live old anchors trade
/// storage slots; a closed anchor's replacement takes the vacated position.
pub fn preserve_anchor_slot<T, W: Copy + Eq>(items: &mut Vec<T>, key: impl Fn(&T) -> W, previous: W, previous_index: usize, anchor: W) {
    let Some(current) = items.iter().position(|item| key(item) == anchor) else { return; };
    if let Some(old) = items.iter().position(|item| key(item) == previous) {
        items.swap(old, current);
    } else {
        let destination = previous_index.saturating_sub(usize::from(current < previous_index));
        let item = items.remove(current);
        items.insert(destination.min(items.len()), item);
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Group<W> {
    pub id: u64,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<std::path::PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub members: Vec<W>,
    pub collapsed: bool,
    pub pinned: bool,
    /// Explicit provenance, never inferred from whether a shell looks idle.
    pub generated_anchor: bool,
}

impl<W> Group<W> {
    pub fn removable_generated_anchor(&self) -> Option<&W> {
        (self.generated_anchor && self.members.len() == 1).then(|| &self.members[0])
    }

    pub fn map_members<T>(&self, mut map: impl FnMut(&W) -> Result<T>) -> Result<Group<T>> {
        Ok(Group {
            id: self.id, name: self.name.clone(),
            external_id: self.external_id.clone(), cwd: self.cwd.clone(),
            color: self.color, icon: self.icon.clone(),
            members: self.members.iter().map(&mut map).collect::<Result<_>>()?,
            collapsed: self.collapsed, pinned: self.pinned,
            generated_anchor: self.generated_anchor,
        })
    }
}

#[derive(Debug)]
pub struct Groups<W> {
    entries: Vec<Group<W>>,
    next_id: u64,
}

impl<W> Default for Groups<W> {
    fn default() -> Self { Self { entries: Vec::new(), next_id: 1 } }
}

impl<W: Copy + Eq> Groups<W> {
    /// Storage position for a new top-level workspace. Group children use their
    /// anchor as the origin; new unpinned rows cannot enter the pinned tier.
    pub fn new_workspace_index(&self, workspaces: &[(W, bool)], current: usize, placement: Placement) -> usize {
        let origin = workspaces.get(current);
        let group = origin.and_then(|(id, _)| self.containing(*id));
        let anchor = group.and_then(|group| group.members.first())
            .and_then(|anchor| workspaces.iter().position(|(id, _)| id == anchor))
            .unwrap_or(current);
        let pinned = group.map(|group| group.pinned)
            .unwrap_or_else(|| origin.is_some_and(|(_, pinned)| *pinned));
        let placement = if placement == Placement::AfterCurrent && pinned { Placement::Top } else { placement };
        insertion_index(workspaces.len(), anchor, placement)
    }

    /// Pinned top-level rows first; member pins never split a group. Navigation
    /// can include collapsed children so hiding rows does not renumber shortcuts.
    pub fn ordered_indices(&self, workspaces: &[(W, bool)], include_collapsed: bool) -> Vec<usize> {
        let mut order = Vec::with_capacity(workspaces.len());
        for pinned in [true, false] {
            for (index, (id, workspace_pin)) in workspaces.iter().enumerate() {
                if let Some(group) = self.containing(*id) {
                    if group.members.first() != Some(id) || group.pinned != pinned { continue; }
                    order.push(index);
                    if include_collapsed || !group.collapsed {
                        for member in group.members.iter().skip(1) {
                            if let Some(index) = workspaces.iter().position(|(id, _)| id == member) { order.push(index); }
                        }
                    }
                } else if *workspace_pin == pinned { order.push(index); }
            }
        }
        order
    }

    pub fn from_entries(entries: Vec<Group<W>>, exists: impl Fn(W) -> bool) -> Result<Self> {
        ensure!(entries.len() <= 64, "too many workspace groups");
        let mut members = Vec::new();
        let mut ids = Vec::new();
        let mut external_ids = Vec::new();
        let mut next_id = 1;
        for group in &entries {
            if let Some(id) = &group.external_id {
                ensure!(!id.trim().is_empty() && id.len() <= 1024 && !id.chars().any(char::is_control) && !external_ids.contains(id), "invalid or duplicate external group identity");
                external_ids.push(id.clone());
            }
            ensure!(group.cwd.as_ref().is_none_or(|path| path.is_absolute() && path.as_os_str().len() <= 8192), "invalid group directory");
            ensure!(group.color.is_none_or(|color| color <= 0xffffff), "invalid group color");
            ensure!(group.icon.as_deref().is_none_or(valid_workspace_icon), "invalid group icon");
            ensure!(group.id > 0 && group.id < u64::MAX && !ids.contains(&group.id), "invalid or duplicate group identity");
            ensure!(!group.name.trim().is_empty() && group.name.len() <= 1024 && !group.name.chars().any(char::is_control), "invalid group name");
            ensure!(!group.members.is_empty() && group.members.len() <= 64, "invalid group membership");
            for member in &group.members {
                ensure!(exists(*member) && !members.contains(member), "missing or duplicate workspace in group");
                members.push(*member);
            }
            ids.push(group.id);
            next_id = next_id.max(group.id + 1);
        }
        Ok(Self { entries, next_id })
    }

    pub fn entries(&self) -> &[Group<W>] { &self.entries }

    pub fn containing(&self, workspace: W) -> Option<&Group<W>> {
        self.entries.iter().find(|group| group.members.contains(&workspace))
    }

    /// A collapsed group represents its selected children through its anchor.
    pub fn visible_workspace(&self, workspace: W) -> W {
        self.containing(workspace).filter(|group| group.collapsed)
            .and_then(|group| group.members.first()).copied().unwrap_or(workspace)
    }

    /// Selecting an anchor preserves collapse; selecting a child reveals it.
    pub fn reveal(&mut self, workspace: W) {
        if let Some(group) = self.entries.iter_mut().find(|group| group.members.contains(&workspace)) {
            if group.members.first() != Some(&workspace) { group.collapsed = false; }
        }
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut Group<W>> {
        self.entries.iter_mut().find(|group| group.id == id)
    }

    /// Validate everything before moving members out of existing groups.
    pub fn create(&mut self, name: String, members: Vec<W>, generated_anchor: bool) -> Result<u64> {
        ensure!(!name.trim().is_empty() && name.len() <= 1024 && !name.chars().any(char::is_control), "invalid group name");
        ensure!(!members.is_empty(), "a group needs an anchor");
        ensure!(members.iter().enumerate().all(|(index, member)| !members[..index].contains(member)), "duplicate group member");
        ensure!(!generated_anchor || members.len() == 1, "generated groups start with one anchor");
        let id = self.next_id;
        let next_id = id.checked_add(1).ok_or_else(|| anyhow::anyhow!("group identity exhausted"))?;
        for member in &members { self.remove(*member); }
        self.next_id = next_id;
        self.entries.push(Group { id, name, external_id: None, cwd: None, color: None, icon: None, members, collapsed: false, pinned: false, generated_anchor });
        Ok(id)
    }

    /// Dissolve only the container. The caller continues to own all workspaces.
    pub fn ungroup(&mut self, id: u64) -> Option<Group<W>> {
        let index = self.entries.iter().position(|group| group.id == id)?;
        Some(self.entries.remove(index))
    }

    /// Used by closing, detaching, and explicit ungrouping of one workspace.
    pub fn remove(&mut self, workspace: W) {
        let Some(index) = self.entries.iter().position(|group| group.members.contains(&workspace)) else { return; };
        let group = &mut self.entries[index];
        if group.members.first() == Some(&workspace) { group.generated_anchor = false; }
        group.members.retain(|member| *member != workspace);
        if group.members.is_empty() { self.entries.remove(index); }
    }

    /// Move a workspace into a group, after a member or at the end.
    /// A same-group move preserves the anchor and existing order.
    pub fn add(&mut self, id: u64, workspace: W, after: Option<W>) -> Result<()> {
        let target = self.entries.iter().find(|group| group.id == id)
            .ok_or_else(|| anyhow::anyhow!("group no longer exists"))?;
        if target.members.contains(&workspace) { return Ok(()); }
        let position = match after {
            Some(member) => target.members.iter().position(|candidate| *candidate == member)
                .ok_or_else(|| anyhow::anyhow!("placement member is outside the group"))? + 1,
            None => target.members.len(),
        };
        self.remove(workspace);
        // Removing a source group may shift the destination's vector index.
        self.get_mut(id).expect("destination retains its anchor").members.insert(position, workspace);
        Ok(())
    }

    pub fn add_new(&mut self, id: u64, workspace: W, reference: Option<W>, placement: Placement) -> Result<()> {
        let group = self.entries.iter().find(|group| group.id == id)
            .ok_or_else(|| anyhow::anyhow!("group no longer exists"))?;
        let after = match placement {
            Placement::AfterCurrent => reference.filter(|member| group.members.contains(member)).or_else(|| group.members.first().copied()),
            Placement::Top => group.members.first().copied(),
            Placement::End => None,
        };
        self.add(id, workspace, after)
    }

    pub fn set_anchor(&mut self, id: u64, workspace: W) -> Result<()> {
        let group = self.get_mut(id).ok_or_else(|| anyhow::anyhow!("group no longer exists"))?;
        let index = group.members.iter().position(|member| *member == workspace)
            .ok_or_else(|| anyhow::anyhow!("anchor must belong to the group"))?;
        if index != 0 {
            group.members.remove(index);
            group.members.insert(0, workspace);
            group.generated_anchor = false;
        }
        Ok(())
    }

    /// Reorder children without replacing the anchor or its provenance.
    pub fn reorder_member(&mut self, workspace: W, delta: isize) -> bool {
        let Some(group) = self.entries.iter_mut().find(|group| group.members.contains(&workspace)) else { return false; };
        let index = group.members.iter().position(|member| *member == workspace).unwrap();
        if index == 0 { return false; }
        let next = index.saturating_add_signed(delta).clamp(1, group.members.len() - 1);
        if next == index { return false; }
        let member = group.members.remove(index);
        group.members.insert(next, member);
        true
    }

    /// Drop a child after another member, keeping the anchor fixed.
    pub fn place_member_after(&mut self, workspace: W, target: W) -> Result<()> {
        if workspace == target { return Ok(()); }
        let id = self.containing(target).ok_or_else(|| anyhow::anyhow!("target is not grouped"))?.id;
        if let Some(source) = self.containing(workspace) {
            ensure!(source.members.first() != Some(&workspace), "move group anchors with group ordering or choose a new anchor first");
        }
        if self.containing(workspace).is_some_and(|group| group.id == id) {
            let group = self.get_mut(id).unwrap();
            group.members.retain(|member| *member != workspace);
            let index = group.members.iter().position(|member| *member == target).unwrap() + 1;
            group.members.insert(index, workspace);
        } else {
            self.add(id, workspace, Some(target))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn new_workspaces_follow_group_anchors_and_respect_pinned_tiers() {
        let mut groups = super::Groups::default();
        let id = groups.create("Project".into(), vec![2u64, 4], false).unwrap();
        let entries = [(1, false), (2, false), (3, false), (4, false)];
        assert_eq!(groups.new_workspace_index(&entries, 3, Placement::AfterCurrent), 2);
        assert_eq!(groups.new_workspace_index(&entries, 0, Placement::AfterCurrent), 1);
        groups.get_mut(id).unwrap().pinned = true;
        assert_eq!(groups.new_workspace_index(&entries, 3, Placement::AfterCurrent), 0);
        assert_eq!(groups.new_workspace_index(&entries, 3, Placement::End), 4);
        assert_eq!(groups.new_workspace_index(&[(1, true)], 0, Placement::AfterCurrent), 0);
        assert_eq!(groups.new_workspace_index(&[], 0, Placement::AfterCurrent), 0);
    }
    #[test]
    fn navigation_reveals_children_but_preserves_collapsed_anchors() {
        let mut groups = super::Groups::default();
        let id = groups.create("Project".into(), vec![1u64, 2, 3], false).unwrap();
        groups.get_mut(id).unwrap().collapsed = true;
        assert_eq!(groups.visible_workspace(2), 1);
        assert_eq!(groups.visible_workspace(9), 9);
        groups.reveal(1);
        assert!(groups.get_mut(id).unwrap().collapsed);
        groups.reveal(3);
        assert!(!groups.get_mut(id).unwrap().collapsed);
        assert_eq!(groups.visible_workspace(2), 2);
        groups.reveal(9);
        assert_eq!(groups.entries().len(), 1);
    }
    #[test]
    fn close_neighbor_follows_logical_order_without_wrapping() {
        let order = [4, 1, 7, 2];
        assert_eq!(neighbor_after_removal(&order, 4), Some(1));
        assert_eq!(neighbor_after_removal(&order, 1), Some(7));
        assert_eq!(neighbor_after_removal(&order, 2), Some(7));
        assert_eq!(neighbor_after_removal(&order, 99), None);
        assert_eq!(neighbor_after_removal(&[1], 1), None);
        assert_eq!(neighbor_after_removal(&[], 1), None);
    }

    #[test]
    fn directory_override_survives_member_mapping_and_restore() {
        let mut groups = super::Groups::default();
        let id = groups.create("Project".into(), vec![1u64], false).unwrap();
        let directory = std::env::temp_dir().join("rmux-group-project");
        groups.get_mut(id).unwrap().cwd = Some(directory.clone());
        let mapped = groups.entries()[0].map_members(|id| Ok(id + 10)).unwrap();
        let bytes = serde_json::to_vec(&mapped).unwrap();
        let restored: super::Group<u64> = serde_json::from_slice(&bytes).unwrap();
        let mut groups = super::Groups::from_entries(vec![restored], |id| id == 11).unwrap();
        assert_eq!(groups.entries()[0].cwd.as_ref(), Some(&directory));
        groups.get_mut(id).unwrap().cwd = Some("relative".into());
        assert!(super::Groups::from_entries(groups.entries().to_vec(), |_| true).is_err());
    }

    use super::*;

    #[test]
    fn cleanup_requires_explicit_provenance_and_no_members() -> Result<()> {
        let mut groups = Groups::default();
        let id = groups.create("Generated".into(), vec![1], true)?;
        assert_eq!(groups.get_mut(id).unwrap().removable_generated_anchor(), Some(&1));
        groups.add(id, 2, None)?;
        assert!(groups.get_mut(id).unwrap().removable_generated_anchor().is_none());
        groups.remove(1);
        assert!(groups.get_mut(id).unwrap().removable_generated_anchor().is_none());
        let user = groups.create("User".into(), vec![3], false)?;
        assert!(groups.get_mut(user).unwrap().removable_generated_anchor().is_none());
        Ok(())
    }

    #[test]
    fn new_member_placement_keeps_anchor_and_falls_back_to_top() -> Result<()> {
        let mut groups = Groups::default();
        let id = groups.create("Build".into(), vec![1, 2, 3], false)?;
        groups.add_new(id, 4, Some(2), Placement::AfterCurrent)?;
        assert_eq!(groups.containing(1).unwrap().members, vec![1, 2, 4, 3]);
        groups.add_new(id, 5, Some(3), Placement::Top)?;
        assert_eq!(groups.containing(1).unwrap().members, vec![1, 5, 2, 4, 3]);
        groups.add_new(id, 6, Some(1), Placement::End)?;
        assert_eq!(groups.containing(1).unwrap().members, vec![1, 5, 2, 4, 3, 6]);
        groups.add_new(id, 7, Some(99), Placement::AfterCurrent)?;
        assert_eq!(groups.containing(1).unwrap().members, vec![1, 7, 5, 2, 4, 3, 6]);
        Ok(())
    }

    #[test]
    fn navigation_keeps_hidden_members_and_respects_group_pin_tiers() -> Result<()> {
        let mut groups = Groups::default();
        let id = groups.create("Build".into(), vec![3, 1], false)?;
        let workspaces = [(1, true), (2, true), (3, false), (4, false)];
        assert_eq!(groups.ordered_indices(&workspaces, true), vec![1, 2, 0, 3]);
        groups.get_mut(id).unwrap().collapsed = true;
        assert_eq!(groups.ordered_indices(&workspaces, false), vec![1, 2, 3]);
        assert_eq!(groups.ordered_indices(&workspaces, true), vec![1, 2, 0, 3]);
        groups.get_mut(id).unwrap().pinned = true;
        let reordered = [(3, false), (1, true), (2, true), (4, false)];
        assert_eq!(groups.ordered_indices(&reordered, true), vec![0, 1, 2, 3]);
        assert_eq!(groups.ordered_indices(&reordered, false), vec![0, 2, 3]);
        Ok(())
    }

    #[test]
    fn anchor_slot_survives_close_and_explicit_replacement() {
        let mut items = vec![9, 1, 8, 2, 7];
        preserve_anchor_slot(&mut items, |id| *id, 1, 1, 2);
        assert_eq!(items, vec![9, 2, 8, 1, 7]);
        let mut items = vec![9, 8, 2, 7];
        preserve_anchor_slot(&mut items, |id| *id, 1, 1, 2);
        assert_eq!(items, vec![9, 2, 8, 7]);
        // Member storage may precede its header after earlier drag operations.
        let mut items = vec![2, 9, 8, 7];
        preserve_anchor_slot(&mut items, |id| *id, 1, 3, 2);
        assert_eq!(items, vec![9, 8, 2, 7]);
    }

    #[test]
    fn member_reordering_and_drops_preserve_anchor() -> Result<()> {
        let mut groups = Groups::default();
        let id = groups.create("Build".into(), vec![1, 2, 3, 4], false)?;
        assert!(!groups.reorder_member(1, 1));
        assert!(!groups.reorder_member(2, -1));
        assert!(groups.reorder_member(2, 1));
        assert_eq!(groups.containing(1).unwrap().members, vec![1, 3, 2, 4]);
        groups.place_member_after(4, 1)?;
        assert_eq!(groups.containing(1).unwrap().members, vec![1, 4, 3, 2]);
        groups.place_member_after(4, 2)?;
        assert_eq!(groups.containing(1).unwrap().members, vec![1, 3, 2, 4]);
        let other = groups.create("Other".into(), vec![5, 6], false)?;
        assert!(groups.place_member_after(5, 1).is_err());
        assert_eq!(groups.containing(5).unwrap().id, other);
        groups.place_member_after(6, 3)?;
        assert_eq!(groups.containing(6).unwrap().id, id);
        assert_eq!(groups.containing(1).unwrap().members, vec![1, 3, 6, 2, 4]);
        assert_eq!(groups.containing(5).unwrap().members, vec![5]);
        Ok(())
    }

    #[test]
    fn moving_last_source_member_keeps_destination_anchor_and_order() -> Result<()> {
        let mut groups = Groups::default();
        let source = groups.create("Source".into(), vec![1], true)?;
        let destination = groups.create("Destination".into(), vec![2], true)?;
        assert!(groups.add(999, 1, None).is_err());
        assert_eq!(groups.containing(1).unwrap().id, source);
        groups.add(destination, 1, Some(2))?;
        assert_eq!(groups.entries().len(), 1);
        let group = groups.containing(1).unwrap();
        assert_eq!(group.id, destination);
        assert_eq!(group.members, vec![2, 1]);
        assert!(group.generated_anchor);
        groups.add(destination, 1, None)?;
        assert_eq!(groups.containing(1).unwrap().members, vec![2, 1]);
        groups.set_anchor(destination, 1)?;
        assert_eq!(groups.containing(2).unwrap().members, vec![1, 2]);
        assert!(!groups.containing(1).unwrap().generated_anchor);
        Ok(())
    }

    #[test]
    fn restored_identity_sequence_does_not_reuse_live_ids() -> Result<()> {
        let mut original = Groups::default();
        original.create("First".into(), vec![1], false)?;
        let last = original.create("Second".into(), vec![2], false)?;
        let mut restored = Groups::from_entries(original.entries().to_vec(), |member| member <= 2)?;
        assert!(restored.create("Third".into(), vec![3], false)? > last);
        let mut invalid = original.entries().to_vec();
        invalid[1].id = invalid[0].id;
        assert!(Groups::from_entries(invalid, |_| true).is_err());
        let mut invalid = original.entries().to_vec();
        invalid[1].id = u64::MAX;
        assert!(Groups::from_entries(invalid, |_| true).is_err());
        Ok(())
    }

    #[test]
    fn old_groups_default_style_and_mapping_preserves_custom_style() -> Result<()> {
        let mut group: Group<usize> = serde_json::from_str(r#"{"id":1,"name":"Build","members":[0],"collapsed":false,"pinned":false,"generated_anchor":false}"#)?;
        assert!(group.color.is_none() && group.icon.is_none());
        group.color = Some(0xabcdef);
        group.icon = Some("📂".into());
        group.external_id = Some("project/build".into());
        let mapped = group.map_members(|index| Ok(index + 42))?;
        assert_eq!(mapped.members, vec![42]);
        assert_eq!(mapped.color, group.color);
        assert_eq!(mapped.icon, group.icon);
        assert_eq!(mapped.external_id, group.external_id);
        Ok(())
    }

    #[test]
    fn saved_external_identities_are_unique_and_bounded() -> Result<()> {
        let mut groups = Groups::default();
        let first = groups.create("One".into(), vec![1], false)?;
        let second = groups.create("Two".into(), vec![2], false)?;
        groups.get_mut(first).unwrap().external_id = Some("project".into());
        groups.get_mut(second).unwrap().external_id = Some("project".into());
        assert!(Groups::from_entries(groups.entries().to_vec(), |_| true).is_err());
        groups.get_mut(second).unwrap().external_id = Some(" ".into());
        assert!(Groups::from_entries(groups.entries().to_vec(), |_| true).is_err());
        groups.get_mut(second).unwrap().external_id = Some("other".into());
        assert!(Groups::from_entries(groups.entries().to_vec(), |_| true).is_ok());
        Ok(())
    }

    #[test]
    fn closing_anchor_promotes_member_without_discarding_group_state() -> Result<()> {
        let mut groups = Groups::default();
        let id = groups.create("Build".into(), vec![1], true)?;
        groups.add(id, 2, Some(1))?;
        groups.add(id, 3, None)?;
        let group = groups.get_mut(id).unwrap();
        group.collapsed = true;
        group.pinned = true;
        groups.remove(1);
        let group = groups.containing(2).unwrap();
        assert_eq!(group.members, vec![2, 3]);
        assert_eq!(group.id, id);
        assert!(group.collapsed && group.pinned && !group.generated_anchor);
        groups.remove(2);
        groups.remove(3);
        assert!(groups.entries().is_empty());
        Ok(())
    }

    #[test]
    fn regrouping_is_exclusive_and_invalid_moves_leave_ownership_intact() -> Result<()> {
        let mut groups = Groups::default();
        let first = groups.create("Same name".into(), vec![1, 2], false)?;
        let second = groups.create("Same name".into(), vec![3], false)?;
        assert_ne!(first, second);
        assert!(groups.add(second, 1, Some(99)).is_err());
        assert_eq!(groups.containing(1).unwrap().id, first);
        groups.add(second, 1, Some(3))?;
        assert_eq!(groups.containing(2).unwrap().members, vec![2]);
        groups.set_anchor(second, 1)?;
        assert_eq!(groups.containing(3).unwrap().members, vec![1, 3]);
        assert!(groups.create("bad".into(), vec![2, 2], false).is_err());
        assert_eq!(groups.containing(2).unwrap().id, first);
        let third = groups.create("Regrouped".into(), vec![2, 3], false)?;
        assert_eq!(groups.containing(1).unwrap().members, vec![1]);
        assert_eq!(groups.ungroup(third).unwrap().members, vec![2, 3]);
        assert!(groups.containing(2).is_none());
        Ok(())
    }
}

/// A workspace icon is one visible grapheme with bounded storage.
pub fn valid_workspace_icon(icon: &str) -> bool {
    use unicode_segmentation::UnicodeSegmentation;
    !icon.trim().is_empty() && icon.len() <= 64
        && !icon.chars().any(char::is_control)
        && icon.graphemes(true).count() == 1
}

/// Number nine selects the last logical workspace, including collapsed members.
pub fn numbered_target<T: Copy>(order: &[T], number: usize) -> Option<T> {
    match number {
        1..=8 => order.get(number - 1).copied(),
        9 => order.last().copied(),
        _ => None,
    }
}

/// Cycle through logical group/pin order, independently of storage positions.
pub fn cycle_target<T: Copy + Eq>(order: &[T], active: T, delta: isize) -> Option<T> {
    let current = order.iter().position(|item| *item == active).unwrap_or(0);
    crate::layout::cycle_index(current, order.len(), delta).map(|next| order[next])
}

#[cfg(test)]
mod navigation_tests {
    use super::*;
    #[test]
    fn navigation_uses_logical_order_and_last_shortcut() {
        let order = [4, 1, 7, 2];
        assert_eq!(numbered_target(&order, 1), Some(4));
        assert_eq!(numbered_target(&order, 9), Some(2));
        for number in [0, 5, 10] { assert_eq!(numbered_target(&order, number), None); }
        assert_eq!(numbered_target::<usize>(&[], 9), None);
        assert_eq!(cycle_target(&order, 4, -1), Some(2));
        assert_eq!(cycle_target(&order, 2, 1), Some(4));
        assert_eq!(cycle_target(&order, 99, 1), Some(1));
        assert_eq!(cycle_target(&[], 0, 1), None);
    }
}
