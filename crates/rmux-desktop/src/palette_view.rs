//! Shared palette presentation. Hosts supply commands, actions and result content.
use gpui::{prelude::*, *};
pub(crate) fn top_inset(height: f32) -> f32 {
    if height >= 480. { 48. } else { 8. }
}

// Modified arrows belong to text editing or configured shortcuts, not list navigation.
pub(crate) fn navigation_key(key: &Keystroke) -> Option<&str> {
    let modifiers = key.modifiers;
    if modifiers.platform || modifiers.control || modifiers.alt || modifiers.shift { return None; }
    matches!(key.key.as_str(), "up" | "down" | "pageup" | "pagedown").then_some(key.key.as_str())
}

pub(crate) fn navigated_result(selected: usize, count: usize, key: &str) -> Option<usize> {
    if count == 0 { return None; }
    let selected = selected.min(count - 1);
    match key {
        "up" => Some(if selected == 0 { count - 1 } else { selected - 1 }),
        "down" => Some(if selected + 1 == count { 0 } else { selected + 1 }),
        "pageup" => Some(selected.saturating_sub(8)),
        "pagedown" => Some(selected.saturating_add(8).min(count - 1)),
        _ => None,
    }
}

pub(crate) fn panel(height: f32) -> Stateful<Div> {
    div().id("command-palette").w(px(520.)).max_w(relative(0.9)).h_auto()
        .max_h(px((height - top_inset(height) - 8.).max(0.))).overflow_hidden()
        .flex().flex_col().rounded_lg().border_1().border_color(rgb(0x454545)).bg(rgb(0x252525))
}
pub(crate) fn header(text: impl Into<SharedString>, close: impl IntoElement) -> Div {
    div().flex().flex_shrink_0().items_center().justify_between().gap_2().px_3().pt_3()
        .child(div().min_w_0().flex_1().text_size(px(13.)).text_color(rgb(0xbcbcc4)).child(text.into()))
        .child(close)
}
pub(crate) fn close_button() -> Stateful<Div> {
    div().id("close-command-palette").flex_shrink_0().px_2().h(px(28.))
        .flex().items_center().rounded_sm().text_xs().text_color(rgb(0xbcbcc4))
        .cursor_pointer().hover(|style| style.bg(rgb(0x454545)).text_color(rgb(0xffffff)))
        .child("Close ×")
}
pub(crate) fn search(input: Entity<crate::text_field::TextField>, cx: &App) -> Div {
    let can_clear = !input.read(cx).value().is_empty() && !input.read(cx).is_composing();
    div().w_full().min_w_0().flex_shrink_0().p_3().child(div().w_full().min_w_0().px_3().py_2().rounded_md()
        .flex().items_center().gap_2()
        .border_1().border_color(rgb(crate::chrome_style::SELECTED_BORDER)).bg(rgb(0x12141a))
        .child(div().flex_1().min_w_0().child(input.clone()))
        .when(can_clear, |field| field.child(div().id("clear-palette-search")
            .flex_shrink_0().w(px(24.)).h(px(24.)).flex().items_center().justify_center()
            .rounded_sm().text_sm().text_color(rgb(0xbcbcc4)).cursor_pointer()
            .hover(|style| style.bg(rgb(0x353535)).text_color(rgb(0xffffff)))
            .tooltip(|_, cx| cx.new(|_| crate::control_hint::ControlHint("Clear search".into())).into())
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(move |event, window, cx| {
                cx.stop_propagation();
                if !event.standard_click() { return; }
                input.update(cx, |input, cx| {
                    input.replace_draft("", cx);
                    crate::text_layout::focus(&input.focus, window, cx);
                });
            }).child("×"))))
}
pub(crate) fn result_row_height(has_subtitle: bool) -> f32 {
    if has_subtitle { 56. } else { 36. }
}

pub(crate) fn result(id: usize, height: f32, selected: bool) -> Stateful<Div> {
    div().id(("command", id)).w_full().min_w_0().px_3().h(px(height))
        .text_sm().line_height(px(20.)).flex().items_center().gap_3().cursor_pointer()
        .when(selected, |row| row.bg(rgb(crate::chrome_style::SELECTED)))
        .hover(move |row| row.bg(rgb(if selected { crate::chrome_style::SELECTED_HOVER } else { 0x303030 })))
}
pub(crate) fn footer() -> Div {
    div().flex_shrink_0().px_3().py_2().border_t_1().border_color(rgb(0x363636))
        .flex().flex_wrap().items_center().gap_2().text_xs().text_color(rgb(0xa0a0a0))
}

pub(crate) fn shortcut(text: impl Into<SharedString>) -> Div {
    div().flex_shrink_0().max_w(relative(0.4)).truncate().px_2().py_1()
        .text_xs().line_height(px(16.)).rounded_sm().border_1().border_color(rgb(0x45454b))
        .bg(rgb(0x2b2b30)).text_color(rgb(0xbcbcc4)).child(text.into())
}

pub(crate) fn navigation_hints() -> Div {
    div().flex().flex_wrap().items_center().gap_3()
        .children([("↑↓", "Navigate"), ("↵", "Run"), ("Esc", "Close")].into_iter().map(|(key, label)|
            div().flex().items_center().gap_1()
                .child(div().px_1().rounded_sm().border_1().border_color(rgb(0x45454b))
                    .text_color(rgb(0xbcbcc4)).child(key))
                .child(label)))
}

pub(crate) fn empty_state(message: impl Into<SharedString>) -> Div {
    div().w_full().px_3().py_6().flex_shrink_0().text_center().text_sm()
        .text_color(rgb(0xbcbcc4)).child(message.into())
}

// Keep the destination within one viewport of the starting row. Always advance
// at least one result, including in a window shorter than a single row.
pub(crate) fn paged_result(selected: usize, count: usize, backwards: bool, viewport: f32, row_height: impl Fn(usize) -> f32) -> Option<usize> {
    if count == 0 { return None; }
    let mut next = selected.min(count - 1);
    let mut occupied = row_height(next);
    loop {
        let candidate = if backwards { next.checked_sub(1) } else { next.checked_add(1).filter(|index| *index < count) };
        let Some(candidate) = candidate else { break; };
        occupied += row_height(candidate);
        if occupied > viewport && next != selected.min(count - 1) { break; }
        next = candidate;
        if occupied >= viewport { break; }
    }
    Some(next)
}

#[cfg(test)]
mod keyboard_tests {
    use super::navigation_key;
    use gpui::Keystroke;
    #[test]
    fn result_navigation_leaves_modified_keys_to_their_bindings() {
        for name in ["up", "down", "pageup", "pagedown"] {
            let key = Keystroke::parse(name).unwrap();
            assert_eq!(navigation_key(&key), Some(name));
            for prefix in ["shift", "ctrl", "alt", "cmd"] {
                let key = Keystroke::parse(&format!("{prefix}-{name}")).unwrap();
                assert_eq!(navigation_key(&key), None);
            }
        }
        for name in ["left", "right", "home", "end", "enter", "escape", "a"] {
            assert_eq!(navigation_key(&Keystroke::parse(name).unwrap()), None);
        }
    }
}
