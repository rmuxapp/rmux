use gpui::{prelude::*, *};
use rmux_core::layout::{PaneTree, Axis};

pub(crate) struct SplitDrag {
    pub path: Vec<usize>,
    pub boundary: usize,
    pub axis: Axis,
    pub origin: Point<Pixels>,
    pub units_per_pixel: f32,
    pub left: f32,
    pub right: f32,
}

pub(crate) trait SplitHost: Sized + 'static {
    type Pane: Clone + PartialEq;
    fn split_state(&mut self) -> (&mut Option<PaneTree<Self::Pane>>, &mut Option<SplitDrag>);
    fn drag_split(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        let (tree, state) = self.split_state();
        if !event.dragging() { *state = None; return; }
        let Some(drag) = state.as_ref() else { return; };
        let delta = match drag.axis {
            Axis::Columns => event.position.x - drag.origin.x,
            Axis::Rows => event.position.y - drag.origin.y,
        };
        let (left, right) = rmux_core::layout::resized_pair(drag.left, drag.right,
            f32::from(delta) * drag.units_per_pixel, 48. * drag.units_per_pixel);
        if let Some(PaneTree::Split { weights, .. }) = tree.as_mut().and_then(|tree| tree.at_mut(&drag.path)) {
            if drag.boundary + 1 < weights.len()
                && (weights[drag.boundary] != left || weights[drag.boundary + 1] != right) {
                weights[drag.boundary] = left;
                weights[drag.boundary + 1] = right;
                cx.notify();
            }
        }
        cx.stop_propagation();
    }
}

/// Register drag capture before child terminal handlers. Drag state contains no
/// terminal references and is discarded when the layout changes.
pub(crate) fn drag_capture<H: SplitHost>(cx: &Context<H>) -> impl IntoElement {
    let entity = cx.weak_entity();
    canvas(
        |_, _, _| (),
        move |_, _, window, _| {
            let moving = entity.clone();
            window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                if phase == DispatchPhase::Capture {
                    let _ = moving.update(cx, |this, cx| this.drag_split(event, cx));
                }
            });
            let released = entity.clone();
            window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                if phase == DispatchPhase::Capture && event.button == MouseButton::Left {
                    let _ = released.update(cx, |this, cx| {
                        if this.split_state().1.take().is_some() {
                            cx.stop_propagation();
                        }
                    });
                }
            });
        },
    )
    .absolute()
    .size_full()
}

