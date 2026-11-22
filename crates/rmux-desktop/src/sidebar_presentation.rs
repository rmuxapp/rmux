//! Workspace sidebar geometry and selection styling shared by both hosts.
use gpui::{prelude::*, *};
pub(crate) use crate::chrome_style::{ACCENT, SELECTED_DETAIL};

/// Keep the useful end of a long directory visible in a narrow sidebar.
/// The caller retains the unabridged value in its tooltip.
pub(crate) fn directory_label(directory: &str) -> String {
    const LIMIT: usize = 28;
    if directory.chars().count() > LIMIT {
        let leaf = directory.rsplit(['/', '\\']).find(|part| !part.is_empty()).unwrap_or(directory);
        if leaf != directory && leaf.chars().count() <= LIMIT {
            return format!("…/{leaf}");
        }
    }
    match directory.char_indices().rev().nth(LIMIT - 1) {
        Some((offset, _)) if offset > 0 => format!("…{}", &directory[offset..]),
        _ => directory.to_owned(),
    }
}
pub(crate) fn sidebar(width: f32) -> Div {
    div().w(px(width)).overflow_hidden().flex_shrink_0().h_full().flex().flex_col().bg(rgb(0x242424))
}
pub(crate) fn header() -> Stateful<Div> {
    div().h(px(36.)).flex_shrink_0().px_3().flex().items_center().border_b_1()
        .border_color(rgb(0x383838)).text_sm().id("workspace-list-header")
}
pub(crate) fn row(id: impl Into<ElementId>, selected: bool) -> Stateful<Div> {
    div().id(id).mx_1().mb(px(2.)).px_3().py_2().rounded(px(5.)).text_size(px(13.)).cursor_pointer()
        .border_1().border_color(gpui::transparent_black())
        .when(selected, |row| row.bg(rgb(crate::chrome_style::SELECTED)).border_color(rgb(crate::chrome_style::SELECTED_BORDER)).text_color(rgb(crate::chrome_style::SELECTED_TEXT)))
        .hover(move |style| style.bg(rgb(if selected {crate::chrome_style::SELECTED_HOVER} else {0x303030})))
}

pub(crate) fn group_toggle(id: impl Into<ElementId>, collapsed: bool) -> Stateful<Div> {
    div().id(id).w(px(24.)).h(px(24.)).flex_shrink_0().flex().items_center().justify_center().rounded_sm()
        .hover(|style| style.bg(rgb(crate::chrome_style::SELECTED_HOVER)))
        .tooltip(|_, cx| cx.new(|_| crate::control_hint::ControlHint("Expand / Collapse Group".into())).into())
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(crate::icons::icon(if collapsed { "chevron-right" } else { "chevron-down" }))
}
pub(crate) fn pin(id: impl Into<ElementId>, selected: bool, group: bool) -> Stateful<Div> {
    div().id(id).flex_shrink_0()
        .tooltip(move |_, cx| cx.new(|_| crate::control_hint::ControlHint(
            if group { "Pinned group" } else { "Pinned workspace" }.into())).into())
        .child(crate::icons::icon("pin").text_color(rgb(if selected { SELECTED_DETAIL } else { 0x969696 })))
}

/// Custom workspace symbols keep their text shaping; default groups never
/// depend on a font containing a particular Unicode glyph.
pub(crate) fn workspace_icon(custom: Option<String>, group: bool, selected: bool) -> Option<AnyElement> {
    let color = rgb(if selected { SELECTED_DETAIL } else { 0xaaaaaa });
    let slot = div().w(px(16.)).flex_shrink_0().flex().items_center().justify_center();
    match custom {
        Some(icon) => Some(slot.text_color(color).child(icon).into_any_element()),
        None if group => Some(slot.child(crate::icons::icon("group").text_color(color)).into_any_element()),
        None => None,
    }
}

/// Shared title geometry and full-label access in compact sidebars.
pub(crate) fn title_frame(id: impl Into<ElementId>) -> Stateful<Div> {
    div().id(id).flex_1().min_w_0().truncate().font_weight(FontWeight::SEMIBOLD)
}
pub(crate) fn title(id: impl Into<ElementId>, text: SharedString) -> Stateful<Div> {
    let hint = text.clone();
    title_frame(id)
        .tooltip(move |_, cx| cx.new(|_| crate::control_hint::ControlHint(hint.clone())).into())
        .child(text)
}
pub(crate) fn group_add(id: impl Into<ElementId>) -> Stateful<Div> {
    div().id(id).w(px(24.)).h(px(24.)).flex_shrink_0()
        .flex().items_center().justify_center().rounded_sm()
        .child(crate::icons::icon("plus"))
        .hover(|style| style.bg(rgb(crate::chrome_style::SELECTED_HOVER)))
        .tooltip(|_, cx| cx.new(|_| crate::control_hint::ControlHint("New Workspace in Group".into())).into())
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
}
