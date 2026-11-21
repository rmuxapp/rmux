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
