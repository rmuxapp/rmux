//! Tab strip appearance shared by native and browser hosts.
use gpui::{prelude::*, *};
pub(crate) fn menu_frame(window: &Window) -> Stateful<Div> {
    div().id("tab-context-menu")
        .w(px(340.).min((window.viewport_size().width - px(16.)).max(px(1.))))
        .py_1().bg(rgb(0x292929)).border_1().border_color(rgb(0x465063))
        .rounded_md().shadow_md().occlude()
}
pub(crate) fn menu_row(index: usize, section: bool) -> Stateful<Div> {
    div().id(("tab-menu-item", index)).mx_1().px_3().py_2().rounded_sm()
        .text_size(px(13.)).flex().items_center().gap_3()
        .when(section, |row| row.mt_1().border_t_1().border_color(rgb(0x414141)))
}
pub(crate) fn strip() -> Div {
    div().flex().h(px(30.)).flex_shrink_0().items_center()
        .border_b_1().border_color(rgb(0x383838)).bg(rgb(0x242424))
}
pub(crate) fn tab(id: impl Into<ElementId>, active: bool, focused: bool) -> Stateful<Div> {
    div().id(id).flex().items_center().flex_shrink_0().px_2().gap_2()
        .text_size(px(12.)).border_t_1().border_l_2().border_color(gpui::transparent_black())
        .cursor_pointer().text_color(rgb(0x9b9b9b))
        .when(active, |tab| tab.bg(rgb(if focused {0x303030} else {0x292929}))
            .text_color(rgb(if focused {0xe5e5e5} else {0xb8bfcb}))
            .border_color(rgb(if focused {0x4599ef} else {0x4a5362})))
}

pub(crate) fn rename_editor(input: Entity<crate::text_field::TextField>) -> Div {
    div().flex().flex_col().gap_1().px_3().py_2().bg(rgb(0x292929))
        .child(div().text_xs().text_color(rgb(0xaaaaaa)).child("Tab name · empty restores automatic title · Enter to save · Esc to cancel"))
        .child(input)
}

pub(crate) fn title(id: impl Into<ElementId>, title: SharedString, hint: SharedString) -> Stateful<Div> {
    div().id(id).min_w(px(48.)).max_w(px(220.)).truncate()
        .tooltip(move |_, cx| cx.new(|_| crate::control_hint::ControlHint(hint.clone())).into())
        .child(title)
}
