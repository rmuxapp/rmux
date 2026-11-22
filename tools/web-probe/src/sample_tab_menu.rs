// Included in sample_pane: platform/session actions stay with the pane host.
#[derive(Clone)]
struct SampleTabMenu {
    target: EntityId,
    position: Point<Pixels>,
    focus: FocusHandle,
    selected: usize,
    hints: Vec<Option<String>>,
    scroll: ScrollHandle,
    typeahead: crate::menu_navigation::Typeahead,
}
const MENU_LABELS: &[&str] = &["New Tab", "Rename Tab…", "Move Tab Left", "Move Tab Right", "Close Tab", "Close Other Tabs", "Close Tabs to the Right"];
const MENU_COMMANDS: &[rmux_core::commands::CommandId] = {
    use rmux_core::commands::CommandId::*;
    &[NewTab, RenameTab, MoveTabLeft, MoveTabRight, ClosePane, CloseOtherTabs, CloseTabsToRight]
};
impl SamplePane {
    fn menu_enabled(&self, target: EntityId) -> [bool; 7] {
        let Some(index) = self.terminals.iter().position(|tab| tab.entity_id() == target) else { return [false; 7]; };
        [self.terminals.len() < 16, true, index > 0, index + 1 < self.terminals.len(), true,
            self.terminals.len() > 1, index + 1 < self.terminals.len()]
    }
    fn dismiss_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let focused = self.menu.as_ref().is_some_and(|menu| menu.focus.is_focused(window));
        self.menu = None;
        self.menu_blur = None;
        if focused { self.focus(window, cx); }
        cx.notify();
    }
    fn show_menu(&mut self, index: usize, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        if self.rename.is_some() { return; }
        let Some(target) = self.terminals.get(index).map(Entity::entity_id) else { return; };
        self.select(index, window, cx);
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
        let selected = self.menu_enabled(target).iter().position(|enabled| *enabled).unwrap_or(0);
        let hints = self.terminals[index].read(cx).focus_handle();
        let hints = MENU_COMMANDS.iter().map(|command| {
            let action = crate::tab_actions::action(*command)?;
            window.highest_precedence_binding_for_action_in(action.as_ref(), hints).map(|binding|
                binding.keystrokes().iter().map(|key| crate::shortcut_label::format_key_parts(
                    key.key(), *key.modifiers(), crate::terminal_engine::is_mac())).collect::<Vec<_>>().join(" "))
        }).collect();
        self.menu = Some(SampleTabMenu { hints, target, position, focus, selected, scroll: ScrollHandle::new(), typeahead: Default::default() });
        cx.notify();
    }
    fn activate_menu(&mut self, item: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(menu) = self.menu.clone() else { return; };
        if !self.menu_enabled(menu.target).get(item).copied().unwrap_or(false) { return; }
        self.dismiss_menu(window, cx);
        let Some(index) = self.terminals.iter().position(|tab| tab.entity_id() == menu.target) else { return; };
        self.select(index, window, cx);
        crate::tab_actions::TabCommandHost::run_tab_command(self, MENU_COMMANDS[item], window, cx);
    }
    fn menu_element(&self, menu: SampleTabMenu, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let enabled = self.menu_enabled(menu.target);
        let title = self.terminals.iter().find(|terminal| terminal.entity_id() == menu.target)
            .map(|terminal| terminal.read(cx).custom_title.clone().unwrap_or_else(|| self.labels[&menu.target].clone()))
            .unwrap_or_else(|| "Unavailable tab".into());
        deferred(anchored().position(menu.position).snap_to_window().child(
            tab_presentation::menu_frame(window).track_focus(&menu.focus).key_context("TabMenu")
                .on_mouse_down_out(cx.listener(|this, _, window, cx| this.dismiss_menu(window, cx)))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    let Some(target) = this.menu.as_ref().map(|menu| menu.target) else { return; };
                    let enabled = this.menu_enabled(target);
                    let menu = this.menu.as_mut().unwrap();
                    let mods = event.keystroke.modifiers;
                    let typed = if !mods.control && !mods.alt && !mods.platform && !mods.function {
                        menu.typeahead.select(&event.keystroke.key, MENU_LABELS, &enabled, menu.selected, web_time::Instant::now())
                    } else { None };
                    match crate::menu_navigation::action_key(&event.keystroke).unwrap_or("") {
                        "up" | "down" | "home" | "end" => {
                            if let Some(selected) = crate::menu_navigation::navigate(&event.keystroke.key, menu.selected, &enabled) {
                                menu.selected = selected;
                            }
                        }
                        "enter" | "space" => { let selected = menu.selected; this.activate_menu(selected, window, cx); }
                        "escape" => this.dismiss_menu(window, cx),
                        _ => if let Some(selected) = typed { menu.selected = selected; },
                    }
                    if let Some(menu) = &this.menu { menu.scroll.scroll_to_item(menu.selected); }
                    cx.stop_propagation();
                    cx.notify();
                }))
                .child(crate::workspace_menu_view::target_header(title.into(), "Terminal tab"))
                .child(div().id("sample-tab-menu-scroll").max_h((window.viewport_size().height - px(74.)).max(px(1.)))
                    .overflow_y_scroll().track_scroll(&menu.scroll)
                    .children(MENU_LABELS.iter().enumerate().map(|(index, label)| {
                        tab_presentation::menu_row(index, matches!(index, 1 | 2 | 4))
                            .when(enabled[index] && index >= 4, |row| row.text_color(rgb(0xffa4a4)))
                            .when(enabled[index], |row| row.cursor_pointer()
                                .when(index == menu.selected, |row| row.bg(rgb(crate::chrome_style::SELECTED)))
                                .on_mouse_move(cx.listener(move |this, _, _, cx| {
                                    if let Some(menu) = &mut this.menu {
                                        if menu.selected != index { menu.selected = index; cx.notify(); }
                                    }
                                })))
                            .when(!enabled[index], |row| row.text_color(rgb(0x777777)))
                            .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                                if event.standard_click() { this.activate_menu(index, window, cx); }
                                cx.stop_propagation();
                            })).child(div().flex_1().min_w_0().truncate().child(*label))
                            .when_some(menu.hints[index].clone(), |row, hint| row.child(
                                div().max_w(px(120.)).flex_shrink_0().truncate().text_xs().text_color(rgb(0xa0a0a0)).child(hint)))
                    })))
        )).priority(2).into_any_element()
    }
}
