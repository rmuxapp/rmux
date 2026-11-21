//! Browser session host using native tab presentation and navigation definitions.
use gpui::{prelude::*, *};
use crate::{terminal_sample::TerminalSample, tab_presentation, NewTab, NextTab, PreviousTab};
pub(crate) struct SamplePane {
    terminals: Vec<Entity<TerminalSample>>,
    active: usize,
}
impl SamplePane {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self { terminals: vec![cx.new(TerminalSample::new)], active: 0 }
    }
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.terminals[self.active].update(cx, |terminal, cx| terminal.focus(window, cx));
    }
    fn select(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.terminals.len() { return; }
        if self.active != index {
            self.terminals[self.active].update(cx, |terminal, _| terminal.release_render_cache());
        }
        self.active = index;
        self.focus(window, cx);
        cx.notify();
    }
    fn add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.terminals.len() >= 16 { return; }
        self.terminals.push(cx.new(TerminalSample::new));
        self.select(self.terminals.len()-1, window, cx);
    }
    fn cycle(&mut self, direction: isize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(next) = rmux_core::layout::cycle_index(self.active, self.terminals.len(), direction) {
            self.select(next, window, cx);
        }
    }
}
impl Render for SamplePane {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.terminals[self.active].read(cx).is_focused(window);
        div().size_full().flex().flex_col().min_w_0().min_h_0()
            .on_action(cx.listener(|this, _: &NewTab, window, cx| this.add(window,cx)))
            .on_action(cx.listener(|this, _: &NextTab, window, cx| this.cycle(1,window,cx)))
            .on_action(cx.listener(|this, _: &PreviousTab, window, cx| this.cycle(-1,window,cx)))
            .child(tab_presentation::strip()
                .child(div().id("sample-tabs").flex().flex_1().h_full().min_w_0().overflow_x_scroll()
                    .children(self.terminals.iter().enumerate().map(|(index, terminal)|
                        tab_presentation::tab(("sample-tab",terminal.entity_id()),index==self.active,focused)
                            .child(format!("shell {}",index+1))
                            .on_click(cx.listener(move |this,_,window,cx| this.select(index,window,cx))))))
                .child(div().id("add-tab").px_2().cursor_pointer().child("+")
                    .on_click(cx.listener(|this,_,window,cx| this.add(window,cx)))))
            .child(div().flex_1().min_h_0().child(self.terminals[self.active].clone()))
    }
}
