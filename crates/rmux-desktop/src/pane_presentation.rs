//! Native pane framing and control hit areas shared with the browser host.
use gpui::{prelude::*, *};

/// Resolve on hover from the target pane, including remapped or disabled keys.
pub(crate) fn action_control(id: impl Into<ElementId>, label: &'static str, action: Box<dyn Action>, focus: Option<FocusHandle>, mac: bool) -> Stateful<Div> {
    control_button(id).tooltip(move |window, cx| {
        let focus = focus.clone().or_else(|| window.focused(cx));
        let hint = focus.as_ref().and_then(|focus|
            crate::shortcut_label::binding_hint(action.as_ref(), focus, window, mac));
        let text = hint.map_or_else(|| label.to_owned(), |hint| format!("{label}  {hint}"));
        cx.new(|_| crate::control_hint::ControlHint(text.into())).into()
    })
}
pub(crate) fn frame(benchmark: bool, attention: bool) -> Div {
    div().relative().size_full().flex().flex_col().min_w_0().min_h_0()
        .when(!benchmark, |pane| pane.border_1().border_color(gpui::transparent_black()).when(attention, |pane| pane.border_color(rgb(0xf0b85b))))
}
pub(crate) fn control_button(id: impl Into<ElementId>) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .w(px(28.))
        .h(px(28.))
        .flex_shrink_0()
        .rounded_sm()
        .text_color(rgb(0xaaaaaa))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0x353535)).text_color(rgb(0xffffff)))

}

