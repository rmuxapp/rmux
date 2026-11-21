use super::*;
use std::{cell::Cell, rc::Rc};

pub(super) use crate::split_interaction::{SplitDrag, drag_capture};
impl crate::split_interaction::SplitHost for Workspace {
    type Pane = Entity<PaneView>;
    fn split_state(&mut self) -> (&mut Option<PaneTree<Self::Pane>>, &mut Option<SplitDrag>) {
        (&mut self.layout, &mut self.split_drag)
    }
}

pub(super) fn render_panes(
    tree: &PaneTree<Entity<PaneView>>,
    path: Vec<usize>,
    benchmark: bool,
    cx: &mut Context<Workspace>,
) -> AnyElement {
    match tree {
        PaneTree::Pane(view) => split_presentation::leaf(("pane", view.entity_id()))
            .child(view.clone())
            .into_any_element(),
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
                                bar.cursor(if axis == SplitAxis::Columns {
                                    CursorStyle::ResizeLeftRight
                                } else {
                                    CursorStyle::ResizeUpDown
                                })
                                .hover(|style| style.bg(rgb(0x40536b)))
                                .tooltip(|_, cx| cx.new(|_| pane_view::ControlHint(
                                    "Drag to resize · Double-click to equalize these two panes · Resize Pane commands are available in the palette".into()
                                )).into())
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                                        let bounds = bounds.get();
                                        let extent = match axis {
                                            SplitAxis::Columns => bounds.size.width,
                                            SplitAxis::Rows => bounds.size.height,
                                        };
                                        let available = (f32::from(extent)
                                            - divider_width * (count - 1) as f32)
                                            .max(1.);
                                        if event.click_count == 2 {
                                            if let Some(PaneTree::Split { weights, .. }) = this
                                                .layout
                                                .as_mut()
                                                .and_then(|tree| tree.at_mut(&path))
                                            {
                                                let equal =
                                                    (weights[index - 1] + weights[index]) / 2.;
                                                weights[index - 1] = equal;
                                                weights[index] = equal;
                                            }
                                            this.split_drag = None;
                                            cx.notify();
                                        } else {
                                            this.split_drag = Some(SplitDrag {
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
                        .child(render_panes(child, child_path, benchmark, cx))
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
