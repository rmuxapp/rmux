//! Pane sizing and divider appearance shared by native and browser hosts.
use gpui::{prelude::*, *};
use rmux_core::layout::Axis;

pub(crate) fn leaf(id: impl Into<ElementId>) -> Stateful<Div> {
    div().id(id).size_full().min_w_0().min_h_0().overflow_hidden()
}
pub(crate) fn weighted(weight: f32) -> Div {
    div().flex_basis(px(0.)).map(|mut child| {
        child.style().flex_grow = Some(weight); child
    }).min_w_0().min_h_0().overflow_hidden()
}
pub(crate) fn branch(axis: Axis) -> Div {
    div().size_full().flex().min_w_0().min_h_0()
        .when(axis == Axis::Rows, |root| root.flex_col())
}
pub(crate) fn divider(id: impl Into<ElementId>, axis: Axis, width: f32) -> Stateful<Div> {
    div().id(id).flex_shrink_0().bg(rgb(0x242424)).flex().items_center().justify_center()
        .child(if axis == Axis::Columns {
            div().w(px(1.)).h_full().bg(rgb(0x383838))
        } else { div().h(px(1.)).w_full().bg(rgb(0x383838)) })
        .when(axis == Axis::Columns, |bar| bar.w(px(width)).h_full())
        .when(axis == Axis::Rows, |bar| bar.h(px(width)).w_full())
}
