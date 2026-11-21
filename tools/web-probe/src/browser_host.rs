//! Browser service host; workspace presentation comes from native sources.
use gpui::{prelude::*, *};
use crate::{InputProbe, sidebar_presentation, NewWorkspace, NextWorkspace, PreviousWorkspace, SelectWorkspaceNumber};
pub(crate) struct BrowserHost {
    workspaces: Vec<(String, Entity<InputProbe>)>,
    active: usize,
}
impl BrowserHost {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let workspaces = ["rmux", "website"].into_iter().map(|name|
            (name.to_owned(), cx.new(|cx| InputProbe::new(window, cx)))).collect();
        let mut host = Self { workspaces, active: 0 };
        host.select(0,window,cx);
        host
    }
    fn select(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.workspaces.len() { return; }
        self.workspaces[self.active].1.update(cx, |workspace,cx| {
            for pane in workspace.samples.values() {
                pane.update(cx, |pane,cx| pane.release_render_caches(cx));
            }
        });
        self.active=index;
        self.workspaces[index].1.update(cx, |workspace,cx| workspace.close_palette(window,cx));
        cx.notify();
    }
    fn cycle(&mut self, direction: isize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = rmux_core::layout::cycle_index(self.active, self.workspaces.len(), direction) {
            self.select(index, window, cx);
        }
    }
    fn add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.workspaces.len() >= 16 { return; }
        let name=format!("Workspace {}",self.workspaces.len()+1);
        self.workspaces.push((name,cx.new(|cx| InputProbe::new(window,cx))));
        self.select(self.workspaces.len()-1,window,cx);
    }
}
impl Render for BrowserHost {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().flex().font_family("Inter").text_color(rgb(0xe2e3e7))
            .on_action(cx.listener(|this, _: &NewWorkspace, window, cx| this.add(window, cx)))
            .on_action(cx.listener(|this, _: &NextWorkspace, window, cx| this.cycle(1, window, cx)))
            .on_action(cx.listener(|this, _: &PreviousWorkspace, window, cx| this.cycle(-1, window, cx)))
            .on_action(cx.listener(|this, action: &SelectWorkspaceNumber, window, cx| {
                let index = match action.number {
                    1..=8 => action.number - 1,
                    9 => this.workspaces.len().saturating_sub(1),
                    _ => return,
                };
                this.select(index, window, cx);
            }))
            .child(sidebar_presentation::sidebar(220.)
                .child(sidebar_presentation::header()
                    .child(div().flex_1().text_xs().text_color(rgb(0xa0a0a0)).child("Workspaces"))
                    .child(div().id("new-workspace").px_2().cursor_pointer().child("+")
                        .on_click(cx.listener(|this,_,window,cx| this.add(window,cx)))))
                .child(div().id("workspace-list").flex_1().min_h_0().overflow_y_scroll().py_2()
                    .children(self.workspaces.iter().enumerate().map(|(index,(name,entity))|
                        sidebar_presentation::row(("workspace",entity.entity_id()),index==self.active)
                            .child(name.clone())
                            .on_click(cx.listener(move |this,_,window,cx| this.select(index,window,cx)))))))
            .child(div().flex_1().min_w_0().h_full().child(self.workspaces[self.active].1.clone()))
    }
}
