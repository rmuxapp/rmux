//! Terminal actions share the keyboard paths and never recreate a session.
use super::*;

#[derive(Clone)]
pub(super) struct Menu {
    position: Point<Pixels>,
    focus: FocusHandle,
    selected: usize,
    scroll: ScrollHandle,
    link: Option<String>,
    typeahead: menu_navigation::Typeahead,
}
const LABELS: &[&str] = &["Copy", "Copy with Line Breaks", "Paste", "Select All", "Find…", "Copy Working Directory", "Reveal Working Directory", "Clear Scrollback", "Copy Link Address", "Open Link",
    #[cfg(all(target_os = "macos", feature = "browser"))]
    "Open Link in rmux Browser",
];

impl TerminalView {
    fn menu_enabled(&self) -> [bool; LABELS.len()] {
        let selection = self.frame.selections.iter().any(Option::is_some);
        let link = self.terminal_menu.as_ref().and_then(|menu| menu.link.as_deref());
        let local = self.session.remote_destination().is_none();
        [selection, selection, !self.session.paste_pending(), true, true, local || self.remote_directory_label.is_some(), local, true, link.is_some(), link.is_some_and(openable_uri),
            #[cfg(all(target_os = "macos", feature = "browser"))]
            link.is_some_and(browser_window::can_open_link),
        ]
    }
    pub(super) fn show_terminal_menu(&mut self, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        self.selecting = false;
        self.selection_scroll = None;
        self.selection_pointer = None;
        let link = if self.last_bounds.contains(&position) {
            let point = position - self.last_bounds.origin;
            self.session.hyperlink_at((f32::from(point.x) / self.cells.width) as u16,
                (f32::from(point.y) / self.cells.height) as u32, self.frame.revision).ok().flatten()
        } else { None };
        let focus = cx.focus_handle();
        let token = focus.clone();
        self.terminal_menu_blur = Some(cx.on_blur(&focus, window, move |this, _, cx| {
            if this.terminal_menu.as_ref().is_some_and(|menu| menu.focus == token) {
                this.terminal_menu = None;
                this.terminal_menu_blur = None;
                cx.notify();
            }
        }));
        let selected = self.menu_enabled().iter().position(|enabled| *enabled).unwrap_or(0);
        focus.focus(window);
        let scroll = ScrollHandle::new();
        scroll.scroll_to_item(selected);
        self.terminal_menu = Some(Menu { position, focus, selected, scroll, link, typeahead: Default::default() });
        cx.notify();
    }
    fn dismiss_terminal_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let focused = self.terminal_menu.as_ref().is_some_and(|menu| menu.focus.is_focused(window));
        self.terminal_menu = None;
        self.terminal_menu_blur = None;
        if focused { self.focus.focus(window); }
        cx.notify();
    }
    fn activate_terminal_menu(&mut self, item: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.terminal_menu.is_none() || !self.menu_enabled().get(item).copied().unwrap_or(false) { return; }
        // Freeze the clicked target, rather than looking up potentially changed output.
        let link = self.terminal_menu.as_ref().and_then(|menu| menu.link.clone());
        self.dismiss_terminal_menu(window, cx);
        match item {
            0 => self.copy(&CopySelection, window, cx),
            1 => self.copy_selection(false, cx),
            2 => self.paste(&Paste, window, cx),
            3 => self.select_all(&EditorSelectAll, window, cx),
            4 => self.open_find(&Find, window, cx),
            5 => self.lookup_working_directory(true, cx),
            6 => self.lookup_working_directory(false, cx),
            7 => self.clear_scrollback(&ClearScrollback, window, cx),
            8 => { if let Some(link) = link { clipboard::write(ClipboardItem::new_string(link), cx); } },
            9 => { if let Some(link) = link.filter(|uri| openable_uri(uri)) { cx.open_url(&link); } },
            #[cfg(all(target_os = "macos", feature = "browser"))]
            10 => {
                if let Some(link) = link {
                    if let Err(error) = browser_window::open_link(&link, cx) {
                        self.error = Some(error.to_string()); cx.notify();
                    }
                }
            },
            _ => {},
        }
    }
    pub(super) fn terminal_menu_element(&self, menu: Menu, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let enabled = self.menu_enabled();
        let actions: [Box<dyn Action>; 8] = [Box::new(CopySelection), Box::new(CopySelectionWithLineBreaks), Box::new(Paste), Box::new(EditorSelectAll), Box::new(Find), Box::new(CopyWorkingDirectory), Box::new(RevealWorkingDirectory), Box::new(ClearScrollback)];
        let mut hints: Vec<_> = actions.iter().map(|action| shortcuts::hint(action.as_ref(), &self.focus, window)).collect();
        hints.resize(LABELS.len(), None);
        deferred(anchored().position(menu.position).snap_to_window().child(
            div().id("terminal-context-menu").track_focus(&menu.focus).key_context("TerminalMenu")
                .w(px(340.).min((window.viewport_size().width - px(16.)).max(px(1.)))).py_1().bg(rgb(0x292929)).text_color(rgb(0xd6d6d6))
                .border_1().border_color(rgb(0x465063)).rounded_md().shadow_md().occlude()
                .on_mouse_down_out(cx.listener(|this, _, window, cx| this.dismiss_terminal_menu(window, cx)))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_down(MouseButton::Middle, |_, _, cx| cx.stop_propagation())
                .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                .on_mouse_move(|_, _, cx| cx.stop_propagation())
                .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    let enabled = this.menu_enabled();
                    let Some(menu) = this.terminal_menu.as_mut() else { return; };
                    let modifiers = event.keystroke.modifiers;
                    let typed = if !modifiers.control && !modifiers.alt && !modifiers.platform && !modifiers.function {
                        menu.typeahead.select(&event.keystroke.key, LABELS, &enabled, menu.selected, std::time::Instant::now())
                    } else { None };
                    match event.keystroke.key.as_str() {
                        "up" | "down" | "home" | "end" => {
                            if let Some(selected) = crate::menu_navigation::navigate(&event.keystroke.key, menu.selected, &enabled) {
                                menu.selected = selected;
                            }
                        }
                        "enter" | "space" => { let selected = menu.selected; this.activate_terminal_menu(selected, window, cx); },
                        "escape" => this.dismiss_terminal_menu(window, cx),
                        _ => { if let Some(selected) = typed { menu.selected = selected; } },
                    }
                    if let Some(menu) = &this.terminal_menu { menu.scroll.scroll_to_item(menu.selected); }
                    cx.stop_propagation();
                    cx.notify();
                }))
                .child(div().id("terminal-menu-scroll")
                    .max_h((window.viewport_size().height - px(26.)).max(px(1.)))
                    .overflow_y_scroll().track_scroll(&menu.scroll)
                    .children(LABELS.iter().enumerate().map(|(index, label)| {
                    div().id(("terminal-menu-item", index)).mx_1().px_3().py_2().rounded_sm().text_size(px(13.)).flex().items_center().gap_3()
                        .when(enabled[index], |row| row.cursor_pointer()
                            .when(index == menu.selected, |row| row.bg(rgb(crate::chrome_style::SELECTED)))
                            .on_mouse_move(cx.listener(move |this, _, _, cx| {
                                if let Some(menu) = &mut this.terminal_menu {
                                    if menu.selected != index { menu.selected = index; cx.notify(); }
                                }
                            })))
                        .when(!enabled[index], |row| row.text_color(rgb(0x777777)))
                        .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                            if event.standard_click() { this.activate_terminal_menu(index, window, cx); }
                            cx.stop_propagation();
                        }))
                        .child(div().flex_1().min_w_0().truncate().child(*label))
                        .when_some(hints[index].clone(), |row, hint| row.child(
                            div().id(("terminal-menu-shortcut", index)).max_w(px(120.)).flex_shrink_0().truncate().text_xs().text_color(rgb(0xa0a0a0))
                                .tooltip({ let hint = hint.clone(); move |_, cx| cx.new(|_| pane_view::ControlHint(hint.clone().into())).into() })
                                .child(hint)
                        ))
                })))
        )).with_priority(2).into_any_element()
    }
}
