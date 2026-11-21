//! Browser service host; workspace presentation comes from native sources.
use gpui::{prelude::*, *};
use crate::{InputProbe, WorkspaceCommand, PaletteRequest, CommandId, sidebar_presentation, NewWorkspace, NewWorkspaceGroup, NextWorkspace, PreviousWorkspace, SelectWorkspaceNumber};
pub(crate) struct BrowserHost {
    workspaces: Vec<(String, Entity<InputProbe>)>,
    active: usize,
    empty_focus: FocusHandle,
    workspace_scroll: ScrollHandle,
    groups: rmux_core::workspace_groups::Groups<EntityId>,
}
impl BrowserHost {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let workspaces = ["rmux", "website"].into_iter().map(|name|
            (name.to_owned(), Self::new_workspace_view(window, cx))).collect();
        let mut host = Self { workspaces, active: 0, empty_focus: cx.focus_handle(), workspace_scroll: ScrollHandle::new(), groups: Default::default() };
        host.select(0,window,cx);
        host
    }
    fn new_workspace_view(window: &mut Window, cx: &mut Context<Self>) -> Entity<InputProbe> {
        let entity = cx.new(|cx| InputProbe::new(window, cx));
        cx.subscribe_in(&entity, window, |this, _, command: &WorkspaceCommand, window, cx| {
            match command.0 {
                CommandId::CloseWorkspace => this.close_workspace(window, cx),
                CommandId::NewWorkspace => this.add(window, cx),
                CommandId::NewWorkspaceGroup => this.new_group(window, cx),
                CommandId::NextWorkspace => this.cycle(1, window, cx),
                CommandId::PreviousWorkspace => this.cycle(-1, window, cx),
                CommandId::UngroupWorkspaces => this.ungroup(cx),
                _ => {}
            }
        }).detach();
        cx.subscribe_in(&entity, window, |this, source, request: &PaletteRequest, window, cx| {
            match request {
                PaletteRequest::Open => {
                    let workspaces = this.workspace_order(true).into_iter().enumerate().map(|(position, index)| {
                        let (name, entity) = &this.workspaces[index];
                        (entity.entity_id(), format!("Switch to Workspace {}: {name}", position + 1))
                    }).collect();
                    source.update(cx, |workspace, cx| workspace.set_workspace_results(workspaces, cx));
                }
                PaletteRequest::SelectWorkspace(id) => {
                    if let Some(index) = this.workspaces.iter().position(|(_, entity)| entity.entity_id() == *id) {
                        this.select(index, window, cx);
                    }
                }
            }
        }).detach();
        entity
    }
    fn close_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        use rmux_core::workspace_groups::{neighbor_after_removal, preserve_anchor_slot};
        let Some((_, current)) = self.workspaces.get(self.active) else { return; };
        let removed = current.entity_id();
        let order: Vec<_> = self.workspace_order(true).into_iter()
            .map(|index| self.workspaces[index].1.entity_id()).collect();
        let next = neighbor_after_removal(&order, removed);
        let anchor_group = self.groups.containing(removed)
            .filter(|group| group.members.first() == Some(&removed)).map(|group| group.id);
        let previous_index = self.active;
        self.groups.remove(removed);
        self.workspaces.remove(previous_index);
        if let Some(anchor) = anchor_group.and_then(|id| self.groups.entries().iter().find(|group| group.id == id))
            .and_then(|group| group.members.first()).copied() {
            preserve_anchor_slot(&mut self.workspaces, |(_, entity)| entity.entity_id(), removed, previous_index, anchor);
        }
        self.active = next.and_then(|id| self.workspaces.iter().position(|(_, entity)| entity.entity_id() == id)).unwrap_or(0);
        if self.workspaces.is_empty() { self.empty_focus.focus(window, cx); cx.notify(); }
        else { self.select(self.active, window, cx); }
    }
    fn ungroup(&mut self, cx: &mut Context<Self>) {
        let selected = self.workspaces[self.active].1.entity_id();
        let Some(group) = self.groups.containing(selected).map(|group| group.id) else { return; };
        // Materialize logical order before removing the group, as native does.
        let ranks: std::collections::HashMap<_, _> = self.workspace_order(true).into_iter().enumerate()
            .map(|(rank, index)| (self.workspaces[index].1.entity_id(), rank)).collect();
        self.groups.ungroup(group);
        self.workspaces.sort_by_key(|(_, entity)| ranks[&entity.entity_id()]);
        self.active = self.workspaces.iter().position(|(_, entity)| entity.entity_id() == selected).unwrap();
        cx.notify();
    }
    fn select(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.workspaces.len() { return; }
        self.workspaces[self.active].1.update(cx, |workspace,cx| {
            workspace.split_drag = None;
            for pane in workspace.samples.values() {
                pane.update(cx, |pane,cx| pane.release_render_caches(cx));
            }
        });
        self.active=index;
        let member = self.workspaces[index].1.entity_id();
        self.groups.reveal(member);
        self.scroll_to_workspace();
        self.workspaces[index].1.update(cx, |workspace,cx| workspace.close_palette(window,cx));
        cx.notify();
    }
    fn scroll_to_workspace(&self) {
        let Some((_, entity)) = self.workspaces.get(self.active) else { return; };
        let visible = self.groups.visible_workspace(entity.entity_id());
        if let Some(row) = self.workspace_order(false).iter().position(|index|
            self.workspaces[*index].1.entity_id() == visible) {
            self.workspace_scroll.scroll_to_item(row);
        }
    }
    fn workspace_order(&self, include_collapsed: bool) -> Vec<usize> {
        let entries: Vec<_> = self.workspaces.iter().map(|(_, entity)| (entity.entity_id(), false)).collect();
        self.groups.ordered_indices(&entries, include_collapsed)
    }
    fn cycle(&mut self, direction: isize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = rmux_core::workspace_groups::cycle_target(&self.workspace_order(true), self.active, direction) {
            self.select(index, window, cx);
        }
    }
    fn add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.add_workspace(true, window, cx);
    }
    fn add_workspace(&mut self, inherit_group: bool, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.workspaces.len() >= 16 { return false; }
        let current = self.workspaces.get(self.active).map(|(_, entity)| entity.entity_id());
        let group = current.filter(|_| inherit_group).and_then(|id| self.groups.containing(id)).map(|group| group.id);
        let name = (1..).map(|number| format!("Workspace {number}"))
            .find(|name| !self.workspaces.iter().any(|(existing, _)| existing == name)).unwrap();
        let entity = Self::new_workspace_view(window, cx);
        if let Some(group) = group {
            self.groups.add_new(group, entity.entity_id(), current, rmux_core::workspace_groups::Placement::AfterCurrent)
                .expect("current group exists");
        }
        let index = if group.is_some() { self.workspaces.len() } else {
            let entries: Vec<_> = self.workspaces.iter().map(|(_, entity)| (entity.entity_id(), false)).collect();
            self.groups.new_workspace_index(&entries, self.active, Default::default())
        };
        // Preserve the outgoing workspace identity until select releases its caches.
        if !self.workspaces.is_empty() && index <= self.active { self.active += 1; }
        self.workspaces.insert(index, (name,entity));
        self.select(index,window,cx);
        true
    }
    fn new_group(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.add_workspace(false, window, cx) { return; }
        let name = (1..).map(|number| format!("Group {number}"))
            .find(|name| !self.groups.entries().iter().any(|group| &group.name == name)).unwrap();
        self.groups.create(name, vec![self.workspaces[self.active].1.entity_id()], true)
            .expect("new workspace is a valid group anchor");
        cx.notify();
    }

}
impl Render for BrowserHost {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().flex().track_focus(&self.empty_focus).font_family("Inter").text_color(rgb(0xe2e3e7))
            .on_action(cx.listener(|this, _: &NewWorkspaceGroup, window, cx| this.new_group(window, cx)))
            .on_action(cx.listener(|this, _: &NewWorkspace, window, cx| this.add(window, cx)))
            .on_action(cx.listener(|this, _: &NextWorkspace, window, cx| this.cycle(1, window, cx)))
            .on_action(cx.listener(|this, _: &PreviousWorkspace, window, cx| this.cycle(-1, window, cx)))
            .on_action(cx.listener(|this, action: &SelectWorkspaceNumber, window, cx| {
                if let Some(index) = rmux_core::workspace_groups::numbered_target(&this.workspace_order(true), action.number) {
                    this.select(index, window, cx);
                }
            }))
            .child(sidebar_presentation::sidebar(220.)
                .child(sidebar_presentation::header()
                    .child(div().flex_1().text_xs().text_color(rgb(0xa0a0a0)).child("Workspaces"))
                    .child(div().id("new-workspace").px_2().cursor_pointer().child("+")
                        .on_click(cx.listener(|this,_,window,cx| this.add(window,cx)))))
                .child(div().id("workspace-list").flex_1().min_h_0().overflow_y_scroll().track_scroll(&self.workspace_scroll).py_2()
                    .children(self.workspace_order(false).into_iter().map(|index| {
                        let (name, entity) = &self.workspaces[index];
                        let group = self.groups.containing(entity.entity_id());
                        let header = group.filter(|group| group.members.first() == Some(&entity.entity_id()));
                        let selected = index == self.active || self.groups.visible_workspace(
                            self.workspaces[self.active].1.entity_id()) == entity.entity_id();
                        sidebar_presentation::row(("workspace",entity.entity_id()),selected)
                            .when(group.is_some() && header.is_none(), |row| row.ml(px(20.)))
                            .child(div().flex().items_center().gap_2()
                                .when_some(header, |row, group| {
                                    let id = group.id;
                                    row.child(div().id(("group-collapse",entity.entity_id()))
                                        .w(px(24.)).h(px(24.)).flex_shrink_0().flex().items_center().justify_center()
                                        .child(if group.collapsed { "›" } else { "⌄" })
                                        .on_mouse_down(MouseButton::Left, |_,_,cx| cx.stop_propagation())
                                        .on_click(cx.listener(move |this,_,_,cx| {
                                            cx.stop_propagation();
                                            if let Some(group) = this.groups.get_mut(id) { group.collapsed = !group.collapsed; }
                                            cx.notify();
                                        })))
                                })
                                .child(header.map(|group| group.name.clone()).unwrap_or_else(|| name.clone())))
                            .on_click(cx.listener(move |this,_,window,cx| this.select(index,window,cx)))
                    })) ))

             .child(div().flex_1().min_w_0().h_full().child(
                if let Some((_, workspace)) = self.workspaces.get(self.active) {
                    workspace.clone().into_any_element()
                } else {
                    div().size_full().bg(rgb(0x151517)).flex().flex_col().items_center().justify_center().gap_3()
                        .child("No workspaces open")
                        .child(div().id("empty-new-workspace").px_4().py_2().rounded_md().bg(rgb(0x303030)).cursor_pointer()
                            .child("New Workspace").on_click(cx.listener(|this,_,window,cx| this.add(window,cx))))
                        .into_any_element()
                }))
    }
}
