use super::*;

pub(super) use crate::split_interaction::{SplitDrag, drag_capture};
impl crate::split_interaction::SplitHost for Workspace {
    type Pane = Entity<PaneView>;
    fn split_state(&mut self) -> (&mut Option<PaneTree<Self::Pane>>, &mut Option<SplitDrag>) {
        (&mut self.layout, &mut self.split_drag)
    }
}

pub(super) fn render_panes(tree: &PaneTree<Entity<PaneView>>, path: Vec<usize>, benchmark: bool, cx: &mut Context<Workspace>) -> AnyElement {
    crate::split_interaction::render_panes(tree, path, benchmark, cx, &|view, _| {
        split_presentation::leaf(("pane", view.entity_id())).child(view.clone()).into_any_element()
    })
}
