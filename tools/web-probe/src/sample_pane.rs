//! Browser session host using native tab presentation and navigation definitions.
use gpui::{prelude::*, *};
use crate::{terminal_sample::TerminalSample, tab_presentation, NewTab, NextTab, PreviousTab, ClosePane};
pub(crate) struct PaneEmpty;
impl EventEmitter<PaneEmpty> for SamplePane {}
pub(crate) struct SamplePane {
    terminals: rmux_core::tabs::Tabs<Entity<TerminalSample>>,
}
impl SamplePane {
    pub fn scroll(&self, scroll: rmux_core::scroll::Scroll, cx: &mut App) {
        if let Some(terminal) = self.terminals.active() {
            terminal.update(cx, |terminal,cx| terminal.scroll(scroll,cx));
        }
    }
    pub fn release_render_caches(&mut self, cx: &mut Context<Self>) {
        for terminal in &self.terminals {
            terminal.update(cx, |terminal,_| terminal.release_render_cache());
        }
    }
    fn close(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.terminals.len() { return; }
        let replace_focus = self.terminals[index].read(cx).is_focused(window);
        self.terminals.remove(index);
        if self.terminals.is_empty() { cx.emit(PaneEmpty); }
        else {
            if replace_focus { self.focus(window, cx); }
        }
        cx.notify();
    }
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self { terminals: rmux_core::tabs::Tabs::new(vec![cx.new(TerminalSample::new)], 0) }
    }
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        if let Some(terminal) = self.terminals.active() {
            terminal.update(cx, |terminal, cx| terminal.focus(window, cx));
        }
    }
    fn select(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.terminals.len() { return; }
        if self.terminals.active_index() != index {
            self.terminals[self.terminals.active_index()].update(cx, |terminal, _| terminal.release_render_cache());
        }
        self.terminals.select(index);
        self.focus(window, cx);
        cx.notify();
    }
    fn add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.terminals.len() >= 16 { return; }
        self.terminals.push(cx.new(TerminalSample::new));
        self.select(self.terminals.len()-1, window, cx);
    }
    fn cycle(&mut self, direction: isize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(next) = self.terminals.cycle_target(direction) {
            self.select(next, window, cx);
        }
    }
}
impl Render for SamplePane {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.terminals.active().is_some_and(|terminal| terminal.read(cx).is_focused(window));
        crate::pane_presentation::frame(false, false)
            .on_action(cx.listener(|this, _: &NewTab, window, cx| this.add(window,cx)))
            .on_action(cx.listener(|this, _: &NextTab, window, cx| this.cycle(1,window,cx)))
            .on_action(cx.listener(|this, _: &PreviousTab, window, cx| this.cycle(-1,window,cx)))
            .on_action(cx.listener(|this, _: &ClosePane, window, cx| this.close(this.terminals.active_index(),window,cx)))
            .child(tab_presentation::strip()
                .child(div().id("sample-tabs").flex().flex_1().h_full().min_w_0().overflow_x_scroll()
                    .children(self.terminals.iter().enumerate().map(|(index, terminal)|
                        tab_presentation::tab(("sample-tab",terminal.entity_id()),index==self.terminals.active_index(),focused)
                            .child(format!("shell {}",index+1))
                            .child(div().id(("close-sample-tab",terminal.entity_id())).px_1().child("×")
                                .on_mouse_down(MouseButton::Left, |_,_,cx| cx.stop_propagation())
                                .on_click(cx.listener(move |this,_,window,cx| { cx.stop_propagation(); this.close(index,window,cx); })))
                            .on_mouse_down(MouseButton::Middle, cx.listener(move |this,_,window,cx| { cx.stop_propagation(); this.close(index,window,cx); }))
                            .on_click(cx.listener(move |this,_,window,cx| this.select(index,window,cx))))))
                .child(crate::pane_presentation::control_button("add-tab").child(crate::icons::icon("plus"))
                    .on_click(cx.listener(|this,_,window,cx| this.add(window,cx))))
                .children([
                    ("split-right", "Split Right", Box::new(crate::Split) as Box<dyn Action>),
                    ("split-down", "Split Below", Box::new(crate::SplitBelow) as Box<dyn Action>),
                    ("expand", "Toggle Pane Zoom", Box::new(crate::TogglePaneZoom) as Box<dyn Action>),
                ].into_iter().map(|(icon,label,action)| {
                    crate::pane_presentation::control_button(icon).child(crate::icons::icon(icon))
                        .tooltip(move |_,cx| cx.new(|_| crate::control_hint::ControlHint(label.into())).into())
                        .on_click(cx.listener(move |this,event: &ClickEvent,window,cx| {
                            if !event.standard_click() { return; }
                            this.focus(window,cx);
                            window.dispatch_action(action.boxed_clone(),cx);
                        }))
                })))
            .child(div().flex_1().min_h_0().children(self.terminals.active().cloned()))
    }
}
