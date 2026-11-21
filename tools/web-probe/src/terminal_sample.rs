//! Sample shell output parsed by browser Ghostty and drawn by the native painter.
use gpui::{prelude::*, *};
use rmux_core::terminal_frame::Frame;
use crate::{cell_metrics::CellMetrics, row_cache, row_host, terminal_engine::TerminalEngine};
pub(crate) struct TerminalSample {
    engine: TerminalEngine,
    frame: Frame,
    rows: row_cache::RowCache,
    focus: FocusHandle,
    command: String,
}
impl TerminalSample {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let engine = TerminalEngine::new(52, 16);
        engine.write("rmux shared renderer\r\n\x1b[32mLive Ghostty parser\x1b[0m\r\nSample session: help, clear, ls\r\n$ ");
        let frame = engine.frame(None);
        Self { engine, frame, rows: Default::default(), focus: cx.focus_handle(), command: String::new() }
    }
    fn key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke;
        if key.modifiers.platform || key.modifiers.control || key.modifiers.alt { return; }
        match key.key.as_str() {
            "enter" => {
                self.engine.write("\r\n");
                let response = match self.command.trim() {
                    "help" => "Sample commands: help, clear, ls\r\nNo shell or server access.\r\n",
                    "ls" => "\x1b[34mcrates/  docs/  scripts/\x1b[0m  Cargo.toml\r\n",
                    "clear" => "\x1b[2J\x1b[H",
                    "" => "",
                    _ => "Unknown sample command. Try help.\r\n",
                };
                self.engine.write(response);
                self.engine.write("$ ");
                self.command.clear();
            }
            "backspace" if !self.command.is_empty() => {
                self.command.pop(); self.engine.write("\x08 \x08");
            }
            _ => {
                let Some(text) = key.key_char.as_deref() else { return; };
                // Initial ASCII sample-shell path; native IME/input sharing follows.
                if self.command.len() >= 48 || !text.bytes().all(|b| (32..127).contains(&b)) { return; }
                self.command.push_str(text); self.engine.write(text);
            }
        }
        self.frame = self.engine.frame(Some(&self.frame));
        cx.notify(); cx.stop_propagation();
    }
}
impl Render for TerminalSample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        div().size_full().track_focus(&self.focus)
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.focus.focus(window, cx)))
            .on_key_down(cx.listener(Self::key))
            .child(canvas(move |bounds, window, cx| entity.update(cx, |this, _| {
                let family: SharedString = "Fira Mono".into();
                let cells = CellMetrics::measured(14., family.clone(), window);
                let rows = this.rows.update(&this.frame, cells, &family, window, None);
                (rows, cells, row_host::color(this.frame.background), bounds)
            }), move |_, (rows, cells, background, bounds), window, cx| {
                window.with_content_mask(Some(row_host::mask(bounds)), |window| {
                    window.paint_quad(fill(bounds, background));
                    for (y, row) in rows.iter().enumerate() {
                        row_cache::paint_row(row, bounds.origin + point(px(8.), px(8. + y as f32 * cells.height)), None, None, cells, window, cx);
                    }
                });
            }).size_full())
    }
}
