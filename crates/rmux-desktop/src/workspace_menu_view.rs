//! Workspace menu content and geometry shared by native and browser hosts.
use gpui::{prelude::*, *};
use rmux_core::commands::CommandId;
pub(crate) fn labels(group: bool, pinned: bool) -> [&'static str; 6] {
    if group { ["Rename Group", "Change Group Color", "Change Group Icon", if pinned { "Unpin Group" } else { "Pin Group" }, "Ungroup Workspaces", "Close Group…"] }
    else { ["Rename Workspace", "Edit Description", "Change Color", "Change Icon", if pinned { "Unpin Workspace" } else { "Pin Workspace" }, "Close Workspace…"] }
}
pub(crate) fn commands(group: bool) -> [CommandId; 6] {
    use CommandId::*;
    if group { [RenameWorkspaceGroup, EditGroupColor, EditGroupIcon, ToggleGroupPin, UngroupWorkspaces, CloseWorkspaceGroup] }
    else { [RenameWorkspace, EditWorkspaceDescription, EditWorkspaceColor, EditWorkspaceIcon, ToggleWorkspacePin, CloseWorkspace] }
}
pub(crate) fn frame(window: &Window) -> Stateful<Div> {
    div().id("workspace-context-menu")
        .w(px(340.).min((window.viewport_size().width - px(16.)).max(px(1.))))
        .py_1().bg(rgb(0x292929)).border_1().border_color(rgb(0x465063)).rounded_md().shadow_md().occlude()
}
pub(crate) fn target_header(name: SharedString, kind: &'static str) -> Div {
    let hint = name.clone();
    div().h(px(48.)).flex_shrink_0().px_3().py_1().flex().flex_col().justify_center().gap_1()
        .border_b_1().border_color(rgb(0x414141))
        .child(div().text_xs().text_color(rgb(0x96969e)).child(kind))
        .child(div().id("context-menu-target").text_size(px(13.)).font_weight(FontWeight::SEMIBOLD).truncate()
            .tooltip(move |_, cx| cx.new(|_| crate::control_hint::ControlHint(hint.clone())).into())
            .child(name))
}
pub(crate) fn row(index: usize, selected: usize) -> Stateful<Div> {
    div().id(("workspace-menu-item", index)).mx_1().px_3().py_2().rounded_sm().text_size(px(13.)).cursor_pointer().flex().items_center().gap_3()
        .when(index == 4, |row| row.mt_1().border_t_1().border_color(rgb(0x414141)))
        .when(index == 5, |row| row.text_color(rgb(0xffa4a4)))
        .when(index == selected, |row| row.bg(rgb(crate::chrome_style::SELECTED)))
}
