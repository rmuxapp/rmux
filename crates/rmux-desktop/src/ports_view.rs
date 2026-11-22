use super::*;
#[derive(Default)]
pub(super) struct State {
    pub visible: bool,
    generation: u64,
    loading: bool,
    report: Option<ports::Report>,
    error: Option<String>,
    canceled: Option<Arc<AtomicBool>>,
    focus: Option<FocusHandle>,
    selected: usize,
    scroll: ScrollHandle,
}
impl Drop for State {
    fn drop(&mut self) {
        if let Some(canceled) = &self.canceled { canceled.store(true, Ordering::Release); }
    }
}
impl State {
    fn listener_offset(&self) -> usize {
        self.report.as_ref().map_or(0, |report| usize::from(report.ownership_changed)
            + usize::from(report.truncated)) + usize::from(self.error.is_some())
    }
}

fn refreshed_selection(listeners: &[ports::Listener], previous: Option<ports::Listener>) -> usize {
    previous.and_then(|previous| listeners.iter().position(|row| *row == previous)).unwrap_or(0)
}

impl TerminalView {
    fn reveal_selected_port(&self) {
        if self.ports.report.as_ref().is_some_and(|report| !report.listeners.is_empty()) {
            self.ports.scroll.scroll_to_item(self.ports.listener_offset() + self.ports.selected);
        }
    }
    fn copy_port(&self, cx: &mut Context<Self>) {
        if let Some(row) = self.ports.report.as_ref().and_then(|report| report.listeners.get(self.ports.selected)) {
            clipboard::write(ClipboardItem::new_string(row.address.to_string()), cx);
        }
    }
    fn open_port_http(&self, cx: &mut Context<Self>) {
        if let Some(url) = self.ports.report.as_ref().and_then(|report| report.listeners.get(self.ports.selected))
            .and_then(|row| ports::http_url(row.address)) { cx.open_url(&url); }
    }
    fn ports_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let count = self.ports.report.as_ref().map_or(0, |report| report.listeners.len());
        match event.keystroke.key.as_str() {
            "escape" => self.close_ports(window, cx),
            "r" if !event.keystroke.modifiers.control && !event.keystroke.modifiers.platform && !event.keystroke.modifiers.alt => self.refresh_ports(window, cx),
            "enter" => self.copy_port(cx),
            "o" if !event.keystroke.modifiers.control && !event.keystroke.modifiers.platform && !event.keystroke.modifiers.alt => self.open_port_http(cx),
            "up" if count > 0 => self.ports.selected = (self.ports.selected + count - 1) % count,
            "down" if count > 0 => self.ports.selected = (self.ports.selected + 1) % count,
            "home" => self.ports.selected = 0,
            "end" => self.ports.selected = count.saturating_sub(1),
            key @ ("pageup" | "pagedown") => {
                let offset = self.ports.listener_offset();
                if let Some(next) = command_palette::paged_result(self.ports.selected, count,
                    key == "pageup", f32::from(self.ports.scroll.bounds().size.height), |index| {
                        self.ports.scroll.bounds_for_item(offset + index)
                            .map(|bounds| f32::from(bounds.size.height)).unwrap_or(36.).max(1.)
                    }) { self.ports.selected = next; }
            }
            _ => {},
        }
        self.reveal_selected_port();
        cx.stop_propagation();
        cx.notify();
    }

    pub(super) fn close_ports(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(canceled) = self.ports.canceled.take() { canceled.store(true, Ordering::Release); }
        self.ports.generation = self.ports.generation.wrapping_add(1);
        self.ports.loading = false;
        if self.ports.focus.as_ref().is_some_and(|focus| focus.is_focused(window)) { self.focus.focus(window); }
        self.ports.visible = false;
        self.ports.report = None;
        self.ports.error = None;
        cx.notify();
    }
    pub(super) fn refresh_ports(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.ports.focus.get_or_insert_with(|| cx.focus_handle()).focus(window);
        if self.ports.loading { self.ports.visible = true; cx.notify(); return; }
        let previous = self.ports.report.as_ref()
            .and_then(|report| report.listeners.get(self.ports.selected)).copied();
        self.ports.visible = true;
        self.ports.error = None;
        self.ports.report = None;
        self.ports.selected = 0;
        let prepare = || -> Result<_> {
            anyhow::ensure!(self.session.exit_status().is_none(), "Terminal process has exited");
            let pid = self.session.process_id().ok_or_else(|| anyhow::anyhow!("Terminal has no local process ID"))?;
            Ok((pid, ports::Permit::acquire()?))
        };
        match prepare() {
            Err(error) => self.ports.error = Some(error.to_string()),
            Ok((pid, permit)) => {
                self.ports.loading = true;
                self.ports.generation = self.ports.generation.wrapping_add(1);
                let generation = self.ports.generation;
                let canceled = Arc::new(AtomicBool::new(false));
                self.ports.canceled = Some(canceled.clone());
                cx.spawn(async move |entity, cx| {
                    let result = cx.background_executor().spawn(async move {
                        let _permit = permit;
                        ports::scan(pid, &canceled)
                    }).await;
                    let _ = entity.update(cx, |this, cx| {
                        if this.ports.generation != generation { return; }
                        this.ports.loading = false;
                        this.ports.canceled = None;
                        if !this.ports.visible { return; }
                        if this.session.exit_status().is_some() { this.ports.error = Some("Terminal process has exited".into()); }
                        else { match result {
                            Ok(report) => {
                                this.ports.selected = report.as_ref().map_or(0, |report| refreshed_selection(&report.listeners, previous));
                                this.ports.report = report;
                            }
                            Err(error) => this.ports.error = Some(error.to_string()),
                        } }
                        this.reveal_selected_port();
                        cx.notify();
                    });
                }).detach();
            }
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::{ports::Listener, refreshed_selection};
    #[test]
    fn refreshed_listener_selection_tracks_address_and_owner() {
        let a = Listener { address: "127.0.0.1:3000".parse().unwrap(), pid: 10 };
        let b = Listener { address: "[::1]:4000".parse().unwrap(), pid: 20 };
        assert_eq!(refreshed_selection(&[b, a], Some(a)), 1);
        assert_eq!(refreshed_selection(&[a, b], Some(a)), 0);
        assert_eq!(refreshed_selection(&[b, Listener { pid: 30, ..a }], Some(a)), 0);
        assert_eq!(refreshed_selection(&[b, Listener { address: "127.0.0.1:3001".parse().unwrap(), ..a }], Some(a)), 0);
        assert_eq!(refreshed_selection(&[b], Some(a)), 0);
        assert_eq!(refreshed_selection(&[], Some(a)), 0);
        assert_eq!(refreshed_selection(&[b, a], None), 0);
    }
}
pub(super) fn panel(state: &State, window: &Window, cx: &mut Context<TerminalView>) -> AnyElement {
    div().key_context("ListeningPorts")
        .when_some(state.focus.as_ref(), |panel, focus| panel.track_focus(focus))
        .on_key_down(cx.listener(TerminalView::ports_key))
        .on_action(cx.listener(|this, _: &CopySelection, _, cx| { this.copy_port(cx); cx.stop_propagation(); }))
        .h(px(180.)).max_h(relative(0.4)).min_w_0().overflow_hidden().flex_shrink_0().flex().flex_col()
        .bg(rgb(0x242424)).border_b_1().border_color(rgb(0x383838)).text_size(px(12.))
        .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
            if let Some(focus) = &this.ports.focus { focus.focus(window); }
            cx.stop_propagation();
        }))
        .on_mouse_down(MouseButton::Middle, |_, _, cx| cx.stop_propagation())
        .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
        .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_mouse_move(|_, _, cx| cx.stop_propagation())
        .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
        .child(div().h(px(32.)).min_w_0().flex_shrink_0().px_2().flex().items_center().gap_2()
            .child(div().id("ports-heading").flex_1().min_w_0().truncate()
                .tooltip({ let text: SharedString = if state.loading { "Local TCP listeners · Scanning…" } else { "Local TCP listeners · Snapshot" }.into(); move |_, cx| cx.new(|_| pane_view::ControlHint(text.clone())).into() })
                .child(if state.loading { "Local TCP listeners · Scanning…" } else { "Local TCP listeners · Snapshot" }))
            .child(pane_view::pane_control("refresh-ports", "Refresh Listening Ports").on_click(cx.listener(|this, event: &ClickEvent, window, cx| { if event.standard_click() { this.refresh_ports(window, cx); } })).child(icons::icon("refresh")))
            .child(pane_view::pane_control("close-ports", "Close Listening Ports").on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                if event.standard_click() { this.close_ports(window, cx); }
            })).child(icons::icon("close"))))
        .child(div().id("port-list").flex_1().min_h_0().overflow_y_scroll().track_scroll(&state.scroll).px_2().pb_2()
            .when_some(state.error.clone(), |list, error| list.child(div().text_color(rgb(0xf2a5a5)).child(error)))
            .when_some(state.report.as_ref(), |list, report| list
                .when(report.listeners.is_empty(), |list| list.child("No local TCP listeners found for this terminal."))
                .when(report.ownership_changed, |list| list.child("Some process ownership changed during the scan; refresh for current results."))
                .when(report.truncated, |list| list.child("Scan limit reached; results are incomplete."))
                .children(report.listeners.iter().enumerate().map(|(index, row)| {
                    let address = row.address.to_string();
                    div().id(("listener-row", index)).py_1().px_1().rounded_sm().flex().items_center().gap_2().cursor_pointer()
                        .when(index == state.selected, |row| row.bg(rgb(if state.focus.as_ref().is_some_and(|focus| focus.is_focused(window)) { crate::chrome_style::SELECTED } else { 0x333333 })))
                        .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, window, cx| {
                            this.ports.selected = index;
                            if let Some(focus) = &this.ports.focus { focus.focus(window); }
                            cx.stop_propagation(); cx.notify();
                        }))
                        .child(div().id(("listener-address", index)).flex_1().min_w_0().truncate()
                            .tooltip({ let address: SharedString = address.clone().into(); move |_, cx| cx.new(|_| pane_view::ControlHint(address.clone())).into() })
                            .child(address.clone()))
                        .child(div().text_color(rgb(0xa0a0a0)).child(format!("PID {}", row.pid)))
                        .when_some(ports::http_url(row.address), |row, url| row.child(
                            pane_view::pane_control("open-port-http", "Open as HTTP in Default Browser").id(("open-port-http", index))
                                .on_click(move |event, _, cx| { if event.standard_click() { cx.stop_propagation(); cx.open_url(&url); } })
                                .child(icons::icon("external-link"))))
                        .child(pane_view::pane_control("copy-port", "Copy Listener Address").id(("copy-port", index))
                            .on_click(move |event, _, cx| { if event.standard_click() { clipboard::write(ClipboardItem::new_string(address.clone()), cx); } })
                            .child(icons::icon("copy")))
                }))))
        .child(div().px_2().py_1().text_xs().text_color(rgb(0xa0a0a0)).child("↑↓ Select · Enter Copy · O Open HTTP · R Refresh · Esc Close"))
        .into_any_element()
}
