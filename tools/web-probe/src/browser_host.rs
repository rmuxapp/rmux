//! Browser service host; workspace presentation comes from native sources.
use gpui::{prelude::*, *};
use crate::{InputProbe, WorkspaceCommand, PaletteRequest, CommandId, sidebar_presentation};
pub(crate) struct BrowserHost {
    workspace_state: rmux_core::workspace_lifecycle::State<(String, Entity<InputProbe>), EntityId>,
    sidebar: bool,
    sidebar_width: f32,
    sidebar_drag: Option<(Pixels, f32)>,
    empty_focus: FocusHandle,
    workspace_scroll: ScrollHandle,
}
impl BrowserHost {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let workspaces = ["rmux", "website"].into_iter().map(|name|
            (name.to_owned(), Self::new_workspace_view(name.to_owned(), window, cx))).collect();
        let mut host = Self { workspace_state: rmux_core::workspace_lifecycle::State { entries: workspaces, ..Default::default() }, sidebar: true, sidebar_width: crate::sidebar_interaction::DEFAULT_WIDTH, sidebar_drag: None, empty_focus: cx.focus_handle(), workspace_scroll: ScrollHandle::new() };
        host.select(0,window,cx);
        host
    }
    fn new_workspace_view(name: String, window: &mut Window, cx: &mut Context<Self>) -> Entity<InputProbe> {
        let entity = cx.new(|cx| InputProbe::new(name, window, cx));
        cx.subscribe_in(&entity, window, |this, _, command: &WorkspaceCommand, window, cx| {
            crate::workspace_actions::WorkspaceCommandHost::run_workspace_command(this, command.0, window, cx);
        }).detach();
        cx.subscribe_in(&entity, window, |this, source, request: &PaletteRequest, window, cx| {
            match request {
                PaletteRequest::Open => {
                    let workspaces = this.workspace_order(true).into_iter().enumerate().map(|(position, index)| {
                        let (name, entity) = &this.workspace_state.entries[index];
                        (entity.entity_id(), format!("Switch to Workspace {}: {name}", position + 1))
                    }).collect();
                    source.update(cx, |workspace, cx| workspace.set_workspace_results(workspaces, cx));
                }
                PaletteRequest::SelectWorkspace(id) => {
                    if let Some(index) = this.workspace_state.entries.iter().position(|(_, entity)| entity.entity_id() == *id) {
                        this.select(index, window, cx);
                    }
                }
            }
        }).detach();
        entity
    }
    fn close_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let index = self.workspace_state.active;
        let Some(_closed) = self.workspace_state.close(index, |(_, entity)| (entity.entity_id(), false),
        ) else { return; };
        if self.workspace_state.entries.is_empty() { self.empty_focus.focus(window, cx); cx.notify(); }
        else { self.select(self.workspace_state.active, window, cx); }
    }
    fn ungroup(&mut self, cx: &mut Context<Self>) {
        let Some((_, selected)) = self.workspace_state.entries.get(self.workspace_state.active) else { return; };
        let Some(group) = self.workspace_state.groups.containing(selected.entity_id()).map(|group| group.id) else { return; };
        if !self.workspace_state.ungroup(group,
            |(_, entity)| (entity.entity_id(), false),
        ) { return; }
        self.scroll_to_workspace();
        cx.notify();
    }
    fn select(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.workspace_state.entries.len() { return; }
        self.workspace_state.entries[self.workspace_state.active].1.update(cx, |workspace,cx| {
            workspace.split_drag = None;
            for pane in workspace.samples.values() {
                pane.update(cx, |pane,cx| pane.release_render_caches(cx));
            }
        });
        self.workspace_state.select(index, |(_, entity)| entity.entity_id(),
        );
        self.scroll_to_workspace();
        self.workspace_state.entries[index].1.update(cx, |workspace,cx| workspace.close_palette(window,cx));
        cx.notify();
    }
    fn toggle_sidebar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sidebar = !self.sidebar;
        self.sidebar_drag = None;
        if let Some((_, workspace)) = self.workspace_state.entries.get(self.workspace_state.active) {
            workspace.update(cx, |workspace, cx| workspace.close_palette(window, cx));
        } else { self.empty_focus.focus(window, cx); }
        cx.notify();
    }
    fn last_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.workspace_state.navigation_target(rmux_core::workspace_lifecycle::Navigation::Previous, |(_, entity)| (entity.entity_id(), false)) {
            self.select(index, window, cx);
        }
    }
    fn scroll_to_workspace(&self) {
        if let Some(row) = self.workspace_state.sidebar_row(self.workspace_state.active, |(_, entity)| (entity.entity_id(), false)) {
            self.workspace_scroll.scroll_to_item(row);
        }
    }
    fn workspace_order(&self, include_collapsed: bool) -> Vec<usize> {
        self.workspace_state.order(include_collapsed, |(_, entity)| (entity.entity_id(), false))
    }
    fn cycle(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.workspace_state.navigation_target(rmux_core::workspace_lifecycle::Navigation::Cycle(delta), |(_, entity)| (entity.entity_id(), false)) {
            self.select(index, window, cx);
        }
    }
    fn add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.add_workspace(true, window, cx);
    }
    fn add_workspace(&mut self, inherit_group: bool, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.workspace_state.entries.len() >= 16 { return false; }
        let current = self.workspace_state.entries.get(self.workspace_state.active).map(|(_, entity)| entity.entity_id());
        let name = (1..).map(|number| format!("Workspace {number}"))
            .find(|name| !self.workspace_state.entries.iter().any(|(existing, _)| existing == name)).unwrap();
        let entity = Self::new_workspace_view(name.clone(), window, cx);
        let Ok(index) = self.workspace_state.insert((name, entity),
            rmux_core::workspace_lifecycle::Insertion {
                origin: current, inherit_group, placement: Default::default(),
            }, |(_, entity)| (entity.entity_id(), false),
        ) else { return false; };
        self.select(index,window,cx);
        true
    }
    fn create_group(&mut self, empty: bool, window: &mut Window, cx: &mut Context<Self>) {
        if empty && !self.add_workspace(false, window, cx) { return; }
        if self.workspace_state.entries.get(self.workspace_state.active).is_none() { return; }
        self.workspace_state.create_group(self.workspace_state.active, empty, |(_, entity)| entity.entity_id())
            .expect("new workspace is a valid group anchor");
        self.sidebar = true;
        self.scroll_to_workspace();
        cx.notify();
    }

}
impl crate::workspace_actions::WorkspaceCommandHost for BrowserHost {
    fn run_workspace_command(&mut self, command: CommandId, window: &mut Window, cx: &mut Context<Self>) {
        match command {
            CommandId::CloseWorkspace => self.close_workspace(window, cx),
            CommandId::NewWorkspace => self.add(window, cx),
            CommandId::NewWorkspaceGroup => self.create_group(true, window, cx),
            CommandId::GroupCurrentWorkspace => self.create_group(false, window, cx),
            CommandId::ToggleSidebar => self.toggle_sidebar(window, cx),
            CommandId::LastWorkspace => self.last_workspace(window, cx),
            CommandId::NextWorkspace => self.cycle(1, window, cx),
            CommandId::PreviousWorkspace => self.cycle(-1, window, cx),
            CommandId::UngroupWorkspaces => self.ungroup(cx),
            _ => {}
        }
    }
    fn select_workspace_number(&mut self, number: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.workspace_state.navigation_target(rmux_core::workspace_lifecycle::Navigation::Number(number), |(_, entity)| (entity.entity_id(), false)) {
            self.select(index, window, cx);
        }
    }
}
impl Render for BrowserHost {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let width = crate::sidebar_interaction::visible_width(self.sidebar_width, window);
        div().size_full().relative().flex().track_focus(&self.empty_focus).font_family("Inter").text_color(rgb(0xe2e3e7))
            .child(crate::sidebar_interaction::drag_capture(cx))
            .map(|root| crate::workspace_actions::bind(root, cx))
            .when(self.sidebar, |root| root.child(sidebar_presentation::sidebar(width)
                .child(sidebar_presentation::header()
                    .child(div().flex_1().text_xs().text_color(rgb(0xa0a0a0)).child("Workspaces"))
                    .child(div().id("new-workspace").px_2().cursor_pointer().child("+")
                        .on_click(cx.listener(|this,_,window,cx| this.add(window,cx)))))
                .child(div().id("workspace-list").flex_1().min_h_0().overflow_y_scroll().track_scroll(&self.workspace_scroll).py_2()
                    .children(self.workspace_order(false).into_iter().map(|index| {
                        let (name, entity) = &self.workspace_state.entries[index];
                        let group = self.workspace_state.groups.containing(entity.entity_id());
                        let header = group.filter(|group| group.members.first() == Some(&entity.entity_id()));
                        let selected = index == self.workspace_state.active || self.workspace_state.groups.visible_workspace(
                            self.workspace_state.entries[self.workspace_state.active].1.entity_id()) == entity.entity_id();
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
                                            if let Some(group) = this.workspace_state.groups.get_mut(id) { group.collapsed = !group.collapsed; }
                                            cx.notify();
                                        })))
                                })
                                .child(header.map(|group| group.name.clone()).unwrap_or_else(|| name.clone())))
                            .on_click(cx.listener(move |this,_,window,cx| this.select(index,window,cx)))
                    })) )).child(crate::sidebar_interaction::divider(width, cx)))

             .child(div().flex_1().min_w_0().h_full().child(
                if let Some((_, workspace)) = self.workspace_state.entries.get(self.workspace_state.active) {
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

impl crate::sidebar_interaction::SidebarHost for BrowserHost {
    fn sidebar_state(&mut self) -> (&mut f32, &mut Option<(Pixels, f32)>) {
        (&mut self.sidebar_width, &mut self.sidebar_drag)
    }
}
