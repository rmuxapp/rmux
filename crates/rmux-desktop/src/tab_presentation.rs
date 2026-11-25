//! Tab strip appearance shared by native and browser hosts.
use gpui::{prelude::*, *};
pub(crate) fn menu_frame(window: &Window) -> Stateful<Div> {
    div().id("tab-context-menu")
        .w(px(340.).min((window.viewport_size().width - px(16.)).max(px(1.))))
        .py_1().bg(crate::chrome_style::elevated()).border_1().border_color(crate::chrome_style::border())
        .rounded_md().shadow_md().occlude()
}
pub(crate) fn menu_row(index: usize, section: bool) -> Stateful<Div> {
    div().id(("tab-menu-item", index)).mx_1().px_3().py_2().rounded_sm()
        .text_size(px(13.)).line_height(px(20.)).flex().items_center().gap_3()
        .when(section, |row| row.mt_2().relative().child(crate::workspace_menu_view::separator()))
}
pub(crate) fn strip() -> Div {
    div().flex().w_full().min_w_0().max_w_full().overflow_hidden()
        .h(px(30.)).flex_shrink_0().items_center()
        .border_b_1().border_color(crate::chrome_style::border()).bg(crate::chrome_style::surface())
}
pub(crate) fn close_button(id: impl Into<ElementId>) -> Stateful<Div> {
    div().id(id).flex().items_center().justify_center().w(px(22.)).h(px(22.))
        .flex_shrink_0().rounded_sm().cursor_pointer()
        .tooltip(|_, cx| cx.new(|_| crate::control_hint::ControlHint("Close Terminal Tab".into())).into())
        .hover(|style| style.bg(crate::chrome_style::hover_strong()).text_color(crate::chrome_style::foreground()))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(crate::icons::icon("close"))
}
pub(crate) fn tab(id: impl Into<ElementId>, active: bool, focused: bool, color: Option<u32>, count: usize) -> Stateful<Div> {
    div().id(id).h_full().w(relative(1. / count.clamp(1, 3) as f32)).max_w(px(220.)).min_w_0()
        .flex().items_center().flex_shrink_0().px_2().gap_1()
        .text_size(px(12.)).border_b_2().border_color(gpui::transparent_black())
        .cursor_pointer().text_color(crate::chrome_style::muted())
        .hover(move |style| style.bg(color.map(|color| crate::chrome_style::custom_fill(color, active, true)).unwrap_or_else(crate::chrome_style::hover)))
        .when(active, |tab| tab.bg(if focused {crate::chrome_style::hover()} else {crate::chrome_style::elevated()})
            .text_color(if focused {crate::chrome_style::foreground()} else {crate::chrome_style::muted()})
            .border_color(if focused {crate::chrome_style::accent()} else {crate::chrome_style::border()}))
        .when_some(color, |tab, color| tab.bg(crate::chrome_style::custom_fill(color, active, false))
            .when(active, |tab| tab.border_color(crate::chrome_style::custom_border(color))))
}

pub(crate) fn rename_editor(input: Entity<crate::text_field::TextField>, cx: &App) -> Div {
    let count = rmux_core::tabs::custom_title_length(input.read(cx).value());
    let too_long = count > rmux_core::tabs::CUSTOM_TITLE_LIMIT;
    let save = input.clone();
    let cancel = input.clone();
    div().flex().flex_col().gap_2().px_3().py_2().bg(crate::chrome_style::elevated()).min_w_0()
        .border_b_1().border_color(crate::chrome_style::border())
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(div().flex().items_center().justify_between().gap_2().text_xs()
            .child(div().text_color(crate::chrome_style::muted()).child("Rename tab"))
            .child(div().text_color(if too_long { crate::chrome_style::danger() } else { crate::chrome_style::muted() })
                .child(format!("{count}/{}", rmux_core::tabs::CUSTOM_TITLE_LIMIT))))
        .child(div().flex().flex_wrap().items_center().gap_2().min_w_0()
            .child(div().flex_1().min_w(px(80.)).max_w_full().px_2().py_1().rounded_sm()
                .border_1().border_color(crate::chrome_style::selected_border()).bg(crate::chrome_style::background()).child(input))
            .child(div().id("cancel-tab-rename").px_2().py_1().rounded_sm().text_xs().cursor_pointer()
                .hover(|style| style.bg(crate::chrome_style::border()))
                .on_click(move |event, _, cx| {
                    cx.stop_propagation();
                    if event.standard_click() { cancel.update(cx, |input, cx| input.cancel(cx)); }
                }).child("Cancel"))
            .child(div().id("save-tab-rename").when(too_long, |button| button.opacity(0.45)).px_2().py_1().rounded_sm().text_xs().cursor_pointer()
                .bg(crate::chrome_style::selected()).border_1().border_color(crate::chrome_style::selected_border())
                .hover(|style| style.bg(crate::chrome_style::selected_hover()))
                .on_click(move |event, _, cx| {
                    cx.stop_propagation();
                    if event.standard_click() && !too_long { save.update(cx, |input, cx| input.submit(false, cx)); }
                }).child("Save")))
        .child(div().text_xs().text_color(crate::chrome_style::muted()).child(if too_long { "Name is too long. Shorten it to 128 characters to save." } else { "Leave empty for automatic title · Enter saves · Esc cancels" }))
}

pub(crate) fn title_frame(id: impl Into<ElementId>) -> Stateful<Div> {
    div().id(id).flex_1().min_w_0().max_w(px(220.)).truncate()
}
// Browser preview uses plain titles; native tabs compose their own title content.
#[cfg(target_family = "wasm")]
pub(crate) fn title(id: impl Into<ElementId>, title: SharedString, hint: SharedString) -> Stateful<Div> {
    title_frame(id)
        .tooltip(move |_, cx| cx.new(|_| crate::control_hint::ControlHint(hint.clone())).into())
        .child(title)
}
