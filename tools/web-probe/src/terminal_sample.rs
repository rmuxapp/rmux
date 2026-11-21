//! Sample shell output parsed by browser Ghostty and drawn by the native painter.
use gpui::{prelude::*, *};
use std::ops::Range;
use rmux_core::terminal_frame::Frame;
use crate::{cell_metrics::CellMetrics, row_cache, row_host, terminal_engine::TerminalEngine};
pub(crate) struct TerminalSample {
    engine: TerminalEngine,
    frame: Frame,
    rows: row_cache::RowCache,
    cursor_cache: row_cache::CursorCache,
    focus: FocusHandle,
    command: String,
    composition: crate::composition::Composition,
    cursor_bounds: Bounds<Pixels>,
    selection_geometry: Option<(Bounds<Pixels>, CellMetrics)>,
    selecting: bool,
    scroll_remainder: f32,
    scrollbar_drag: Option<crate::scrollbar_view::Drag>,
}
impl TerminalSample {
    pub fn scroll(&mut self, scroll: rmux_core::scroll::Scroll, cx: &mut Context<Self>) {
        use rmux_core::scroll::Scroll;
        let (kind,delta) = match scroll {
            Scroll::Row(row) => ("ROW",row.min(i32::MAX as u64) as isize),
            Scroll::Lines(delta) => ("DELTA",delta),
            Scroll::Prompts(delta) => ("DELTA_PROMPT",delta),
            Scroll::Top => ("TOP",0),
            Scroll::Bottom => ("BOTTOM",0),
            Scroll::LastPrompt => {
                self.engine.scroll("BOTTOM",0);
                self.engine.scroll("DELTA_PROMPT",-1);
                ("DELTA_PROMPT",i32::MAX as isize)
            }
        };
        self.engine.scroll(kind,delta as i32);
        self.frame = self.engine.frame(Some(&self.frame));
        self.scroll_remainder = 0.;
        cx.notify();
    }
    pub fn is_focused(&self, window: &Window) -> bool { self.focus.is_focused(window) }
    pub fn release_render_cache(&mut self) {
        self.selecting = false;
        self.scrollbar_drag = None;
        self.engine.focus(false);
        self.rows.release();
        self.cursor_cache = Default::default();
    }
    pub fn focus(&self, window: &mut Window, cx: &mut App) { self.focus.focus(window, cx); }
    pub fn new(cx: &mut Context<Self>) -> Self {
        let engine = TerminalEngine::new(52, 16);
        engine.write("rmux shared renderer\r\n\x1b[32mLive Ghostty parser\x1b[0m\r\nSample session: help, clear, ls\r\n$ ");
        let frame = engine.frame(None);
        Self { engine, frame, rows: Default::default(), cursor_cache: Default::default(), focus: cx.focus_handle(), command: String::new(), composition: Default::default(), cursor_bounds: Default::default(), selection_geometry: None, selecting: false, scroll_remainder: 0., scrollbar_drag: None }
    }
    fn select(&mut self, action: &str, position: Point<Pixels>, rectangle: bool, clicks: usize, cx: &mut Context<Self>) {
        let Some((bounds,cells)) = self.selection_geometry else { return; };
        let point = position - bounds.origin;
        self.engine.select(action, f32::from(point.x), f32::from(point.y), cells, &self.frame, rectangle, clicks);
        self.frame = self.engine.frame(Some(&self.frame));
        cx.notify();
    }
    fn mouse_up(&mut self, event: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(drag) = self.scrollbar_drag.take() {
            if let Some((bounds,_)) = self.selection_geometry {
                if let Some(scroll) = drag.target(bounds,&self.frame,event.position) { self.scroll(scroll,cx); }
            }
            cx.stop_propagation();
            return;
        }
        if self.selecting {
            self.selecting = false;
            self.select("RELEASE", event.position, event.modifiers.alt, 1, cx);
            cx.stop_propagation();
        }
    }
    fn key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke;
        if key.modifiers.platform { return; }
        if key.modifiers.shift && !key.modifiers.control && !key.modifiers.alt {
            if let Some(scroll) = rmux_core::scroll::navigation(&key.key, self.frame.rows.len()) {
                self.scroll(scroll,cx);
                cx.stop_propagation();
                return;
            }
        }
        // Printable text is committed by the platform input handler, as on desktop.
        if !key.modifiers.control && !key.modifiers.alt && key.key_char.is_some() { return; }
        let Some(symbol) = rmux_core::keyboard::key(&key.key) else { return; };
        let mods = rmux_core::keyboard::Modifiers { shift:key.modifiers.shift, control:key.modifiers.control, alt:key.modifiers.alt };
        let text = if mods.control || mods.alt { key.key.as_str() } else { key.key_char.as_deref().unwrap_or("") };
        let encoded = self.engine.encode(symbol, mods, event.is_held, text);
        self.accept(&encoded, cx);
        cx.stop_propagation();
    }
    fn accept(&mut self, encoded: &str, cx: &mut Context<Self>) {
        if !encoded.is_empty() && self.frame.scrollback_offset > 0 { self.engine.scroll("BOTTOM", 0); }
        match encoded {
            "\r" => {
                self.engine.write("\r\n");
                if self.command.trim() == "logs" {
                    let output = (1..=80).map(|line| format!("Sample log {line:03}: workspace ready\r\n")).collect::<String>();
                    self.engine.write(&output);
                }
                let response = match self.command.trim() {
                    "help" => "Sample commands: help, clear, ls, logs\r\nNo shell or server access.\r\n",
                    "logs" => "",
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
                if self.command.is_empty() { cx.stop_propagation(); return; }
                self.engine.write(&"\x08 \x08".repeat(self.command.len()));
                self.command.clear();
            }
            text => {
                // The sample shell accepts ASCII; IME and a real backend follow.
                if text.is_empty() || self.command.len()+text.len() > 48 || !text.bytes().all(|b| (32..127).contains(&b)) { return; }
                self.command.push_str(text); self.engine.write(text);
            }
        }
        self.frame = self.engine.frame(Some(&self.frame));
        cx.notify();
    }
}
impl Render for TerminalSample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        let input = entity.clone();
        div().size_full().key_context("Terminal").track_focus(&self.focus)
            .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &MouseDownEvent, window, cx| {
                this.focus.focus(window, cx);
                if let Some((bounds,_)) = this.selection_geometry {
                    if let Some(drag) = crate::scrollbar_view::begin_drag(bounds,&this.frame,event.position) {
                        this.selecting = false;
                        this.scrollbar_drag = Some(drag);
                        if let Some(scroll) = drag.target(bounds,&this.frame,event.position) { this.scroll(scroll,cx); }
                        return;
                    }
                }
                this.selecting = true;
                this.select("PRESS", event.position, event.modifiers.alt, event.click_count as usize, cx);
            }))
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                if let Some(drag) = this.scrollbar_drag {
                    if event.dragging() {
                        if let Some((bounds,_)) = this.selection_geometry {
                            if let Some(scroll) = drag.target(bounds,&this.frame,event.position) { this.scroll(scroll,cx); }
                        }
                        cx.stop_propagation();
                    } else { this.scrollbar_drag = None; }
                    return;
                }
                if this.selecting && event.dragging() {
                    this.select("DRAG", event.position, event.modifiers.alt, 1, cx);
                    cx.stop_propagation();
                } else if !event.dragging() { this.selecting = false; }
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_action(cx.listener(|this, _: &crate::CopySelection, _, cx| {
                if let Some(text) = this.engine.selected_text() {
                    crate::clipboard::write(ClipboardItem::new_string(text), cx);
                }
            }))
            .on_key_down(cx.listener(Self::key))
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, cx| {
                if let Some((_,cells)) = this.selection_geometry {
                    let lines = rmux_core::scroll::wheel_lines(&mut this.scroll_remainder,
                        f32::from(event.delta.pixel_delta(px(cells.height)).y), cells.height);
                    if lines != 0 {
                        this.engine.scroll("DELTA", lines as i32);
                        this.frame = this.engine.frame(Some(&this.frame));
                        cx.notify();
                    }
                    cx.stop_propagation();
                }
            }))
            .child(canvas(move |bounds, window, cx| entity.update(cx, |this, _| {
                let family: SharedString = "Fira Mono".into();
                let cells = CellMetrics::measured(14., family.clone(), window);
                let content = crate::cell_metrics::content_bounds(bounds, 8);
                this.selection_geometry = Some((content,cells));
                let (columns, height) = cells.grid_size(content.size);
                // Initial/hidden canvas layout may have zero space. Do not
                // reflow a live session through a synthetic 1×1 viewport.
                let visible = content.size.width >= px(cells.width) && content.size.height >= px(cells.height);
                if visible && (columns != this.frame.columns || usize::from(height) != this.frame.rows.len()) {
                    this.engine.resize(columns, height);
                    this.frame = this.engine.frame(Some(&this.frame));
                }
                let (cursor_x, cursor_y) = this.frame.cursor.unwrap_or_default();
                this.cursor_bounds = Bounds::new(content.origin + point(px(f32::from(cursor_x)*cells.width), px(f32::from(cursor_y)*cells.height)), size(px(cells.width), px(cells.height)));
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
                let scrollbar = crate::scrollbar_view::indicator(content, &this.frame);
                (rows, cells, row_host::color(this.frame.background), bounds, content, cursor, this.frame.selections.clone(), scrollbar)
            }), move |_, (rows, cells, background, bounds, content, cursor, selections, scrollbar), window, cx| {
                window.handle_input(&input.read(cx).focus, ElementInputHandler::new(bounds, input.clone()), cx);
                input.read(cx).engine.focus(input.read(cx).focus.is_focused(window));
                window.with_content_mask(Some(row_host::mask(bounds)), |window| {
                    window.paint_quad(fill(bounds, background));
                    for (y, row) in rows.iter().enumerate() {
                        row_cache::paint_row(row, content.origin + point(px(0.), px(y as f32 * cells.height)), selections.get(y).copied().flatten(), None, cells, window, cx);
                    }
                    if let Some((quads, glyph, origin)) = cursor {
                        for quad in quads { window.paint_quad(quad); }
                        if let Some(glyph) = glyph { glyph.paint(origin, px(cells.height), window, cx); }
                    }
                    if let Some(scrollbar) = scrollbar { window.paint_quad(scrollbar); }
                });
            }).size_full())
    }
}

// The same committed-text/composition boundary as the desktop terminal. Keydown
// handles encoded control keys; platform text events own printable input.
impl EntityInputHandler for TerminalSample {
    fn text_for_range(&mut self, range: Range<usize>, actual: &mut Option<Range<usize>>, _: &mut Window, _: &mut Context<Self>) -> Option<String> {
        let (text, adjusted) = self.composition.substring(range);
        *actual = Some(adjusted);
        Some(text)
    }
    fn selected_text_range(&mut self, _: bool, _: &mut Window, _: &mut Context<Self>) -> Option<UTF16Selection> {
        Some(UTF16Selection { range: self.composition.selected.clone(), reversed: false })
    }
    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> { self.composition.marked.clone() }
    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let text = self.composition.take();
        if !text.is_empty() { self.accept(&text, cx); }
    }
    fn replace_text_in_range(&mut self, range: Option<Range<usize>>, text: &str, _: &mut Window, cx: &mut Context<Self>) {
        let text = self.composition.commit(range, text);
        self.accept(&text, cx);
    }
    fn replace_and_mark_text_in_range(&mut self, range: Option<Range<usize>>, text: &str, selected: Option<Range<usize>>, _: &mut Window, cx: &mut Context<Self>) {
        self.composition.update(range, text, selected);
        cx.notify();
    }
    fn bounds_for_range(&mut self, _: Range<usize>, _: Bounds<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<Bounds<Pixels>> { Some(self.cursor_bounds) }
    fn character_index_for_point(&mut self, _: Point<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<usize> { Some(0) }
}
