use super::*;
use rmux_terminal::{search::Direction, search_worker::SearchWorker};
use text_field::{EditEvent, TextField};

pub(super) use rmux_core::search_state::QueryHistory;

pub(super) struct Closed;
pub(super) struct FindBar {
    pub focus: FocusHandle,
    pub worker: SearchWorker,
    input: Entity<TextField>,
    status: String,
    status_error: bool,
    history: QueryHistory,
    _updates: Task<()>,
}
impl EventEmitter<Closed> for FindBar {}
impl FindBar {
    pub fn navigate(&mut self, direction: Direction, cx: &App) {
        let query = self.input.read(cx).value();
        if query.is_empty() || self.status_error { return; }
        self.history.remember(query);
        // The worker retains navigation while the initial scan runs.
        self.worker.navigate(direction);
    }
    pub fn focus_query(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.input.update(cx, |input, cx| input.select_all_text(cx));
        self.focus.focus(window);
    }
    pub fn query<'a>(&'a self, cx: &'a App) -> &'a str {
        self.input.read(cx).value()
    }
    pub fn saved_history(&self, cx: &App) -> QueryHistory {
        let mut history = self.history.clone();
        history.remember(self.query(cx));
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
    pub fn new(
        worker: SearchWorker,
        parent: WeakEntity<TerminalView>,
        initial_query: &str,
        history: QueryHistory,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = cx.new(|cx| TextField::new(initial_query, cx));
        let mut status_error = false;
        let status = if initial_query.is_empty() {
            "Find in terminal".into()
        } else {
            match worker.set_query(initial_query.to_owned()) {
                Ok(_) => "Searching…".into(),
                Err(error) => { status_error = true; error.to_string() },
            }
        };
        let focus = input.read(cx).focus.clone();
        cx.subscribe(&input, |view, _, event: &EditEvent, cx| match event {
            EditEvent::Changed(text) => {
                view.history.edited(text);
                view.status_error = false;
                view.status = if text.is_empty() { "Find in terminal" } else { "Searching…" }.into();
                if let Err(error) = view.worker.set_query(text.clone()) {
                    view.status_error = true;
                    view.status = error.to_string();
                }
                cx.notify();
            }
            EditEvent::Submitted { reverse, .. } => {
                view.navigate(if *reverse { Direction::Newer } else { Direction::Older }, cx);
            },
            EditEvent::Cancelled => cx.emit(Closed),
        })
        .detach();
        let updates = worker.updates.clone();
        let task = cx.spawn(async move |entity, cx| {
            while updates.recv().await.is_ok() {
                if entity
                    .update(cx, |view, cx| {
                        if let Some(update) = view.worker.latest() {
                            if view.input.read(cx).value().len() > 4096 { return; }
                            view.status_error = update.result.is_err();
                            view.status = match update.result {
                                Ok(_) if view.input.read(cx).value().is_empty() => "Find in terminal".into(),
                                Ok(p) if p.matches == 0 => if p.pending { "Searching…" } else { "No matches" }.into(),
                                Ok(p) => format!(
                                    "{} / {}{}",
                                    p.selected.map_or(0, |i| i + 1),
                                    p.matches,
                                    if p.pending { " …" } else { "" }
                                ),
                                Err(error) => error,
                            };
                            cx.notify();
                            let _ = parent.update(cx, |_, cx| cx.notify());
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            focus,
            worker,
            input,
            status,
            status_error,
            history,
            _updates: task,
        }
    }
}
impl crate::find_presentation::FindHost for FindBar {
    fn focus_find(&mut self, window: &mut Window, _: &mut Context<Self>) { self.focus.focus(window); }
    fn navigate_find(&mut self, older: bool, cx: &mut Context<Self>) {
        self.navigate(if older { Direction::Older } else { Direction::Newer }, cx);
    }
    fn close_find(&mut self, cx: &mut Context<Self>) { cx.emit(Closed); }
}
impl Render for FindBar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::find_presentation::bar(self.input.clone(), self.status.clone(), self.status_error, cx)
            .on_key_down(cx.listener(Self::history_key))
    }
}

