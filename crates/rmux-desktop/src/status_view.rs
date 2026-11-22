use super::*;
use rmux_terminal::program_status::{Kind, Record, State as TaskState};

#[derive(Clone)]
pub struct State {
    pub focus: FocusHandle,
    pub selected: Option<String>,
    pub scroll: ScrollHandle,
}

fn selected_index(records: &[Record], id: Option<&str>) -> Option<usize> {
    if records.is_empty() { return None; }
    Some(records.iter().position(|record| Some(record.id.as_str()) == id).unwrap_or(0))
}

fn report(record: &Record, ended: bool) -> String {
    let id = if record.id.is_empty() { "Root" } else { &record.id };
    let state = match record.kind { Kind::Permission => "Approval".into(), Kind::Question => "Question".into(),
        Kind::Auth => "Login".into(), Kind::None => format!("{:?}", record.state) };
    let progress = record.progress.map_or(String::new(), |value| format!(" · {value}%"));
    let metadata = [&record.app, &record.title].into_iter().filter(|value| !value.is_empty())
        .map(String::as_str).collect::<Vec<_>>().join(" · ");
    [if ended { "Terminal finished · Last reported status".into() } else { String::new() }, format!("{id} · {state}{progress}"), metadata, record.message.clone()].into_iter()
        .filter(|line| !line.is_empty()).collect::<Vec<_>>().join("\n")
}

impl TerminalView {
    pub(super) fn close_status_details(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let focused = self.status_ui.as_ref().is_some_and(|state| state.focus.contains_focused(window, cx));
        self.status_ui = None;
        self.status_details = None;
        self.status_load = None;
        if focused { self.focus.focus(window); }
        cx.notify();
    }
    fn copy_status_record(&self, cx: &mut Context<Self>) {
        let Some((records, state)) = self.status_details.as_ref().zip(self.status_ui.as_ref()) else { return; };
        if let Some(index) = selected_index(records, state.selected.as_deref()) {
            clipboard::write(ClipboardItem::new_string(report(&records[index], self.process_exit.is_some())), cx);
        }
    }
    fn status_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.modifiers.platform || event.keystroke.modifiers.control || event.keystroke.modifiers.alt
            || (event.keystroke.modifiers.shift && event.keystroke.key != "tab") { return; }
        if event.keystroke.key == "escape" { self.close_status_details(window, cx); }
        else if event.keystroke.key == "c" { self.copy_status_record(cx); }
        else if let Some((records, state)) = self.status_details.as_ref().zip(self.status_ui.as_mut()) {
            if let Some(current) = selected_index(records, state.selected.as_deref()) {
                let key = if event.keystroke.key == "tab" {
                    if event.keystroke.modifiers.shift { "up" } else { "down" }
                } else { event.keystroke.key.as_str() };
                let next = match key {
                    "home" => Some(0), "end" => Some(records.len() - 1),
                    key @ ("pageup" | "pagedown") => command_palette::paged_result(
                        current, records.len(), key == "pageup",
                        f32::from(state.scroll.bounds().size.height), |index| {
                            // Cards wrap to their message height. Include the margin
                            // between cards instead of treating every task as one line.
                            state.scroll.bounds_for_item(index).map(|bounds| {
                                state.scroll.bounds_for_item(index + 1)
                                    .map(|next| f32::from(next.top() - bounds.top()))
                                    .unwrap_or_else(|| f32::from(bounds.size.height) + 8.)
                            }).unwrap_or(1.).max(1.)
                        }),
                    key => command_palette::navigated_result(current, records.len(), key),
                };
                if let Some(next) = next {
                    state.selected = Some(records[next].id.clone());
                    state.scroll.scroll_to_item(next);
                    cx.notify();
                }
            }
        }
        cx.stop_propagation();
    }
}

#[cfg(test)]
mod tests {
    use super::{Record, TaskState, Kind, selected_index, report};
    fn record(id: &str) -> Record {
        Record { id: id.into(), state: TaskState::Blocked, kind: Kind::Permission,
            progress: Some(42), app: "builder".into(), title: String::new(), message: "Review 世界\nnext line".into() }
    }
    #[test]
    fn selected_task_survives_reordering_and_deleted_tasks_fall_back() {
        let records = [record("a"), record("b")];
        assert_eq!(selected_index(&records, Some("b")), Some(1));
        assert_eq!(selected_index(&[record("b"), record("a")], Some("b")), Some(0));
        assert_eq!(selected_index(&records, Some("removed")), Some(0));
        assert_eq!(selected_index(&[], Some("b")), None);
        assert_eq!(report(&record(""), false), "Root · Approval · 42%\nbuilder\nReview 世界\nnext line");
        assert_eq!(report(&record(""), true), "Terminal finished · Last reported status\nRoot · Approval · 42%\nbuilder\nReview 世界\nnext line");
    }
}

pub fn panel(records: &[Record], loading: bool, ended: bool, state: &State, window: &Window, cx: &mut Context<TerminalView>) -> AnyElement {
    let selected = selected_index(records, state.selected.as_deref());
    let focused = state.focus.is_focused(window);
    div().track_focus(&state.focus).key_context("ProgramStatus")
        .on_key_down(cx.listener(TerminalView::status_key))
        .on_action(cx.listener(|view, _: &CopySelection, _, cx| { view.copy_status_record(cx); cx.stop_propagation(); }))
        .min_w_0()
        .overflow_hidden()
        .h(px(220.))
        .max_h(relative(0.45))
        .flex_shrink_0()
        .flex()
        .flex_col()
        .bg(rgb(0x222222))
        .border_b_1()
        .border_color(rgb(0x383838))
        .on_mouse_down(MouseButton::Left, cx.listener(|view, _, window, cx| {
            if let Some(state) = &view.status_ui { state.focus.focus(window); }
            cx.stop_propagation();
        }))
        .on_mouse_down(MouseButton::Middle, |_, _, cx| cx.stop_propagation())
        .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
        .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_mouse_up(MouseButton::Middle, |_, _, cx| cx.stop_propagation())
        .on_mouse_up(MouseButton::Right, |_, _, cx| cx.stop_propagation())
        .on_mouse_move(|_, _, cx| cx.stop_propagation())
        .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
        .child(
            div()
                .h(px(32.))
                .flex_shrink_0()
                .px_3()
                .flex()
                .items_center()
                .min_w_0()
                .gap_2()
                .text_sm()
                .child(div().flex_1().min_w_0().truncate().child(if loading { "Program status · Loading…".to_owned() } else if ended { format!("Last reported status · {} records", records.len()) } else { format!("Program status · {} records", records.len()) }))
                .child(
                    pane_view::pane_control("close-program-status", "Close Program Status")
                        .on_click(cx.listener(|view, event: &ClickEvent, window, cx| {
                            if !event.standard_click() { return; }
                            cx.stop_propagation();
                            view.close_status_details(window, cx);
                        }))
                        .child(icons::icon("close")),
                ),
        )
        .child(
            div()
                .id("program-status-records")
                .flex_1()
                .min_h_0()
                .min_w_0()
                .overflow_y_scroll().track_scroll(&state.scroll)
                .p_2()
                .when(records.is_empty(), |list| {
                    list.child(div().p_2().text_sm().child(if loading { "Loading reported tasks…" } else { "No reported tasks" }))
                })
                .children(records.iter().enumerate().map(|(index, record)| {
                    let tint = match record.state {
                        TaskState::Blocked => 0xf0b85b,
                        TaskState::Error => 0xf18a8a,
                        TaskState::Done => 0x8dd6a1,
                        TaskState::Working => 0x86b8f5,
                        TaskState::Idle => 0x9ba6b8,
                    };
                    let state = match record.kind {
                        Kind::Permission => "Approval".to_owned(),
                        Kind::Question => "Question".to_owned(),
                        Kind::Auth => "Login".to_owned(),
                        Kind::None => format!("{:?}", record.state),
                    };
                    let progress = record
                        .progress
                        .map_or(String::new(), |value| format!(" · {value}%"));
                    let id = if record.id.is_empty() {
                        "Root"
                    } else {
                        &record.id
                    };
                    let heading: SharedString = format!("{id} · {state}{progress}").into();
                    let metadata = [&record.app, &record.title].into_iter().filter(|value| !value.is_empty())
                        .map(String::as_str).collect::<Vec<_>>().join(" · ");
                    let report = report(record, ended);
                    let record_id = record.id.clone();
                    div().id(("status-record", index))
                        .on_mouse_down(MouseButton::Left, cx.listener(move |view, _, window, cx| {
                            if let Some(state) = &mut view.status_ui { state.selected = Some(record_id.clone()); state.focus.focus(window); }
                            cx.notify(); cx.stop_propagation();
                        }))
                        .min_w_0().overflow_hidden()
                        .mb_2()
                        .ml(px(record.id.matches('/').count().min(8) as f32 * 8.))
                        .p_2()
                        .rounded_sm()
                        .bg(rgb(if selected == Some(index) { if focused { crate::chrome_style::SELECTED } else { 0x333333 } } else { 0x2b2b2b }))
                        .text_sm()
                        .child(
                            div().flex().items_center().min_w_0().gap_2()
                                .child(div().id(("status-heading", index)).flex_1().min_w_0().truncate().text_color(rgb(tint))
                                    .tooltip({ let heading = heading.clone(); move |_, cx| cx.new(|_| pane_view::ControlHint(heading.clone())).into() })
                                    .child(heading))
                                .child(pane_view::pane_control(("copy-status", index), "Copy complete task details")
                                    .on_click(move |event: &ClickEvent, _, cx| {
                                        if !event.standard_click() { return; }
                                        cx.stop_propagation();
                                        clipboard::write(ClipboardItem::new_string(report.clone()), cx);
                                    }).child(icons::icon("copy"))),
                        )
                        .when(!record.app.is_empty() || !record.title.is_empty(), |row| {
                            row.child(
                                div().id(("status-metadata", index)).min_w_0().truncate()
                                    .text_xs().text_color(rgb(0xaaaaaa))
                                    .tooltip({ let metadata: SharedString = metadata.clone().into(); move |_, cx| cx.new(|_| pane_view::ControlHint(metadata.clone())).into() })
                                    .child(metadata),
                            )
                        })
                        .when(!record.message.is_empty(), |row| {
                            row.child(div().mt_1().min_w_0().whitespace_normal().child(record.message.clone()))
                        })
                })),
        )
        .child(div().flex_shrink_0().px_2().py_1().text_xs().text_color(rgb(0xaaaaaa)).child("↑↓ / Tab Select · C Copy · Esc Close"))
        .into_any_element()
}
