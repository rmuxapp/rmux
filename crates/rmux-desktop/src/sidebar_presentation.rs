//! Workspace sidebar geometry and selection styling shared by both hosts.
use gpui::{prelude::*, *};
pub(crate) fn sidebar(width: f32) -> Div {
    div().w(px(width)).overflow_hidden().flex_shrink_0().h_full().flex().flex_col().bg(rgb(0x242424))
}
pub(crate) fn header() -> Stateful<Div> {
    div().h(px(36.)).flex_shrink_0().px_3().flex().items_center().border_b_1()
        .border_color(rgb(0x383838)).text_sm().id("workspace-list-header")
}
pub(crate) fn row(id: impl Into<ElementId>, selected: bool) -> Stateful<Div> {
    div().id(id).mx_1().mb(px(2.)).px_3().py_2().rounded(px(5.)).text_size(px(13.)).cursor_pointer()
        .when(selected, |row| row.bg(rgb(0x007aff)).text_color(rgb(0xffffff)))
        .hover(move |style| style.bg(rgb(if selected {0x1685ff} else {0x303030})))
}

pub(crate) fn group_toggle(id: impl Into<ElementId>, collapsed: bool) -> Stateful<Div> {
    div().id(id).w(px(24.)).h(px(24.)).flex_shrink_0().flex().items_center().justify_center().rounded_sm()
        .hover(|style| style.bg(rgb(0x43516a)))
        .tooltip(|_, cx| cx.new(|_| crate::control_hint::ControlHint("Expand / Collapse Group".into())).into())
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(crate::icons::icon(if collapsed { "chevron-right" } else { "chevron-down" }))
}
pub(crate) fn pin(id: impl Into<ElementId>, selected: bool, group: bool) -> Stateful<Div> {
    div().id(id).flex_shrink_0()
        .tooltip(move |_, cx| cx.new(|_| crate::control_hint::ControlHint(
            if group { "Pinned group" } else { "Pinned workspace" }.into())).into())
        .child(crate::icons::icon("pin").text_color(rgb(if selected { 0xdcecff } else { 0x969696 })))
}

/// Custom workspace symbols keep their text shaping; default groups never
/// depend on a font containing a particular Unicode glyph.
pub(crate) fn workspace_icon(custom: Option<String>, group: bool, selected: bool) -> Option<AnyElement> {
    let color = rgb(if selected { 0xdcecff } else { 0xaaaaaa });
    let slot = div().w(px(16.)).flex_shrink_0().flex().items_center().justify_center();
    match custom {
        Some(icon) => Some(slot.text_color(color).child(icon).into_any_element()),
        None if group => Some(slot.child(crate::icons::icon("group").text_color(color)).into_any_element()),
        None => None,
    }
}
