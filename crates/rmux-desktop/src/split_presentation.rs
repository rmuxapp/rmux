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
    div().id(id).flex_shrink_0().bg(crate::chrome_style::surface()).flex().items_center().justify_center()
        .child(if axis == Axis::Columns {
            div().w(px(1.)).h_full().bg(crate::chrome_style::border())
        } else { div().h(px(1.)).w_full().bg(crate::chrome_style::border()) })
        .when(axis == Axis::Columns, |bar| bar.w(px(width)).h_full())
        .when(axis == Axis::Rows, |bar| bar.h(px(width)).w_full())
}

/// The same zoom status and action surface for both application hosts.
pub(crate) fn restore_layout<V: crate::pane_actions::PaneCommandHost>(hidden: usize, shortcut: Option<String>, cx: &mut Context<V>) -> Stateful<Div> {
    let status = format!("Pane zoomed · {hidden} {} hidden", if hidden == 1 { "pane" } else { "panes" });
    let hint: SharedString = match &shortcut {
        Some(shortcut) => format!("Restore split layout  {shortcut}"),
        None => "Restore split layout".into(),
    }.into();
    div().id("restore-pane-layout").h(px(28.)).min_w_0().flex_shrink_0().px_3()
        .flex().gap_2().items_center().text_xs().bg(crate::chrome_style::elevated()).cursor_pointer()
        .hover(|style| style.bg(crate::chrome_style::selected()))
        .tooltip(move |_, cx| cx.new(|_| crate::control_hint::ControlHint(hint.clone())).into())
        .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
            if event.standard_click() { this.run_pane_command(rmux_core::commands::CommandId::TogglePaneZoom, window, cx); }
        }))
        .child(crate::icons::icon("split-right"))
        .child(div().flex_1().min_w_0().truncate().child(status))
        .child(div().flex_shrink_0().child("Restore layout"))
        .when_some(shortcut, |row, shortcut| row.child(div().max_w(relative(0.3)).truncate().text_color(crate::chrome_style::accent()).child(shortcut)))
}
