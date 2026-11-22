//! Shared application chrome colors. Terminal colors remain theme-controlled.
pub(crate) const ACCENT: u32 = 0xa99be8;
pub(crate) const SELECTED: u32 = 0x373441;
pub(crate) const SELECTED_HOVER: u32 = 0x403b4c;
pub(crate) const SELECTED_BORDER: u32 = 0x625a77;
pub(crate) const SELECTED_TEXT: u32 = 0xf1eff8;
pub(crate) const SELECTED_DETAIL: u32 = 0xcac5dd;

/// Keep a useful terminal width when an inspector is open. Narrow windows
/// overlay the inspector instead of resizing every PTY down to a few columns.
pub(crate) fn overlay_inspector(viewport_width: f32, sidebar_width: f32) -> bool {
    viewport_width - sidebar_width < 380. + 400.
}

pub(crate) fn inspector(overlay: bool) -> gpui::Div {
    use gpui::{prelude::*, *};
    div().w(px(380.)).max_w_full().h_full().flex_shrink_0().flex().flex_col()
        .bg(rgb(0x242424)).border_l_1().border_color(rgb(0x383838))
        .when(overlay, |panel| panel.absolute().right_0().top_0().occlude().shadow_lg())
}
