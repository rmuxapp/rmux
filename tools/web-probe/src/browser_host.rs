//! Browser service host; workspace presentation comes from native sources.
use gpui::{prelude::*, *};
use crate::{InputProbe, sidebar_presentation, NewWorkspace, NewWorkspaceGroup, NextWorkspace, PreviousWorkspace, SelectWorkspaceNumber};
pub(crate) struct BrowserHost {
    workspaces: Vec<(String, Entity<InputProbe>)>,
    active: usize,
    groups: rmux_core::workspace_groups::Groups<EntityId>,
}
impl BrowserHost {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let workspaces = ["rmux", "website"].into_iter().map(|name|
            (name.to_owned(), cx.new(|cx| InputProbe::new(window, cx)))).collect();
        let mut host = Self { workspaces, active: 0, groups: Default::default() };
        host.select(0,window,cx);
        host
    }
    fn select(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.workspaces.len() { return; }
        self.workspaces[self.active].1.update(cx, |workspace,cx| {
            for pane in workspace.samples.values() {
                pane.update(cx, |pane,cx| pane.release_render_caches(cx));
            }
        });
        self.active=index;
        let member = self.workspaces[index].1.entity_id();
        if let Some(group) = self.groups.containing(member) {
            if group.members.first() != Some(&member) {
                let id = group.id;
                self.groups.get_mut(id).unwrap().collapsed = false;
            }
        }
        self.workspaces[index].1.update(cx, |workspace,cx| workspace.close_palette(window,cx));
        cx.notify();
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
        let current = self.workspaces[self.active].1.entity_id();
        let group = inherit_group.then(|| self.groups.containing(current).map(|group| group.id)).flatten();
        let name = format!("Workspace {}", self.workspaces.len()+1);
        let entity = cx.new(|cx| InputProbe::new(window,cx));
        if let Some(group) = group {
            self.groups.add_new(group, entity.entity_id(), Some(current), rmux_core::workspace_groups::Placement::AfterCurrent)
                .expect("current group exists");
        }
        self.workspaces.push((name,entity));
        self.select(self.workspaces.len()-1,window,cx);
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
        div().size_full().flex().font_family("Inter").text_color(rgb(0xe2e3e7))
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
                .child(div().id("workspace-list").flex_1().min_h_0().overflow_y_scroll().py_2()
                    .children(self.workspace_order(false).into_iter().map(|index| {
                        let (name, entity) = &self.workspaces[index];
                        let group = self.groups.containing(entity.entity_id());
                        let header = group.filter(|group| group.members.first() == Some(&entity.entity_id()));
                        let selected = index == self.active || header.is_some_and(|group|
                            group.collapsed && group.members.contains(&self.workspaces[self.active].1.entity_id()));
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

            .child(div().flex_1().min_w_0().h_full().child(self.workspaces[self.active].1.clone()))
    }
}
