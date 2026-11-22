// Included in browser_host; rendering and commands come from desktop sources.
#[derive(Clone)]
struct WorkspaceMenu {
    target: EntityId,
    group: Option<u64>,
    position: Point<Pixels>,
    focus: FocusHandle,
    return_focus: Option<FocusHandle>,
    hints: Vec<Option<String>>,
    selected: usize,
    scroll: ScrollHandle,
    typeahead: crate::menu_navigation::Typeahead,
}
impl BrowserHost {
    fn dismiss_workspace_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let previous = self.menu.take();
        self.menu_blur = None;
        if let Some(menu) = previous {
            if menu.focus.is_focused(window) {
                if let Some(focus) = menu.return_focus { focus.focus(window, cx); }
            }
        }
        cx.notify();
    }
    fn show_workspace_menu(&mut self, target: EntityId, group: Option<u64>, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.workspace_state.entries.iter().position(|entry| entry.1.entity_id() == target) else { return; };
        self.workspace_state.sidebar_selection.clear();
        self.workspace_state.sidebar_selection_anchor = Some(target);
        self.select(index, window, cx);
        let return_focus = window.focused(cx);
        let hints = crate::workspace_menu_view::commands(group.is_some()).iter().map(|command| {
            let action = crate::workspace_actions::action(*command)?;
            let binding = window.highest_precedence_binding_for_action_in(action.as_ref(), return_focus.as_ref()?)?;
            Some(binding.keystrokes().iter().map(|key| crate::shortcut_label::format_key_parts(
                key.key(), *key.modifiers(), crate::terminal_engine::is_mac())).collect::<Vec<_>>().join(" "))
        }).collect();
        let focus = cx.focus_handle();
        let token = focus.clone();
        self.menu_blur = Some(cx.on_blur(&focus, window, move |this, _, cx| {
            if this.menu.as_ref().is_some_and(|menu| menu.focus == token) {
                this.menu = None;
                this.menu_blur = None;
                cx.notify();
            }
        }));
        focus.focus(window, cx);
        self.menu = Some(WorkspaceMenu { target, group, position, focus, return_focus, hints, selected: 0, scroll: ScrollHandle::new(), typeahead: Default::default() });
        cx.notify();
    }
    fn activate_workspace_menu(&mut self, item: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(menu) = self.menu.clone() else { return; };
        let Some(command) = crate::workspace_menu_view::commands(menu.group.is_some()).get(item).copied() else { return; };
        self.dismiss_workspace_menu(window, cx);
        let Some(index) = self.workspace_state.entries.iter().position(|entry| entry.1.entity_id() == menu.target) else { return; };
        if menu.group.is_some_and(|id| !self.workspace_state.groups.containing(menu.target).is_some_and(|group| group.id == id)) { return; }
        self.select(index, window, cx);
        crate::workspace_actions::WorkspaceCommandHost::run_workspace_command(self, command, window, cx);
    }
    fn workspace_menu_element(&self, menu: WorkspaceMenu, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let pinned = menu.group.map_or_else(
            || self.workspace_state.entries.iter().any(|entry| entry.1.entity_id() == menu.target && entry.2),
            |id| self.workspace_state.groups.entries().iter().any(|group| group.id == id && group.pinned));
        let name = menu.group.and_then(|id| self.workspace_state.groups.entries().iter().find(|group| group.id == id).map(|group| group.name.clone()))
            .or_else(|| self.workspace_state.entries.iter().find(|entry| entry.1.entity_id() == menu.target).map(|entry| entry.0.clone()))
            .unwrap_or_else(|| "Unavailable workspace".into());
        let labels = crate::workspace_menu_view::labels(menu.group.is_some(), pinned);
        deferred(anchored().position(menu.position).snap_to_window().child(
            crate::workspace_menu_view::frame(window).track_focus(&menu.focus).key_context("WorkspaceMenu")
                .on_mouse_down_out(cx.listener(|this, _, window, cx| this.dismiss_workspace_menu(window, cx)))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_down(MouseButton::Middle, |_, _, cx| cx.stop_propagation())
                .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                    let Some(menu) = &mut this.menu else { return; };
                    let mods = event.keystroke.modifiers;
                    let typed = if !mods.control && !mods.alt && !mods.platform && !mods.function {
                        menu.typeahead.select(&event.keystroke.key, &labels, &[true; 6], menu.selected, web_time::Instant::now())
                    } else { None };
                    match crate::menu_navigation::action_key(&event.keystroke).unwrap_or("") {
                        "up" | "down" | "home" | "end" => {
                            if let Some(selected) = crate::menu_navigation::navigate(&event.keystroke.key, menu.selected, &[true; 6]) { menu.selected = selected; }
                        }
                        "enter" | "space" => { let selected = menu.selected; this.activate_workspace_menu(selected, window, cx); }
                        "escape" => this.dismiss_workspace_menu(window, cx),
                        _ => if let Some(selected) = typed { menu.selected = selected; },
                    }
                    if let Some(menu) = &this.menu { menu.scroll.scroll_to_item(menu.selected); }
                    cx.stop_propagation();
                    cx.notify();
                }))
                .child(crate::workspace_menu_view::target_header(name.into(), if menu.group.is_some() { "Workspace group" } else { "Workspace" }))
                .child(div().id("workspace-menu-scroll").max_h((window.viewport_size().height - px(74.)).max(px(1.)))
                    .overflow_y_scroll().track_scroll(&menu.scroll)
                    .children(labels.into_iter().enumerate().map(|(index, label)| {
                        crate::workspace_menu_view::row(index, menu.selected)
                            .on_mouse_move(cx.listener(move |this, _, _, cx| {
                                if let Some(menu) = &mut this.menu { if menu.selected != index { menu.selected = index; cx.notify(); } }
                            }))
                            .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                                if event.standard_click() { this.activate_workspace_menu(index, window, cx); }
                                cx.stop_propagation();
                            }))
                            .child(div().flex_1().min_w_0().truncate().child(label))
                            .when_some(menu.hints[index].clone(), |row, hint| row.child(div().max_w(px(120.)).flex_shrink_0().truncate().text_xs().text_color(rgb(0xa0a0a0)).child(hint)))
                    })))
        )).priority(2).into_any_element()
    }
}
