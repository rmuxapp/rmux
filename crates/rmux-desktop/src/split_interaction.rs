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
                            // Commit a final non-dragging scene even when no
                            // further pointer movement follows the release.
                            cx.notify();
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


use std::{cell::Cell, rc::Rc};
use crate::split_presentation;

pub(crate) fn render_panes<H: SplitHost>(
    tree: &PaneTree<H::Pane>,
    path: Vec<usize>,
    benchmark: bool,
    cx: &mut Context<H>,
    leaf: &impl Fn(&H::Pane, &mut Context<H>) -> AnyElement,
) -> AnyElement {
    match tree {
        PaneTree::Pane(view) => leaf(view, cx),
        PaneTree::Split {
            axis,
            children,
            weights,
        } => {
            let axis = *axis;
            let bounds = Rc::new(Cell::new(Bounds::<Pixels>::default()));
            let measured = bounds.clone();
            let divider_width = if benchmark { 1. } else { 5. };
            let mut elements = Vec::new();
            for (index, child) in children.iter().enumerate() {
                if index > 0 {
                    let bounds = bounds.clone();
                    let path = path.clone();
                    let weights = weights.clone();
                    let count = children.len();
                    elements.push(
                        split_presentation::divider(
                            SharedString::from(format!("divider-{path:?}-{index}")), axis, divider_width)
                            .when(!benchmark, |bar| {
                                bar.cursor(if axis == Axis::Columns {
                                    CursorStyle::ResizeLeftRight
                                } else {
                                    CursorStyle::ResizeUpDown
                                })
                                .hover(|style| style.bg(crate::chrome_style::selected()))
                                .tooltip(|_, cx| cx.new(|_| crate::control_hint::ControlHint(
                                    "Drag to resize · Double-click to equalize these two panes".into()
                                )).into())
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                                        let bounds = bounds.get();
                                        let extent = match axis {
                                            Axis::Columns => bounds.size.width,
                                            Axis::Rows => bounds.size.height,
                                        };
                                        let available = (f32::from(extent)
                                            - divider_width * (count - 1) as f32)
                                            .max(1.);
                                        let (tree, drag) = this.split_state();
                                        if event.click_count == 2 {
                                            if let Some(PaneTree::Split { weights, .. }) = tree.as_mut()
                                                .and_then(|tree| tree.at_mut(&path))
                                            {
                                                let equal =
                                                    (weights[index - 1] + weights[index]) / 2.;
                                                weights[index - 1] = equal;
                                                weights[index] = equal;
                                            }
                                            *drag = None;
                                            cx.notify();
                                        } else {
                                            *drag = Some(SplitDrag {
                                                path: path.clone(),
                                                boundary: index - 1,
                                                axis,
                                                origin: event.position,
                                                units_per_pixel: weights.iter().sum::<f32>()
                                                    / available,
                                                left: weights[index - 1],
                                                right: weights[index],
                                            });
                                        }
                                        cx.stop_propagation();
                                    }),
                                )
                            })
                            .into_any_element(),
                    );
                }
                let mut child_path = path.clone();
                child_path.push(index);
                elements.push(
                    split_presentation::weighted(weights[index])
                        .child(render_panes(child, child_path, benchmark, cx, leaf))
                        .into_any_element(),
                );
            }
            split_presentation::branch(axis)
                .child(
                    canvas(move |bounds, _, _| measured.set(bounds), |_, _, _, _| {})
                        .absolute()
                        .size_full(),
                )
                .children(elements)
                .into_any_element()
        }
    }
}
