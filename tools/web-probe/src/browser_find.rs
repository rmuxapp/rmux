//! Browser search scheduling; controls and text editing are shared with desktop.
use gpui::{prelude::*, *};
use crate::{terminal_sample::{TerminalSample, SearchChanged}, terminal_engine::SearchProgress,
    text_field::{TextField, EditEvent}, find_presentation::FindHost};
pub(crate) struct Closed;
pub(crate) struct FindBar {
    input: Entity<TextField>,
    history: rmux_core::search_state::QueryHistory,
    parent: WeakEntity<TerminalSample>,
    status: String,
    error: bool,
    pending: bool,
    navigation: std::collections::VecDeque<bool>,
    task: Option<Task<()>>,
}
impl EventEmitter<Closed> for FindBar {}
impl FindBar {
    pub fn new(parent: Entity<TerminalSample>, history: rmux_core::search_state::QueryHistory, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| TextField::new("", cx));
        input.read(cx).focus.clone().focus(window, cx);
        cx.subscribe(&input, |this, _, event: &EditEvent, cx| match event {
            EditEvent::Changed(query) => {
                this.history.edited(query);
                this.task = None;
                this.navigation.clear();
                this.error = false;
                let result = this.parent.read_with(cx, |terminal, _| terminal.engine.search_query(query));
                this.accept(result.unwrap_or_else(|_| Err("Terminal closed".into())), cx);
            }
            EditEvent::Submitted { reverse, .. } => this.navigate_find(!reverse, cx),
            EditEvent::Cancelled => cx.emit(Closed),
        }).detach();
        cx.subscribe(&parent, |this, _, _: &SearchChanged, cx| {
            if this.input.read(cx).value().is_empty() { return; }
            let result = this.parent.read_with(cx, |terminal, _| terminal.engine.search_refresh());
            this.accept(result.unwrap_or_else(|_| Err("Terminal closed".into())), cx);
        }).detach();
        Self { input, history, parent: parent.downgrade(), status: "Find in terminal".into(), error: false,
            pending: false, navigation: Default::default(), task: None }
    }
    pub fn focus_query(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.input.update(cx, |input, cx| input.select_all_text(cx));
        self.input.read(cx).focus.clone().focus(window, cx);
    }
    pub fn saved_history(&self, cx: &App) -> rmux_core::search_state::QueryHistory {
        let mut history = self.history.clone();
        history.remember(self.input.read(cx).value());
        history
    }
    fn history_key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke;
        if !key.modifiers.alt || key.modifiers.control || key.modifiers.platform || key.modifiers.shift
            || !matches!(key.key.as_str(), "up" | "down") || self.input.read(cx).is_composing() { return; }
        if let Some(query) = self.history.recall(key.key == "up", self.input.read(cx).value()) {
            self.input.update(cx, |input, cx| {
                input.replace_draft(&query, cx);
                input.select_all_text(cx);
            });
        }
        cx.stop_propagation();
    }
    fn accept(&mut self, result: Result<SearchProgress, String>, cx: &mut Context<Self>) {
        match result {
            Ok(progress) => {
                self.pending = progress.pending;
                self.error = false;
                self.status = if progress.pending { format!("{} matches · searching…", progress.matches) }
                    else if self.input.read(cx).value().is_empty() { "Find in terminal".into() }
                    else if let Some(selected) = progress.selected { format!("{} of {}", selected + 1, progress.matches) }
                    else { format!("{} matches", progress.matches) };
                if self.pending { self.schedule(cx); }
                else {
                    self.task = None;
                    while !self.pending {
                        let Some(older) = self.navigation.pop_front() else { break };
                        self.navigate_find(older, cx);
                    }
                }
            }
            Err(error) => { self.error = true; self.pending = false; self.task = None; self.navigation.clear(); self.status = error; }
        }
        cx.notify();
    }
    fn schedule(&mut self, cx: &mut Context<Self>) {
        if self.task.is_some() { return; }
        self.task = Some(cx.spawn(async move |entity, cx| {
            cx.background_executor().timer(std::time::Duration::from_millis(8)).await;
            let _ = entity.update(cx, |this, cx| {
                this.task = None;
                let result = this.parent.read_with(cx, |terminal, _| terminal.engine.search_step());
                this.accept(result.unwrap_or_else(|_| Err("Terminal closed".into())), cx);
            });
        }));
    }
}
impl FindHost for FindBar {
    fn focus_find(&mut self, window: &mut Window, cx: &mut Context<Self>) { self.input.read(cx).focus.clone().focus(window, cx); }
    fn close_find(&mut self, cx: &mut Context<Self>) { cx.emit(Closed); }
    fn navigate_find(&mut self, older: bool, cx: &mut Context<Self>) {
        if self.error || self.input.read(cx).value().is_empty() { return; }
        self.history.remember(self.input.read(cx).value());
        if self.pending {
            if self.navigation.len() < 64 { self.navigation.push_back(older); }
            return;
        }
        let result = self.parent.update(cx, |terminal, cx| {
            let result = terminal.engine.search_navigate(older);
            terminal.refresh_search_selection(cx);
            result
        });
        self.accept(result.unwrap_or_else(|_| Err("Terminal closed".into())), cx);
    }
}
impl Render for FindBar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::find_presentation::bar(self.input.clone(), self.status.clone(), self.error, cx)
            .on_key_down(cx.listener(Self::history_key))
            .on_action(cx.listener(|this, _: &crate::Find, window, cx| this.focus_query(window, cx)))
            .on_action(cx.listener(|this, _: &crate::FindOlder, _, cx| this.navigate_find(true, cx)))
            .on_action(cx.listener(|this, _: &crate::FindNewer, _, cx| this.navigate_find(false, cx)))
    }
}
