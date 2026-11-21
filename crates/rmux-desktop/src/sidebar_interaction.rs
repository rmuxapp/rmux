//! Shared sidebar resizing and pointer capture.
use gpui::{prelude::*, *};
pub(crate) const DEFAULT_WIDTH: f32 = 200.;
pub(crate) trait SidebarHost: Sized + 'static {
    fn sidebar_state(&mut self) -> (&mut f32, &mut Option<(Pixels, f32)>);
}
pub(crate) fn visible_width(width: f32, window: &Window) -> f32 {
    width.min((f32::from(window.viewport_size().width) - 165.).max(0.))
}
pub(crate) fn drag_capture<H: SidebarHost>(cx: &Context<H>) -> impl IntoElement {
    let entity = cx.weak_entity();
    canvas(|_, _, _| (), move |_, _, window, _| {
        let moving = entity.clone();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
            if phase != DispatchPhase::Capture { return; }
            let _ = moving.update(cx, |this, cx| {
                let (width, drag) = this.sidebar_state();
                let Some((origin, initial)) = *drag else { return; };
                if !event.dragging() { *drag = None; return; }
                let maximum = (f32::from(window.viewport_size().width) - 165.).clamp(160., 480.);
                let next = (initial + f32::from(event.position.x - origin)).clamp(160., maximum);
                if *width != next { *width = next; cx.notify(); }
                cx.stop_propagation();
            });
        });
        let released = entity.clone();
        window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
            if phase == DispatchPhase::Capture && event.button == MouseButton::Left {
                let _ = released.update(cx, |this, cx| {
                    if this.sidebar_state().1.take().is_some() { cx.stop_propagation(); }
                });
            }
        });
    }).absolute().size_full()
}
pub(crate) fn divider<H: SidebarHost>(width: f32, cx: &Context<H>) -> Stateful<Div> {
    div().id("sidebar-divider").w(px(5.)).h_full().flex_shrink_0()
        .bg(rgb(0x242424)).border_l_1().border_color(rgb(0x383838)).cursor(CursorStyle::ResizeLeftRight)
        .hover(|style| style.bg(rgb(0x7795c8)))
        .on_mouse_down(MouseButton::Left, cx.listener(move |this, event: &MouseDownEvent, _, cx| {
            let (stored, drag) = this.sidebar_state();
            if event.click_count == 2 { *stored = DEFAULT_WIDTH; *drag = None; cx.notify(); }
            else { *drag = Some((event.position.x, width)); }
            cx.stop_propagation();
        }))
}
