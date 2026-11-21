//! Browser service host; workspace presentation comes from native sources.
use gpui::{prelude::*, *};
use crate::{InputProbe, WorkspaceCommand, PaletteRequest, CommandId, sidebar_presentation};
pub(crate) struct BrowserHost {
    rename: Option<(EntityId, Entity<crate::text_field::TextField>)>,
    workspace_state: rmux_core::workspace_lifecycle::State<(String, Entity<InputProbe>, bool), EntityId>,
    sidebar: bool,
    sidebar_width: f32,
    sidebar_drag: Option<(Pixels, f32)>,
    empty_focus: FocusHandle,
    workspace_scroll: ScrollHandle,
}
impl BrowserHost {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let workspaces = ["rmux", "website"].into_iter().map(|name|
            (name.to_owned(), Self::new_workspace_view(name.to_owned(), window, cx), false)).collect();
        let mut host = Self { rename: None, workspace_state: rmux_core::workspace_lifecycle::State { entries: workspaces, ..Default::default() }, sidebar: true, sidebar_width: crate::sidebar_interaction::DEFAULT_WIDTH, sidebar_drag: None, empty_focus: cx.focus_handle(), workspace_scroll: ScrollHandle::new() };
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
                        let (name, entity, _) = &this.workspace_state.entries[index];
                        (entity.entity_id(), format!("Switch to Workspace {}: {name}", position + 1))
                    }).collect();
                    let groups = this.workspace_state.groups.entries().iter().map(|group|
                        (group.id, format!("Move Workspace to Group: {} (#{})", group.name, group.id))).collect();
                    source.update(cx, |workspace, cx| workspace.set_workspace_results(workspaces, groups, window, cx));
                }
                PaletteRequest::SelectWorkspace(id) => {
                    if let Some(index) = this.workspace_state.entries.iter().position(|(_, entity, _)| entity.entity_id() == *id) {
                        this.select(index, window, cx);
                    }
                }
            }
        }).detach();
        entity
    }
    fn rename_workspace(&mut self, group: bool, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((_, input)) = &self.rename { input.read(cx).focus.clone().focus(window, cx); return; }
        let Some((name, workspace, _)) = self.workspace_state.entries.get(self.workspace_state.active) else { return; };
        let group = if group {
            let Some(group) = self.workspace_state.groups.containing(workspace.entity_id()) else { return; };
            Some(group)
        } else { None };
        let target = group.map_or(workspace.entity_id(), |group| group.members[0]);
        let name = group.map_or(name, |group| &group.name);
        let group_id = group.map(|group| group.id);
        let input = cx.new(|cx| crate::text_field::TextField::new(name, cx));
        cx.subscribe_in(&input, window, move |this, _, event: &crate::text_field::EditEvent, window, cx| {
            match event {
                crate::text_field::EditEvent::Submitted { text, .. } => {
                    if let Some(name) = rmux_core::workspace_lifecycle::normalized_name(text) {
                        if let Some(id) = group_id {
                            if let Some(group) = this.workspace_state.groups.get_mut(id) { group.name = name; }
                        } else if let Some((label, workspace, _)) = this.workspace_state.entries.iter_mut().find(|(_, workspace, _)| workspace.entity_id() == target) {
                            *label = name.clone();
                            workspace.update(cx, |workspace, cx| { workspace.name = name; cx.notify(); });
                        }
                    }
                }
                crate::text_field::EditEvent::Cancelled => {},
                crate::text_field::EditEvent::Changed(_) => return,
            }
            this.rename = None;
            this.select(this.workspace_state.active, window, cx);
        }).detach();
        self.sidebar = true;
        self.scroll_to_workspace();
        input.read(cx).focus.clone().focus(window, cx);
        self.rename = Some((target, input));
        cx.notify();
    }
    fn close_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.rename = None;
        let index = self.workspace_state.active;
        let Some(_closed) = self.workspace_state.close(index, |(_, entity, pinned)| (entity.entity_id(), *pinned),
        ) else { return; };
        if self.workspace_state.entries.is_empty() { self.empty_focus.focus(window, cx); cx.notify(); }
        else { self.select(self.workspace_state.active, window, cx); }
    }
    fn close_group(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((_, workspace, _)) = self.workspace_state.entries.get(self.workspace_state.active) else { return; };
        let Some(group) = self.workspace_state.groups.containing(workspace.entity_id()) else { return; };
        let id = group.id;
        let members = group.members.clone();
        self.rename = None;
        // Sample sessions have no external processes or unsaved local files.
        // Resolve each captured identity after the preceding close changes slots.
        for member in members.into_iter().rev() {
            if let Some(index) = self.workspace_state.group_close_index(id, member, |(_, entity, _)| entity.entity_id()) {
                self.workspace_state.close(index, |(_, entity, pinned)| (entity.entity_id(), *pinned));
            }
        }
        if self.workspace_state.entries.is_empty() { self.empty_focus.focus(window, cx); cx.notify(); }
        else { self.select(self.workspace_state.active, window, cx); }
    }
    fn ungroup(&mut self, cx: &mut Context<Self>) {
        let Some((_, selected, _)) = self.workspace_state.entries.get(self.workspace_state.active) else { return; };
        let Some(group) = self.workspace_state.groups.containing(selected.entity_id()).map(|group| group.id) else { return; };
        if !self.workspace_state.ungroup(group,
            |(_, entity, pinned)| (entity.entity_id(), *pinned),
        ) { return; }
        self.scroll_to_workspace();
        cx.notify();
    }
    fn select(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.workspace_state.entries.len() { return; }
        self.rename = None;
        self.workspace_state.entries[self.workspace_state.active].1.update(cx, |workspace,cx| {
            workspace.split_drag = None;
            for pane in workspace.samples.values() {
                pane.update(cx, |pane,cx| pane.release_render_caches(cx));
            }
        });
        self.workspace_state.select(index, |(_, entity, _)| entity.entity_id(),
        );
        self.scroll_to_workspace();
        self.workspace_state.entries[index].1.update(cx, |workspace,cx| workspace.close_palette(window,cx));
        cx.notify();
    }
    fn toggle_sidebar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sidebar = !self.sidebar;
        self.sidebar_drag = None;
        if let Some((_, workspace, _)) = self.workspace_state.entries.get(self.workspace_state.active) {
            workspace.update(cx, |workspace, cx| workspace.close_palette(window, cx));
        } else { self.empty_focus.focus(window, cx); }
        cx.notify();
    }
    fn last_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.workspace_state.navigation_target(rmux_core::workspace_lifecycle::Navigation::Previous, |(_, entity, pinned)| (entity.entity_id(), *pinned)) {
            self.select(index, window, cx);
        }
    }
    fn scroll_to_workspace(&self) {
        if let Some(row) = self.workspace_state.sidebar_row(self.workspace_state.active, |(_, entity, pinned)| (entity.entity_id(), *pinned)) {
            self.workspace_scroll.scroll_to_item(row);
        }
    }
    fn workspace_order(&self, include_collapsed: bool) -> Vec<usize> {
        self.workspace_state.order(include_collapsed, |(_, entity, pinned)| (entity.entity_id(), *pinned))
    }
    fn cycle(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.workspace_state.navigation_target(rmux_core::workspace_lifecycle::Navigation::Cycle(delta), |(_, entity, pinned)| (entity.entity_id(), *pinned)) {
            self.select(index, window, cx);
        }
    }
    fn add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.add_workspace(true, window, cx);
    }
    fn add_workspace(&mut self, inherit_group: bool, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.workspace_state.entries.len() >= 16 { return false; }
        let current = self.workspace_state.entries.get(self.workspace_state.active).map(|(_, entity, _)| entity.entity_id());
        let name = (1..).map(|number| format!("Workspace {number}"))
            .find(|name| !self.workspace_state.entries.iter().any(|(existing, _, _)| existing == name)).unwrap();
        let entity = Self::new_workspace_view(name.clone(), window, cx);
        let Ok(index) = self.workspace_state.insert((name, entity, false),
            rmux_core::workspace_lifecycle::Insertion {
                origin: current, inherit_group, placement: Default::default(),
            }, |(_, entity, pinned)| (entity.entity_id(), *pinned),
        ) else { return false; };
        self.select(index,window,cx);
        true
    }
    fn create_group(&mut self, empty: bool, window: &mut Window, cx: &mut Context<Self>) {
        if empty && !self.add_workspace(false, window, cx) { return; }
        if self.workspace_state.entries.get(self.workspace_state.active).is_none() { return; }
        self.workspace_state.create_group(self.workspace_state.active, empty, |(_, entity, _)| entity.entity_id())
            .expect("new workspace is a valid group anchor");
        self.sidebar = true;
        self.scroll_to_workspace();
        cx.notify();
    }

}
impl crate::workspace_actions::WorkspaceCommandHost for BrowserHost {
    fn move_workspace_to_group(&mut self, group: u64, cx: &mut Context<Self>) {
        if self.workspace_state.move_to_group(self.workspace_state.active, group, |(_, entity, _)| entity.entity_id()).is_ok() {
            self.scroll_to_workspace();
            cx.notify();
        }
    }
    fn run_workspace_command(&mut self, command: CommandId, window: &mut Window, cx: &mut Context<Self>) {
        match command {
            CommandId::CloseWorkspaceGroup => self.close_group(window, cx),
            CommandId::CloseWorkspace => self.close_workspace(window, cx),
            CommandId::ToggleWorkspacePin => {
                if let Some((_, _, pinned)) = self.workspace_state.entries.get_mut(self.workspace_state.active) {
                    *pinned = !*pinned;
                    self.scroll_to_workspace();
                    cx.notify();
                }
            }
            CommandId::RenameWorkspace => self.rename_workspace(false, window, cx),
            CommandId::RenameWorkspaceGroup => self.rename_workspace(true, window, cx),
            CommandId::NewWorkspace => self.add(window, cx),
            CommandId::NewWorkspaceGroup => self.create_group(true, window, cx),
            CommandId::GroupCurrentWorkspace => self.create_group(false, window, cx),
            CommandId::GroupSelectedWorkspaces => {
                match self.workspace_state.group_sidebar_selection(|(_, entity, pinned)| (entity.entity_id(), *pinned)) {
                    Ok(Some(_)) => { self.sidebar = true; self.scroll_to_workspace(); cx.notify(); }
                    Ok(None) => cx.propagate(),
                    Err(error) => eprintln!("Could not group workspaces: {error}"),
                }
            }
            CommandId::ToggleSidebar => self.toggle_sidebar(window, cx),
            CommandId::LastWorkspace => self.last_workspace(window, cx),
            CommandId::NextWorkspace => self.cycle(1, window, cx),
            CommandId::PreviousWorkspace => self.cycle(-1, window, cx),
            command @ (CommandId::ToggleGroupPin | CommandId::RemoveWorkspaceFromGroup | CommandId::SetGroupAnchor) => {
                use rmux_core::workspace_lifecycle::GroupEdit;
                let edit = match command {
                    CommandId::ToggleGroupPin => GroupEdit::TogglePin,
                    CommandId::SetGroupAnchor => GroupEdit::SetAnchor,
                    _ => GroupEdit::Remove,
                };
                if self.workspace_state.edit_group(self.workspace_state.active, edit, |(_, entity, _)| entity.entity_id()).unwrap_or(false) {
                    self.scroll_to_workspace();
                    cx.notify();
                }
            }
            command @ (CommandId::MoveWorkspaceUp | CommandId::MoveWorkspaceDown) => {
                let delta = if command == CommandId::MoveWorkspaceUp { -1 } else { 1 };
                if self.workspace_state.reorder(delta, |(_, entity, pinned)| (entity.entity_id(), *pinned)) {
                    self.scroll_to_workspace();
                    cx.notify();
                }
            }
            CommandId::UngroupWorkspaces => self.ungroup(cx),
            _ => {}
        }
    }
    fn select_workspace_number(&mut self, number: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.workspace_state.navigation_target(rmux_core::workspace_lifecycle::Navigation::Number(number), |(_, entity, pinned)| (entity.entity_id(), *pinned)) {
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
            // Sidebar clicks focus this host, outside the workspace action path.
            // Resolve palette commands here too, restoring the active terminal
            // context without changing sidebar selection.
            .on_action(cx.listener(|this, _: &crate::CommandPalette, window, cx| {
                if let Some((_, workspace, _)) = this.workspace_state.entries.get(this.workspace_state.active) {
                    workspace.update(cx, |workspace, cx| {
                        workspace.focus_terminal(window, cx);
                        workspace.open_palette(window, cx);
                    });
                }
            }))
            .when(self.sidebar, |root| root.child(sidebar_presentation::sidebar(width)
                .child(sidebar_presentation::header()
                    .child(div().flex_1().text_xs().text_color(rgb(0xa0a0a0)).child(if self.workspace_state.sidebar_selection.len() > 1 { format!("{} selected", self.workspace_state.sidebar_selection.len()) } else { "Workspaces".into() }))
                    .child(div().id("new-workspace").px_2().cursor_pointer().child("+")
                        .on_click(cx.listener(|this,_,window,cx| this.add(window,cx)))))
                .child(div().id("workspace-list").flex_1().min_h_0().overflow_y_scroll().track_scroll(&self.workspace_scroll).py_2()
                    .children(self.workspace_order(false).into_iter().map(|index| {
                        let (name, entity, pinned) = &self.workspace_state.entries[index];
                        let group = self.workspace_state.groups.containing(entity.entity_id());
                        let header = group.filter(|group| group.members.first() == Some(&entity.entity_id()));
                        let selected = self.workspace_state.sidebar_selection.contains(&entity.entity_id()) || index == self.workspace_state.active || self.workspace_state.groups.visible_workspace(
                            self.workspace_state.entries[self.workspace_state.active].1.entity_id()) == entity.entity_id();
                        sidebar_presentation::row(("workspace",entity.entity_id()),selected)
                            .when(group.is_some() && header.is_none(), |row| row.ml(px(20.)))
                            .child(div().flex().items_center().gap_2()
                                .when_some(header, |row, group| {
                                    let id = group.id;
                                    row.child(sidebar_presentation::group_toggle(("group-collapse", entity.entity_id()), group.collapsed)
                                        .on_click(cx.listener(move |this,event: &ClickEvent,_,cx| {
                                            if !event.standard_click() { return; }
                                            cx.stop_propagation();
                                            if let Some(group) = this.workspace_state.groups.get_mut(id) { group.collapsed = !group.collapsed; }
                                            cx.notify();
                                        })))
                                })
                                .child(div().min_w_0().flex_1().truncate().font_weight(FontWeight::SEMIBOLD)
                                    .child(header.map(|group| group.name.clone()).unwrap_or_else(|| name.clone())))
                                .when(header.map_or(*pinned, |group| group.pinned), |row| row.child(
                                    sidebar_presentation::pin(("workspace-pin", entity.entity_id()), selected, header.is_some()))))
                            .when_some(self.rename.as_ref().filter(|(target, _)| *target == entity.entity_id()), |row, (_, input)| row.child(
                                div().flex().flex_col().gap_1().child(input.clone())
                                    .child(div().text_xs().child("Enter to save · Esc to cancel"))))
                            .on_click(cx.listener(move |this,event: &ClickEvent,window,cx| {
                                if !event.standard_click() { return; }
                                let Some((_, entity, _)) = this.workspace_state.entries.get(index) else { return; };
                                let target = entity.entity_id();
                                if this.rename.as_ref().is_some_and(|(id, _)| *id == target) { return; }
                                if let ClickEvent::Mouse(mouse) = event {
                                    let modifiers = mouse.down.modifiers;
                                    let toggle = if crate::terminal_engine::is_mac() { modifiers.platform } else { modifiers.control };
                                    if toggle || modifiers.shift {
                                        this.workspace_state.extend_sidebar_selection(target, modifiers.shift, |(_, entity, pinned)| (entity.entity_id(), *pinned));
                                        cx.notify();
                                        return;
                                    }
                                }
                                this.workspace_state.sidebar_selection.clear();
                                this.workspace_state.sidebar_selection_anchor = Some(target);
                                this.select(index,window,cx);
                                if matches!(event, ClickEvent::Mouse(mouse) if mouse.down.click_count == 2) { let is_anchor = this.workspace_state.groups.containing(target).is_some_and(|group| group.members.first() == Some(&target));
                                    this.rename_workspace(is_anchor, window, cx); }
                            }))
                    })) )).child(crate::sidebar_interaction::divider(width, cx)))

             .child(div().flex_1().min_w_0().h_full().child(
                if let Some((_, workspace, _)) = self.workspace_state.entries.get(self.workspace_state.active) {
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
