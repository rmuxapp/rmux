//! Workspace sidebar geometry and selection styling shared by both hosts.
use gpui::{prelude::*, *};
pub(crate) use crate::chrome_style::{ACCENT, SELECTED_DETAIL};

/// Compact project context prioritizes the working folder, not its ancestors.
/// Full paths remain available in the row tooltip. Remote context includes
/// connection state and a host, so it must not be treated as a filesystem path.
pub(crate) fn directory_label(directory: &str) -> String {
    if directory.starts_with("SSH · ") || directory.starts_with("SSH ended · ") {
        return directory.to_owned();
    }
    let trimmed = directory.trim_end_matches(['/', '\\']);
    if trimmed.is_empty() || (trimmed.len() == 2 && trimmed.ends_with(':')) {
        return directory.to_owned();
    }
    trimmed.rsplit(['/', '\\']).next().unwrap_or(directory).to_owned()
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

/// Branch/directory geometry shared with the browser's explicit sample sessions.
pub(crate) fn context_row(id: impl Into<ElementId>, branch: Option<String>, directory: Option<String>, selected: bool) -> Stateful<Div> {
    let hint: SharedString = [branch.as_deref(), directory.as_deref()].into_iter().flatten().collect::<Vec<_>>().join("\n").into();
    let has_branch = branch.is_some();
    div().id(id).flex().items_center().gap_1().min_w_0().text_size(px(11.))
        .text_color(rgb(if selected { SELECTED_DETAIL } else { 0x969696 }))
        .tooltip(move |_, cx| cx.new(|_| crate::control_hint::ControlHint(hint.clone())).into())
        .when_some(branch, |row, branch| row.child(div().flex_shrink_0().child(crate::icons::icon("branch")))
            .child(div().min_w_0().max_w(relative(0.45)).truncate().child(branch)))
        .when_some(directory, |row, directory| row
            .when(has_branch, |row| row.child(div().flex_shrink_0().px_1().text_color(rgb(0x74717c)).child("·")))
            .child(div().flex_1().min_w_0().truncate().child(directory_label(&directory))))
}
