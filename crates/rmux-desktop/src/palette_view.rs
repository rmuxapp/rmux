//! Shared palette presentation. Hosts supply commands, actions and result content.
use gpui::{prelude::*, *};
pub(crate) fn top_inset(height: f32) -> f32 {
    if height >= 480. { 48. } else { 8. }
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
pub(crate) fn heading(text: impl Into<SharedString>) -> Div {
    div().flex_shrink_0().px_3().pt_2().text_xs().text_color(rgb(0xa0a0a0)).child(text.into())
}
pub(crate) fn search(input: impl IntoElement) -> Div {
    div().flex_shrink_0().p_3().child(div().w_full().px_3().py_2().rounded_md()
        .border_1().border_color(rgb(0x4b5668)).bg(rgb(0x12141a)).child(input))
}
pub(crate) fn result(id: usize, height: f32, selected: bool) -> Stateful<Div> {
    div().id(("command", id)).w_full().min_w_0().px_3().h(px(height))
        .text_sm().line_height(px(20.)).flex().items_center().gap_3().cursor_pointer()
        .when(selected, |row| row.bg(rgb(0x354969)))
        .hover(move |row| row.bg(rgb(if selected { 0x40577b } else { 0x303030 })))
}
pub(crate) fn footer() -> Div {
    div().flex_shrink_0().px_3().py_2().border_t_1().border_color(rgb(0x363636))
        .flex().flex_wrap().items_center().gap_2().text_xs().text_color(rgb(0xa0a0a0))
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
