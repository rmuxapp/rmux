//! Sample shell output parsed by browser Ghostty and drawn by the native painter.
use gpui::{prelude::*, *};
use std::ops::Range;
use rmux_core::terminal_frame::Frame;
use crate::{cell_metrics::CellMetrics, row_cache, row_host, terminal_engine::TerminalEngine};
pub(crate) struct TerminalSample {
    pub(crate) engine: TerminalEngine,
    find: Option<Entity<crate::browser_find::FindBar>>,
    find_history: rmux_core::search_state::QueryHistory,
    frame: Frame,
    font_revision: u64,
    rows: row_cache::RowCache,
    cursor_cache: row_cache::CursorCache,
    focus: FocusHandle,
    command: rmux_core::sample_input::SampleInput,
    input_error: Option<String>,
    pressed_keys: rmux_core::keyboard::PressedKeys,
    composition: crate::composition::Composition,
    cursor_bounds: Bounds<Pixels>,
    selection_geometry: Option<(Bounds<Pixels>, CellMetrics)>,
    selecting: bool,
    scroll_remainder: f32,
    selection_pointer: Option<(Point<Pixels>, bool)>,
    selection_scroll: Option<Task<()>>,
    scrollbar_drag: Option<crate::scrollbar_view::Drag>,
}
pub(crate) struct SearchChanged;
impl EventEmitter<SearchChanged> for TerminalSample {}
impl TerminalSample {
    pub(crate) fn refresh_search_selection(&mut self, cx: &mut Context<Self>) {
        self.frame = self.engine.frame(Some(&self.frame));
        cx.notify();
    }
    fn open_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(find) = &self.find {
            find.update(cx, |find, cx| find.focus_query(window, cx));
            return;
        }
        let parent = cx.entity();
        let history = self.find_history.clone();
        let find = cx.new(|cx| crate::browser_find::FindBar::new(parent, history, window, cx));
        cx.subscribe_in(&find, window, |this, find, _: &crate::browser_find::Closed, window, cx| {
            this.find_history = find.read(cx).saved_history(cx);
            this.find = None;
            this.engine.search_close();
            this.focus.focus(window, cx);
            cx.notify();
        }).detach();
        self.find = Some(find);
        cx.notify();
    }

    fn navigate_find(&self, older: bool, cx: &mut Context<Self>) {
        if let Some(find) = self.find.clone() {
            // Navigation updates the terminal selection, after this entity borrow ends.
            cx.defer(move |cx| find.update(cx, |find, cx| {
                crate::find_presentation::FindHost::navigate_find(find, older, cx);
            }));
        }
    }

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
    pub fn release_render_cache(&mut self, cx: &mut Context<Self>) {
        self.selecting = false;
        self.selection_pointer = None;
        self.selection_scroll = None;
        self.scrollbar_drag = None;
        self.engine.focus(false);
        self.release_pressed(cx);
        self.rows.release();
        self.cursor_cache = Default::default();
    }
    pub fn focus(&self, window: &mut Window, cx: &mut App) { self.focus.focus(window, cx); }
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        cx.on_blur(&focus, window, |this, _, cx| {
            this.release_pressed(cx);
            this.selecting = false;
            this.selection_pointer = None;
            this.selection_scroll = None;
            this.scrollbar_drag = None;
            this.engine.focus(false);
        }).detach();
        let engine = TerminalEngine::new(52, 16);
        engine.write("\x1b[?2027hrmux shared renderer\r\n\x1b[32mLive Ghostty parser\x1b[0m\r\nSample session: help, clear, ls\r\n$ ");
        let frame = engine.frame(None);
        Self { engine, find: None, find_history: Default::default(), frame, font_revision: 0, rows: Default::default(), cursor_cache: Default::default(), focus, command: Default::default(), input_error: None, pressed_keys: Default::default(), composition: Default::default(), cursor_bounds: Default::default(), selection_geometry: None, selecting: false, scroll_remainder: 0., selection_pointer: None, selection_scroll: None, scrollbar_drag: None }
    }
    fn select(&mut self, action: &str, position: Point<Pixels>, rectangle: bool, clicks: usize, cx: &mut Context<Self>) {
        let Some((bounds,cells)) = self.selection_geometry else { return; };
        if self.selecting { self.selection_pointer = Some((position, rectangle)); }
        let point = position - bounds.origin;
        let autoscroll = self.engine.select(action, f32::from(point.x), f32::from(point.y), cells, &self.frame, rectangle, clicks);
        if autoscroll && self.selecting {
            if self.selection_scroll.is_none() {
                let timer = cx.background_executor().timer(rmux_core::scroll::SELECTION_INTERVAL);
                self.selection_scroll = Some(cx.spawn(async move |entity, cx| {
                    timer.await;
                    let _ = entity.update(cx, |this, cx| {
                        this.selection_scroll = None;
                        if this.selecting && let Some((position, rectangle)) = this.selection_pointer {
                            this.select("AUTOSCROLL_TICK", position, rectangle, 1, cx);
                        }
                    });
                }));
            }
        } else { self.selection_scroll = None; }
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
        self.selection_scroll = None;
        self.selection_pointer = None;
        if self.selecting {
            self.selecting = false;
            self.select("RELEASE", event.position, event.modifiers.alt, 1, cx);
            cx.stop_propagation();
        }
    }
    fn key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.focus.is_focused(window) || !self.composition.text.is_empty() { return; }
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
        let text = if mods.control || mods.alt { rmux_core::keyboard::key_text(&key.key) } else { key.key_char.as_deref().unwrap_or("") };
        let action = if event.is_held { rmux_core::keyboard::Action::Repeat } else { rmux_core::keyboard::Action::Press };
        let encoded = self.engine.encode(symbol, mods, action, text);
        self.pressed_keys.press(&key.key);
        self.accept(&encoded, cx);
        cx.stop_propagation();
    }
    fn release_pressed(&mut self, cx: &mut Context<Self>) {
        let held: Vec<_> = self.pressed_keys.drain().collect();
        for name in held { self.release_key(&name, Default::default(), cx); }
    }
    fn release_key(&mut self, name: &str, mods: rmux_core::keyboard::Modifiers, cx: &mut Context<Self>) {
        if let Some(symbol) = rmux_core::keyboard::key(name) {
            let encoded = self.engine.encode(symbol, mods, rmux_core::keyboard::Action::Release, rmux_core::keyboard::key_text(name));
            if !encoded.is_empty() { self.accept(&encoded,cx); }
        }
    }
    fn key_up(&mut self, event: &KeyUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke;
        let Some(pressed) = self.pressed_keys.release(&key.key) else { return; };
        self.release_key(&pressed, rmux_core::keyboard::Modifiers { shift:key.modifiers.shift, control:key.modifiers.control, alt:key.modifiers.alt },cx);
        cx.stop_propagation();
    }
    fn accept(&mut self, encoded: &str, cx: &mut Context<Self>) {
        if !encoded.is_empty() && self.frame.scrollback_offset > 0 { self.engine.scroll("BOTTOM", 0); }
        match encoded {
            "\r" => {
                self.engine.write(&format!("\r\x1b[2K$ {}\r\n", self.command.text()));
                if let Some(text) = self.command.text().strip_prefix("echo ") {
                    self.engine.write(text);
                    self.engine.write("\r\n");
                }
                if self.command.text().trim() == "logs" {
                    let output = (1..=80).map(|line| format!("Sample log {line:03}: workspace ready\r\n")).collect::<String>();
                    self.engine.write(&output);
                }
                let response = match self.command.text().trim() {
                    "help" => "Sample commands: help, clear, ls, logs, echo <text>\r\nNo shell or server access.\r\n",
                    "logs" => "",
                    "ls" => "\x1b[34mcrates/  docs/  scripts/\x1b[0m  Cargo.toml\r\n",
                    "clear" => "\x1b[2J\x1b[H",
                    "" => "",
                    _ if self.command.text().starts_with("echo ") => "",
                    _ => "Unknown sample command. Try help.\r\n",
                };
                self.engine.write(response);
                self.command.clear();
                self.engine.write(&self.command.display(self.frame.columns));
            }
            "\x7f" | "\x08" => {
                self.command.backspace();
                self.engine.write(&self.command.display(self.frame.columns));
            }
            "\x03" => {
                self.engine.write(&format!("\r\x1b[2K$ {}^C\r\n", self.command.text()));
                self.command.clear();
                self.engine.write(&self.command.display(self.frame.columns));
            }
            "\x15" => {
                self.command.clear();
                self.engine.write(&self.command.display(self.frame.columns));
            }
            text => {
                if text.is_empty() || text.chars().any(char::is_control) { return; }
                if let Err(error) = self.command.append(text, |text| crate::terminal_engine::graphemes(text).ok()) {
                    self.input_error = Some(error.into());
                    cx.notify();
                    return;
                }
                crate::fonts::ensure(text, cx);
                self.engine.write(&self.command.display(self.frame.columns));
            }
        }
        self.input_error = None;
        self.frame = self.engine.frame(Some(&self.frame));
        cx.emit(SearchChanged);
        cx.notify();
    }
}
impl Render for TerminalSample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        let input = entity.clone();
        div().size_full().flex().flex_col().key_context("Terminal").track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &crate::Find, window, cx| this.open_find(window, cx)))
            .on_action(cx.listener(|this, _: &crate::FindOlder, _, cx| this.navigate_find(true, cx)))
            .on_action(cx.listener(|this, _: &crate::FindNewer, _, cx| this.navigate_find(false, cx)))
            .when_some(self.find.clone(), |root, find| root.child(find))
            .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &MouseDownEvent, window, cx| {
                this.focus.focus(window, cx);
                if let Some((bounds,_)) = this.selection_geometry {
                    if let Some(drag) = crate::scrollbar_view::begin_drag(bounds,&this.frame,event.position) {
                        this.selecting = false;
                        this.selection_scroll = None;
                        this.selection_pointer = None;
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
                } else if !event.dragging() {
                    this.selecting = false;
                    this.selection_scroll = None;
                    this.selection_pointer = None;
                }
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_action(cx.listener(|this, _: &crate::CopySelection, _, cx| {
                if let Some(text) = this.engine.selected_text() {
                    crate::clipboard::write(ClipboardItem::new_string(text), cx);
                }
            }))
            .on_key_down(cx.listener(Self::key))
            .on_key_up(cx.listener(Self::key_up))
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
            .child(canvas(move |bounds, window, cx| entity.update(cx, |this, cx| {
                let revision = cx.global::<crate::fonts::Fonts>().revision;
                if this.font_revision != revision {
                    this.rows.release(); this.cursor_cache = Default::default();
                    this.font_revision = revision;
                }
                let family: SharedString = "Fira Mono".into();
                let cells = CellMetrics::measured(14., family.clone(), window);
                let content = crate::cell_metrics::content_bounds(bounds, 8);
                this.selection_geometry = Some((content,cells));
                let (columns, height) = cells.grid_size(content.size);
                // Initial/hidden canvas layout may have zero space. Do not
                // reflow a live session through a synthetic 1×1 viewport.
                let visible = content.size.width >= px(cells.width) && content.size.height >= px(cells.height);
                if visible && (columns != this.frame.columns || usize::from(height) != this.frame.rows.len()) {
                    // The draft never wraps. Erase it before reflow and redraw
                    // its grapheme-safe visible suffix at the new width.
                    this.engine.write("\r\x1b[2K");
                    this.engine.resize(columns, height);
                    this.engine.write(&this.command.display(columns));
                    this.frame = this.engine.frame(Some(&this.frame));
                    cx.emit(SearchChanged);
                }
                let (cursor_x, cursor_y) = this.frame.cursor.unwrap_or_default();
                this.cursor_bounds = Bounds::new(content.origin + point(px(f32::from(cursor_x)*cells.width), px(f32::from(cursor_y)*cells.height)), size(px(cells.width), px(cells.height)));
                let rows = this.rows.update(&this.frame, cells, &family, window, None);
                let mut preedit = None;
                let cursor = this.frame.cursor.and_then(|(x,y)| {
                    let cell = this.frame.rows.get(usize::from(y))?.get(usize::from(x))?;
                    let composing = this.focus.is_focused(window) && !this.composition.text.is_empty();
                    let shape = if composing { rmux_core::terminal_frame::CursorShape::Bar } else if this.focus.is_focused(window) { this.frame.cursor_shape } else { rmux_core::terminal_frame::CursorShape::Hollow };
                    let mut origin = content.origin + point(px(f32::from(x) * cells.width), px(f32::from(y) * cells.height));
                    if composing {
                        let overlay = crate::preedit_view::Preedit::new(&this.composition, origin, cells,
                            &family, row_host::color(this.frame.background), window);
                        origin = overlay.cursor;
                        preedit = Some(overlay);
                    }
                    let quads = crate::cursor_view::quads(shape, origin, f32::from(cell.width.max(1))*cells.width, cells.height, row_host::color(this.frame.cursor_color));
                    let glyph = (shape == rmux_core::terminal_frame::CursorShape::Block && !cell.hidden && !cell.text.is_empty())
                        .then(|| this.cursor_cache.shape(cell, cell.background, cells, &family, window, None));
                    Some((quads, glyph, origin))
                });
                let scrollbar = crate::scrollbar_view::indicator(content, &this.frame);
                (rows, cells, row_host::color(this.frame.background), bounds, content, cursor, this.frame.selections.clone(), scrollbar, preedit)
            }), move |_, (rows, cells, background, bounds, content, cursor, selections, scrollbar, mut preedit), window, cx| {
                window.handle_input(&input.read(cx).focus, ElementInputHandler::new(bounds, input.clone()), cx);
                input.read(cx).engine.focus(input.read(cx).focus.is_focused(window));
                window.with_content_mask(Some(row_host::mask(bounds)), |window| {
                    window.paint_quad(fill(bounds, background));
                    for (y, row) in rows.iter().enumerate() {
                        row_cache::paint_row(row, content.origin + point(px(0.), px(y as f32 * cells.height)), selections.get(y).copied().flatten(), None, cells, window, cx);
                    }
                    if let Some(preedit) = &mut preedit { preedit.paint(window, cx); }
                    if let Some((quads, glyph, origin)) = cursor {
                        for quad in quads { window.paint_quad(quad); }
                        if let Some(glyph) = glyph { glyph.paint(origin, px(cells.height), window, cx); }
                    }
                    if let Some(scrollbar) = scrollbar { window.paint_quad(scrollbar); }
                });
            }).flex_1().min_h_0().w_full())
            .when_some(self.input_error.clone().or_else(|| cx.global::<crate::fonts::Fonts>().error.clone()), |root, message| root.child(div().px_2().py_1().text_xs().text_color(rgb(0xf0b85b)).child(message)))
    }
}

// The same committed-text/composition boundary as the desktop terminal. Keydown
// handles encoded control keys; platform text events own printable input.
impl EntityInputHandler for TerminalSample {
    fn paste(&mut self, item: ClipboardItem, _: &mut Window, cx: &mut Context<Self>) {
        let Some(text) = item.text() else { return; };
        match self.engine.encode_paste(&text) {
            Ok(encoded) => { self.input_error = None; self.accept(&encoded, cx); }
            Err(_) => { self.input_error = Some("Could not paste text. Maximum size is 1 MiB.".into()); }
        }
        cx.notify();
    }
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
        cx.notify();
    }
    fn replace_text_in_range(&mut self, range: Option<Range<usize>>, text: &str, _: &mut Window, cx: &mut Context<Self>) {
        let text = self.composition.commit(range, text);
        self.accept(&text, cx);
        cx.notify();
    }
    fn replace_and_mark_text_in_range(&mut self, range: Option<Range<usize>>, text: &str, selected: Option<Range<usize>>, _: &mut Window, cx: &mut Context<Self>) {
        crate::fonts::ensure(text, cx);
        self.composition.update(range, text, selected);
        cx.notify();
    }
    fn bounds_for_range(&mut self, range: Range<usize>, _: Bounds<Pixels>, window: &mut Window, _: &mut Context<Self>) -> Option<Bounds<Pixels>> {
        let (_, cells) = self.selection_geometry?;
        Some(crate::preedit_view::bounds_for_range(&self.composition, range, self.cursor_bounds.origin,
            cells, &"Fira Mono".into(), window))
    }
    fn character_index_for_point(&mut self, position: Point<Pixels>, window: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        let (_, cells) = self.selection_geometry?;
        crate::preedit_view::character_index_for_point(&self.composition, position, self.cursor_bounds.origin,
            cells, &"Fira Mono".into(), window)
    }
}
