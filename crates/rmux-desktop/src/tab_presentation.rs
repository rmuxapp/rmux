//! Tab strip appearance shared by native and browser hosts.
use gpui::{prelude::*, *};
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
