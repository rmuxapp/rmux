//! Sample shell output parsed by browser Ghostty and drawn by the native painter.
use gpui::{prelude::*, *};
use rmux_core::terminal_frame::Frame;
use crate::{cell_metrics::CellMetrics, row_cache, row_host, terminal_engine::TerminalEngine};
pub(crate) struct TerminalSample {
    engine: TerminalEngine,
    frame: Frame,
    rows: row_cache::RowCache,
    cursor_cache: row_cache::CursorCache,
    focus: FocusHandle,
    command: String,
}
impl TerminalSample {
    pub fn is_focused(&self, window: &Window) -> bool { self.focus.is_focused(window) }
    pub fn release_render_cache(&mut self) {
        self.rows.release();
        self.cursor_cache = Default::default();
    }
    pub fn focus(&self, window: &mut Window, cx: &mut App) { self.focus.focus(window, cx); }
    pub fn new(cx: &mut Context<Self>) -> Self {
        let engine = TerminalEngine::new(52, 16);
        engine.write("rmux shared renderer\r\n\x1b[32mLive Ghostty parser\x1b[0m\r\nSample session: help, clear, ls\r\n$ ");
        let frame = engine.frame(None);
        Self { engine, frame, rows: Default::default(), cursor_cache: Default::default(), focus: cx.focus_handle(), command: String::new() }
    }
    fn key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke;
        if key.modifiers.platform { return; }
        let Some(symbol) = rmux_core::keyboard::key(&key.key) else { return; };
        let mods = rmux_core::keyboard::Modifiers { shift:key.modifiers.shift, control:key.modifiers.control, alt:key.modifiers.alt };
        let text = if mods.control || mods.alt { key.key.as_str() } else { key.key_char.as_deref().unwrap_or("") };
        let encoded = self.engine.encode(symbol, mods, event.is_held, text);
        match encoded.as_str() {
            "\r" => {
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
            "\x7f" | "\x08" if !self.command.is_empty() => {
                self.command.pop(); self.engine.write("\x08 \x08");
            }
            "\x03" => { self.engine.write("^C\r\n$ "); self.command.clear(); }
            "\x15" => {
                for _ in 0..self.command.len() { self.engine.write("\x08 \x08"); }
                self.command.clear();
            }
            text => {
                // The sample shell accepts ASCII; IME and a real backend follow.
                if text.is_empty() || self.command.len()+text.len() > 48 || !text.bytes().all(|b| (32..127).contains(&b)) { return; }
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
        div().size_full().key_context("Terminal").track_focus(&self.focus)
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.focus.focus(window, cx)))
            .on_key_down(cx.listener(Self::key))
            .child(canvas(move |bounds, window, cx| entity.update(cx, |this, _| {
                let family: SharedString = "Fira Mono".into();
                let cells = CellMetrics::measured(14., family.clone(), window);
                let content = crate::cell_metrics::content_bounds(bounds, 8);
                let (columns, height) = cells.grid_size(content.size);
                // Initial/hidden canvas layout may have zero space. Do not
                // reflow a live session through a synthetic 1×1 viewport.
                let visible = content.size.width >= px(cells.width) && content.size.height >= px(cells.height);
                if visible && (columns != this.frame.columns || usize::from(height) != this.frame.rows.len()) {
                    this.engine.resize(columns, height);
                    this.frame = this.engine.frame(Some(&this.frame));
                }
                let rows = this.rows.update(&this.frame, cells, &family, window, None);
                let cursor = this.frame.cursor.and_then(|(x,y)| {
                    let cell = this.frame.rows.get(usize::from(y))?.get(usize::from(x))?;
                    let shape = if this.focus.is_focused(window) { this.frame.cursor_shape } else { rmux_core::terminal_frame::CursorShape::Hollow };
                    let origin = content.origin + point(px(f32::from(x) * cells.width), px(f32::from(y) * cells.height));
                    let quads = crate::cursor_view::quads(shape, origin, f32::from(cell.width.max(1))*cells.width, cells.height, row_host::color(this.frame.cursor_color));
                    let glyph = (shape == rmux_core::terminal_frame::CursorShape::Block && !cell.hidden && !cell.text.is_empty())
                        .then(|| this.cursor_cache.shape(cell, cell.background, cells, &family, window, None));
                    Some((quads, glyph, origin))
                });
                (rows, cells, row_host::color(this.frame.background), bounds, content, cursor)
            }), move |_, (rows, cells, background, bounds, content, cursor), window, cx| {
                window.with_content_mask(Some(row_host::mask(bounds)), |window| {
                    window.paint_quad(fill(bounds, background));
                    for (y, row) in rows.iter().enumerate() {
                        row_cache::paint_row(row, content.origin + point(px(0.), px(y as f32 * cells.height)), None, None, cells, window, cx);
                    }
                    if let Some((quads, glyph, origin)) = cursor {
                        for quad in quads { window.paint_quad(quad); }
                        if let Some(glyph) = glyph { glyph.paint(origin, px(cells.height), window, cx); }
                    }
                });
            }).size_full())
    }
}
