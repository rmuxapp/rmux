//! Workspace toolbar geometry shared by native and browser hosts.
use gpui::{prelude::*, *};
pub(crate) fn toolbar() -> Div {
    div().h(px(36.)).flex_shrink_0().px_3().flex().items_center()
        .bg(crate::chrome_style::surface()).border_b_1().border_color(crate::chrome_style::border()).gap_2()
}
pub(crate) fn title(id: impl Into<ElementId>, name: impl Into<SharedString>) -> Stateful<Div> {
    div().id(id).flex_1().min_w_0().truncate().text_size(px(13.))
        .font_weight(FontWeight::SEMIBOLD).child(name.into())
}

pub(crate) fn sidebar_toggle(mac: bool) -> Stateful<Div> {
    crate::pane_presentation::action_control("toolbar-sidebar", "Toggle Sidebar", Box::new(crate::ToggleSidebar), None, mac)
        .on_click(|event, window, cx| {
            if event.standard_click() { window.dispatch_action(Box::new(crate::ToggleSidebar), cx); }
        })
        .child(crate::icons::icon("sidebar"))
}
