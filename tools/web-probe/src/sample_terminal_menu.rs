// Included in terminal_sample; commands use the shared action dispatch.
#[derive(Clone)]
struct TerminalMenu {
    position: Point<Pixels>,
    focus: FocusHandle,
    selected: usize,
    hints: Vec<Option<String>>,
    scroll: ScrollHandle,
    typeahead: crate::menu_navigation::Typeahead,
}
const MENU_LABELS: &[&str] = &["Copy", "Copy with Line Breaks", "Paste", "Select All", "Find…", "Clear Scrollback"];
fn menu_action(index: usize) -> Box<dyn Action> {
    match index {
        0 => Box::new(crate::CopySelection),
        1 => Box::new(crate::CopySelectionWithLineBreaks),
        2 => Box::new(crate::Paste),
        3 => Box::new(crate::EditorSelectAll),
        4 => Box::new(crate::Find),
        _ => Box::new(crate::ClearScrollback),
    }
}
impl TerminalSample {
    fn paste_from_clipboard(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.clipboard_read.is_some() { return; }
        // Begin synchronously while the click/keypress still grants activation.
        let read = match crate::browser_clipboard::read() {
            Ok(read) => read,
            Err(_) => { self.input_error = Some("Clipboard access was unavailable. Use your keyboard paste shortcut.".into()); cx.notify(); return; }
        };
        self.clipboard_read = Some(cx.spawn_in(window, async move |entity, cx| {
            let result = wasm_bindgen_futures::JsFuture::from(read).await;
            let _ = entity.update_in(cx, |this, window, cx| {
                this.clipboard_read = None;
                match result {
                    Ok(value) => match value.as_string().filter(|text| !text.is_empty()) {
                        Some(text) => EntityInputHandler::paste(this, ClipboardItem::new_string(text), window, cx),
                        None => this.input_error = Some("The clipboard has no text to paste.".into()),
                    },
                    Err(error) => this.input_error = Some(error.as_string().unwrap_or_else(|| "Could not read clipboard text.".into())),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn menu_enabled(&self) -> [bool; 6] {
        let selected = self.frame.selections.iter().any(Option::is_some);
        [selected, selected, self.clipboard_read.is_none(), true, true, true]
    }
    fn dismiss_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let focused = self.menu.as_ref().is_some_and(|menu| menu.focus.is_focused(window));
        self.menu = None;
        self.menu_blur = None;
        if focused { self.focus(window, cx); }
        cx.notify();
    }
    fn show_menu(&mut self, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        self.selecting = false;
        self.selection_pointer = None;
        self.selection_scroll = None;
        self.scrollbar_drag = None;
        let hints = (0..MENU_LABELS.len()).map(|index| {
            window.highest_precedence_binding_for_action_in(menu_action(index).as_ref(), &self.focus).map(|binding|
                binding.keystrokes().iter().map(|key| crate::shortcut_label::format_key_parts(
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
        let selected = self.menu_enabled().iter().position(|enabled| *enabled).unwrap_or(0);
        self.menu = Some(TerminalMenu { hints, position, focus, selected, scroll: ScrollHandle::new(), typeahead: Default::default() });
        cx.notify();
    }
    fn activate_menu(&mut self, item: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.menu.is_none() || !self.menu_enabled().get(item).copied().unwrap_or(false) { return; }
        self.dismiss_menu(window, cx);
        self.focus(window, cx);
        window.dispatch_action(menu_action(item), cx);
    }
    fn menu_element(&self, menu: TerminalMenu, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let enabled = self.menu_enabled();
        deferred(anchored().position(menu.position).snap_to_window().child(
            crate::tab_presentation::menu_frame(window).track_focus(&menu.focus).key_context("TerminalMenu")
                .on_mouse_down_out(cx.listener(|this, _, window, cx| this.dismiss_menu(window, cx)))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    if this.menu.is_none() { return; }
                    let enabled = this.menu_enabled();
                    let menu = this.menu.as_mut().unwrap();
                    let mods = event.keystroke.modifiers;
                    let typed = if !mods.control && !mods.alt && !mods.platform && !mods.function {
                        menu.typeahead.select(&event.keystroke.key, MENU_LABELS, &enabled, menu.selected, web_time::Instant::now())
                    } else { None };
                    match event.keystroke.key.as_str() {
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
                .child(div().id("sample-terminal-menu-scroll").max_h((window.viewport_size().height - px(26.)).max(px(1.)))
                    .overflow_y_scroll().track_scroll(&menu.scroll)
                    .children(MENU_LABELS.iter().enumerate().map(|(index, label)| {
                        crate::tab_presentation::menu_row(index, matches!(index, 3 | 5))
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
