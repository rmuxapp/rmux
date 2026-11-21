// Included in sample_pane: platform/session actions stay with the pane host.
#[derive(Clone)]
struct SampleTabMenu {
    target: EntityId,
    position: Point<Pixels>,
    focus: FocusHandle,
    selected: usize,
    scroll: ScrollHandle,
    typeahead: crate::menu_navigation::Typeahead,
}
const MENU_LABELS: &[&str] = &["New Tab", "Rename Tab…", "Move Tab Left", "Move Tab Right", "Close Tab", "Close Other Tabs", "Close Tabs to the Right"];
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
        self.menu = Some(SampleTabMenu { target, position, focus, selected, scroll: ScrollHandle::new(), typeahead: Default::default() });
        cx.notify();
    }
    fn activate_menu(&mut self, item: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(menu) = self.menu.clone() else { return; };
        if !self.menu_enabled(menu.target).get(item).copied().unwrap_or(false) { return; }
        self.dismiss_menu(window, cx);
        let Some(index) = self.terminals.iter().position(|tab| tab.entity_id() == menu.target) else { return; };
        self.select(index, window, cx);
        use rmux_core::commands::CommandId::*;
        let commands = [NewTab, RenameTab, MoveTabLeft, MoveTabRight, ClosePane, CloseOtherTabs, CloseTabsToRight];
        crate::tab_actions::TabCommandHost::run_tab_command(self, commands[item], window, cx);
    }
    fn menu_element(&self, menu: SampleTabMenu, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let enabled = self.menu_enabled(menu.target);
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
                    match event.keystroke.key.as_str() {
                        "up" | "down" => {
                            for step in 1..=enabled.len() {
                                let candidate = (menu.selected + if event.keystroke.key == "up" { enabled.len() - step } else { step }) % enabled.len();
                                if enabled[candidate] { menu.selected = candidate; break; }
                            }
                        }
                        "home" => menu.selected = enabled.iter().position(|value| *value).unwrap_or(0),
                        "end" => menu.selected = enabled.iter().rposition(|value| *value).unwrap_or(0),
                        "enter" | "space" => { let selected = menu.selected; this.activate_menu(selected, window, cx); }
                        "escape" => this.dismiss_menu(window, cx),
                        _ => if let Some(selected) = typed { menu.selected = selected; },
                    }
                    if let Some(menu) = &this.menu { menu.scroll.scroll_to_item(menu.selected); }
                    cx.stop_propagation();
                    cx.notify();
                }))
                .child(div().id("sample-tab-menu-scroll").max_h((window.viewport_size().height - px(26.)).max(px(1.)))
                    .overflow_y_scroll().track_scroll(&menu.scroll)
                    .children(MENU_LABELS.iter().enumerate().map(|(index, label)| {
                        tab_presentation::menu_row(index, matches!(index, 1 | 2 | 4))
                            .when(enabled[index], |row| row.cursor_pointer()
                                .when(index == menu.selected, |row| row.bg(rgb(0x354969)))
                                .on_mouse_move(cx.listener(move |this, _, _, cx| {
                                    if let Some(menu) = &mut this.menu {
                                        if menu.selected != index { menu.selected = index; cx.notify(); }
                                    }
                                })))
                            .when(!enabled[index], |row| row.text_color(rgb(0x777777)))
                            .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                                if event.standard_click() { this.activate_menu(index, window, cx); }
                                cx.stop_propagation();
                            })).child(*label)
                    })))
        )).priority(2).into_any_element()
    }
}
