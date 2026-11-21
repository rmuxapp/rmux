//! Browser session host using native tab presentation and navigation definitions.
use gpui::{prelude::*, *};
use crate::{terminal_sample::TerminalSample, tab_presentation};
pub(crate) struct PaneEmpty;
impl EventEmitter<PaneEmpty> for SamplePane {}
pub(crate) struct SamplePane {
    terminals: rmux_core::tabs::Tabs<Entity<TerminalSample>>,
    labels: std::collections::HashMap<EntityId, String>,
    next_number: usize,
    tab_scroll: ScrollHandle,
}
pub(crate) struct TabTransfer {
    terminals: Vec<Entity<TerminalSample>>,
    labels: std::collections::HashMap<EntityId, String>,
    active: usize,
    next_number: usize,
}
impl SamplePane {
    pub fn take_active_for_split(&mut self, cx: &mut Context<Self>) -> Option<TabTransfer> {
        if self.terminals.len() < 2 { return None; }
        let terminal = self.terminals.remove(self.terminals.active_index());
        let id = terminal.entity_id();
        let label = self.labels.remove(&id).expect("tab label follows terminal ownership");
        self.tab_scroll.scroll_to_item(self.terminals.active_index());
        cx.notify();
        Some(TabTransfer { terminals: vec![terminal], labels: [(id, label)].into_iter().collect(),
            active: 0, next_number: self.next_number })
    }
    pub fn from_transfer(transfer: TabTransfer) -> Self {
        Self { terminals: rmux_core::tabs::Tabs::new(transfer.terminals, transfer.active),
            labels: transfer.labels, next_number: transfer.next_number, tab_scroll: ScrollHandle::new() }
    }
    pub fn tab_count(&self) -> usize { self.terminals.len() }
    pub fn take_all_tabs(&mut self, cx: &mut Context<Self>) -> TabTransfer {
        let active = self.terminals.active_index();
        let transfer = TabTransfer { terminals: self.terminals.take_all(),
            labels: std::mem::take(&mut self.labels), active, next_number: self.next_number };
        cx.notify();
        transfer
    }
    pub fn append_tabs(&mut self, transfer: TabTransfer, window: &mut Window, cx: &mut Context<Self>) {
        self.labels.extend(transfer.labels);
        self.next_number = self.next_number.max(transfer.next_number);
        if let Some(selected) = self.terminals.append(transfer.terminals, transfer.active) {
            self.select(selected, window, cx);
        }
    }

    pub fn scroll(&self, scroll: rmux_core::scroll::Scroll, cx: &mut App) {
        if let Some(terminal) = self.terminals.active() {
            terminal.update(cx, |terminal,cx| terminal.scroll(scroll,cx));
        }
    }
    pub fn release_render_caches(&mut self, cx: &mut Context<Self>) {
        for terminal in &self.terminals {
            terminal.update(cx, |terminal,cx| terminal.release_render_cache(cx));
        }
    }
    fn close(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.terminals.len() { return; }
        let replace_focus = self.terminals[index].read(cx).is_focused(window);
        let removed = self.terminals.remove(index);
        self.labels.remove(&removed.entity_id());
        if self.terminals.is_empty() { cx.emit(PaneEmpty); }
        else {
            self.tab_scroll.scroll_to_item(self.terminals.active_index());
            if replace_focus { self.focus(window, cx); }
        }
        cx.notify();
    }
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let terminal = cx.new(|cx| TerminalSample::new(window,cx));
        let labels = [(terminal.entity_id(), "Terminal 1".into())].into_iter().collect();
        Self { terminals: rmux_core::tabs::Tabs::new(vec![terminal], 0), labels, next_number: 2, tab_scroll: ScrollHandle::new() }
    }
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        if let Some(terminal) = self.terminals.active() {
            terminal.update(cx, |terminal, cx| terminal.focus(window, cx));
        }
    }
    fn select(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.terminals.len() { return; }
        if self.terminals.active_index() != index {
            self.terminals[self.terminals.active_index()].update(cx, |terminal, cx| terminal.release_render_cache(cx));
        }
        self.terminals.select(index);
        self.tab_scroll.scroll_to_item(index);
        self.focus(window, cx);
        cx.notify();
    }
    fn add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.terminals.len() >= 16 { return; }
        let terminal = cx.new(|cx| TerminalSample::new(window,cx));
        self.labels.insert(terminal.entity_id(), format!("Terminal {}",self.next_number));
        self.next_number += 1;
        self.terminals.push(terminal);
        self.select(self.terminals.len()-1, window, cx);
    }
    pub fn reorder(&mut self, delta: isize, cx: &mut Context<Self>) {
        if let Some((_, next)) = self.terminals.move_active(delta) {
            self.tab_scroll.scroll_to_item(next);
            cx.notify();
        }
    }
    fn cycle(&mut self, direction: isize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(next) = self.terminals.cycle_target(direction) {
            self.select(next, window, cx);
        }
    }
}
impl crate::tab_actions::TabCommandHost for SamplePane {
    fn run_tab_command(&mut self, command: rmux_core::commands::CommandId, window: &mut Window, cx: &mut Context<Self>) {
        use rmux_core::commands::CommandId;
        match command {
            CommandId::NewTab => self.add(window, cx),
            CommandId::NextTab => self.cycle(1, window, cx),
            CommandId::PreviousTab => self.cycle(-1, window, cx),
            CommandId::MoveTabLeft => self.reorder(-1, cx),
            CommandId::MoveTabRight => self.reorder(1, cx),
            CommandId::ClosePane => self.close(self.terminals.active_index(), window, cx),
            _ => {}
        }
    }
}
impl Render for SamplePane {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.terminals.active().is_some_and(|terminal| terminal.read(cx).is_focused(window));
        crate::pane_presentation::frame(false, false)
            .map(|root| crate::tab_actions::bind(root, cx))
            .child(tab_presentation::strip()
                .child(div().id("sample-tabs").flex().flex_1().h_full().min_w_0().overflow_x_scroll().track_scroll(&self.tab_scroll)
                    .children(self.terminals.iter().enumerate().map(|(index, terminal)|
                        tab_presentation::tab(("sample-tab",terminal.entity_id()),index==self.terminals.active_index(),focused)
                            .child(self.labels[&terminal.entity_id()].clone())
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
