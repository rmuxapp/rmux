//! Find controls shared by desktop and browser session adapters.
use gpui::{prelude::*, *};
use crate::text_field::TextField;
pub(crate) fn query_input(query: &str, cx: &mut Context<TextField>) -> TextField {
    let mut input = TextField::new(query, cx);
    input.placeholder = Some("Find in terminal…");
    input
}
pub(crate) fn bindings(mac: bool) -> Vec<KeyBinding> {
    ["Terminal", "FindInput"].into_iter().flat_map(|context| [
        KeyBinding::new(if mac { "cmd-f" } else { "ctrl-shift-f" }, crate::Find, Some(context)),
        KeyBinding::new(if mac { "cmd-g" } else { "f3" }, crate::FindOlder, Some(context)),
        KeyBinding::new(if mac { "cmd-shift-g" } else { "shift-f3" }, crate::FindNewer, Some(context)),
    ]).collect()
}
pub(crate) fn action(command: rmux_core::commands::CommandId) -> Option<Box<dyn Action>> {
    use rmux_core::commands::CommandId;
    match command {
        CommandId::Find => Some(Box::new(crate::Find)),
        CommandId::FindOlder => Some(Box::new(crate::FindOlder)),
        CommandId::FindNewer => Some(Box::new(crate::FindNewer)),
        _ => None,
    }
}
pub(crate) trait FindHost: Sized + 'static {
    fn find_input(&self) -> Entity<TextField>;
    fn find_history(&mut self) -> &mut rmux_core::search_state::QueryHistory;
    fn focus_find(&mut self, window: &mut Window, cx: &mut Context<Self>);
    fn navigate_find(&mut self, older: bool, cx: &mut Context<Self>);
    fn close_find(&mut self, cx: &mut Context<Self>);
}
fn history_key<V: FindHost>(this: &mut V, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<V>) {
    let key = &event.keystroke;
    let input = this.find_input();
    if !key.modifiers.alt || key.modifiers.control || key.modifiers.platform || key.modifiers.shift
        || !matches!(key.key.as_str(), "up" | "down") || input.read(cx).is_composing() { return; }
    if let Some(query) = this.find_history().recall(key.key == "up", input.read(cx).value()) {
        input.update(cx, |input, cx| {
            input.replace_draft(&query, cx);
            input.select_all_text(cx);
        });
    }
    cx.stop_propagation();
}
fn control(id: &'static str, hint: &'static str) -> Stateful<Div> {
    crate::pane_presentation::control_button(id)
        .tooltip(move |_, cx| cx.new(|_| crate::control_hint::ControlHint(hint.into())).into())
}
pub(crate) fn bar<V: FindHost>(input: Entity<TextField>, status: String, status_error: bool, cx: &mut Context<V>) -> Div {
        let has_query = !input.read(cx).value().is_empty();
        let can_clear = has_query && !input.read(cx).is_composing();
        let can_navigate = can_clear && !status_error;
        div()
            .key_context("FindInput")
            .on_key_down(cx.listener(history_key::<V>))
            .min_h(px(34.))
            .flex_shrink_0()
            .flex().flex_wrap().min_w_0()
            .items_center()
            .gap_1()
            .px_2().py_1()
            .bg(rgb(0x242424))
            .border_b_1()
            .border_color(rgb(0x353535))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    this.focus_find(window, cx);
                    cx.stop_propagation();
                }),
            )
            .on_mouse_down(MouseButton::Middle, |_, _, cx| cx.stop_propagation())
            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
            .on_mouse_move(|_, _, cx| cx.stop_propagation())
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .child(div().id("find-query").flex_1().min_w(px(80.)).max_w_full()
                .flex().items_center().gap_1().px_2().rounded_sm().border_1()
                .border_color(rgb(if status_error { 0xb85c5c } else { crate::chrome_style::SELECTED_BORDER }))
                .bg(rgb(0x12141a))
                .tooltip(|_, cx| cx.new(|_| crate::control_hint::ControlHint("Find in terminal · Enter / Shift+Enter navigates matches · Alt+Up/Down recalls queries".into())).into())
                .child(div().flex_1().min_w_0().child(input))
                .when(can_clear, |field| field.child(div().id("find-clear").flex_shrink_0()
                    .px_1().text_xs().text_color(rgb(0xaaaaaa)).cursor_pointer()
                    .hover(|style| style.text_color(rgb(0xffffff)))
                    .tooltip(|_, cx| cx.new(|_| crate::control_hint::ControlHint("Clear search; keep Find open".into())).into())
                    .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                        cx.stop_propagation();
                        if !event.standard_click() { return; }
                        this.find_input().update(cx, |input, cx| input.replace_draft("", cx));
                        this.focus_find(window, cx);
                    })).child("Clear"))))
            .child(div().flex().items_center().gap_1().min_w_0().max_w_full()
                .child(div().id("find-status").min_w_0().max_w(px(140.))
                    .text_xs()
                    .text_color(rgb(if status_error { 0xf08080 } else { 0xa0a0a0 }))
                    .truncate()
                    .tooltip({ let status: SharedString = status.clone().into(); move |_, cx| cx.new(|_| crate::control_hint::ControlHint(status.clone())).into() })
                    .child(if has_query || status_error { status.clone() } else { String::new() }))
                .child(control("find-older", "Older match · Enter")
                    .when(!can_navigate, |control| control.opacity(0.35).cursor_default())
                    .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                        if can_navigate && event.standard_click() { this.navigate_find(true, cx); }
                    })).child(crate::icons::icon("chevron-up")))
                .child(control("find-newer", "Newer match · Shift+Enter")
                    .when(!can_navigate, |control| control.opacity(0.35).cursor_default())
                    .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                        if can_navigate && event.standard_click() { this.navigate_find(false, cx); }
                    })).child(crate::icons::icon("chevron-down")))
                .child(control("find-close", "Close Find · Escape")
                    .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                        if event.standard_click() { this.close_find(cx); }
                    })).child(crate::icons::icon("close"))))
}
