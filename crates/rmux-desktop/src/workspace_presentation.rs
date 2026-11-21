//! Workspace toolbar geometry shared by native and browser hosts.
use gpui::{prelude::*, *};
pub(crate) fn toolbar() -> Div {
    div().h(px(36.)).flex_shrink_0().px_3().flex().items_center()
        .bg(rgb(0x242424)).border_b_1().border_color(rgb(0x383838)).gap_2()
}
pub(crate) fn title(id: impl Into<ElementId>, name: impl Into<SharedString>) -> Stateful<Div> {
    div().id(id).flex_1().min_w_0().truncate().text_size(px(13.))
        .font_weight(FontWeight::SEMIBOLD).child(name.into())
}
