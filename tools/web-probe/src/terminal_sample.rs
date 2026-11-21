//! Sample shell output parsed by browser Ghostty and drawn by the native painter.
use gpui::{prelude::*, *};
use std::ops::Range;
use rmux_core::terminal_frame::Frame;
use crate::{cell_metrics::CellMetrics, row_cache, row_host, terminal_engine::TerminalEngine};
include!("sample_terminal_menu.rs");
pub(crate) struct TerminalSample {
    menu: Option<TerminalMenu>,
    menu_blur: Option<Subscription>,
    clipboard_read: Option<Task<()>>,
    pub(crate) engine: TerminalEngine,
    pub(crate) custom_title: Option<String>,
    find: Option<Entity<crate::browser_find::FindBar>>,
    find_history: rmux_core::search_state::QueryHistory,
    last_find_query: String,
    frame: Frame,
    frame_released: bool,
    font_revision: u64,
    measured_cells: Option<CellMetrics>,
    pub(crate) font_size: f32,
    rows: row_cache::RowCache,
    cursor_cache: row_cache::CursorCache,
    focus: FocusHandle,
    session: rmux_core::sample_session::SampleSession,
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
    fn refresh_frame(&mut self) {
        self.frame = self.engine.frame((!self.frame_released).then_some(&self.frame));
        self.frame_released = false;
    }

    pub(crate) fn refresh_search_selection(&mut self, cx: &mut Context<Self>) {
        self.refresh_frame();
        cx.notify();
    }
    fn open_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open_find_with_query(None, window, cx);
    }
    fn open_find_with_query(&mut self, initial: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(find) = &self.find {
            find.update(cx, |find, cx| find.focus_query(window, cx));
            return;
        }
        let parent = cx.entity();
        let history = self.find_history.clone();
        let query = initial.or_else(|| self.engine.selected_search_query()).unwrap_or_else(|| self.last_find_query.clone());
        let progress = self.engine.search_query(&query);
        let find = cx.new(|cx| crate::browser_find::FindBar::new(parent, history, &query, progress, window, cx));
        cx.subscribe_in(&find, window, |this, find, _: &crate::browser_find::Closed, window, cx| {
            this.find_history = find.read(cx).saved_history(cx);
            this.last_find_query = find.read(cx).query(cx).to_owned();
            this.find = None;
            this.engine.search_close();
            this.focus.focus(window, cx);
            cx.notify();
        }).detach();
        self.find = Some(find);
        cx.notify();
    }

    fn navigate_find(&mut self, older: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.find.is_none() {
            let remembered = (!self.last_find_query.is_empty()).then(|| self.last_find_query.clone());
            self.open_find_with_query(remembered, window, cx);
        }
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
        self.refresh_frame();
        self.scroll_remainder = 0.;
        cx.notify();
    }
    pub(crate) fn focus_handle(&self) -> &FocusHandle { &self.focus }
    pub fn is_focused(&self, window: &Window) -> bool { self.focus.is_focused(window) }
    pub fn release_render_cache(&mut self, cx: &mut Context<Self>) {
        self.selecting = false;
        self.selection_pointer = None;
        self.selection_scroll = None;
        self.scrollbar_drag = None;
        self.engine.focus(false);
        self.release_pressed(cx);
        self.engine.release_render_state();
        row_cache::release_frame_cells(&mut self.frame);
        self.frame_released = true;
        self.selection_geometry = None;
        self.rows.release();
        self.cursor_cache = Default::default();
    }
    pub fn focus(&self, window: &mut Window, cx: &mut App) { self.focus.focus(window, cx); }
    pub fn new(font_size: f32, window: &mut Window, cx: &mut Context<Self>) -> Self {
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
        Self { menu: None, menu_blur: None, clipboard_read: None, engine, custom_title: None, find: None, find_history: Default::default(), last_find_query: String::new(), frame, frame_released: false, font_revision: 0, measured_cells: None, font_size: CellMetrics::new(font_size).font_size, rows: Default::default(), cursor_cache: Default::default(), focus, session: Default::default(), input_error: None, pressed_keys: Default::default(), composition: Default::default(), cursor_bounds: Default::default(), selection_geometry: None, selecting: false, scroll_remainder: 0., selection_pointer: None, selection_scroll: None, scrollbar_drag: None }
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
        self.refresh_frame();
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
        match self.session.input(encoded, self.frame.columns, |text| crate::terminal_engine::graphemes(text).ok()) {
            Ok(Some(output)) => {
                crate::fonts::ensure(encoded, cx);
                self.engine.write(&output);
            }
            Ok(None) => return,
            Err(error) => {
                self.input_error = Some(error.into());
                cx.notify();
                return;
            }
        }
        self.input_error = None;
        self.refresh_frame();
        cx.emit(SearchChanged);
        cx.notify();
    }
}
impl Render for TerminalSample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        let input = entity.clone();
        crate::terminal_actions::bind(div(), cx).size_full().flex().flex_col().key_context("Terminal").track_focus(&self.focus)
            .when_some(self.menu.clone(), |root, menu| root.child(self.menu_element(menu, window, cx)))
            .on_mouse_down(MouseButton::Right, cx.listener(|this, event: &MouseDownEvent, window, cx| {
                cx.stop_propagation(); this.show_menu(event.position, window, cx);
            }))
            .on_action(cx.listener(|this, _: &crate::Paste, window, cx| this.paste_from_clipboard(window, cx)))
            .on_action(cx.listener(|this, _: &crate::ShowTerminalMenu, window, cx| {
                let position = this.selection_geometry.map(|(bounds, _)| bounds.origin).unwrap_or_default();
                this.show_menu(position + point(px(8.), px(8.)), window, cx);
            }))
            .on_action(cx.listener(|this, _: &crate::Find, window, cx| this.open_find(window, cx)))
            .on_action(cx.listener(|this, _: &crate::FindOlder, window, cx| this.navigate_find(true, window, cx)))
            .on_action(cx.listener(|this, _: &crate::FindNewer, window, cx| this.navigate_find(false, window, cx)))
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
            .on_key_down(cx.listener(Self::key))
            .on_key_up(cx.listener(Self::key_up))
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, cx| {
                if let Some((_,cells)) = this.selection_geometry {
                    let lines = rmux_core::scroll::wheel_lines(&mut this.scroll_remainder,
                        f32::from(event.delta.pixel_delta(px(cells.height)).y), cells.height);
                    if lines != 0 {
                        this.engine.scroll("DELTA", lines as i32);
                        this.refresh_frame();
                        cx.notify();
                    }
                    cx.stop_propagation();
                }
            }))
            .child(canvas(move |bounds, window, cx| entity.update(cx, |this, cx| {
                if this.frame_released { this.refresh_frame(); }
                let revision = cx.global::<crate::fonts::Fonts>().revision;
                if this.font_revision != revision {
                    this.rows.release(); this.cursor_cache = Default::default();
                    this.font_revision = revision;
                    this.measured_cells = None;
                }
                let family: SharedString = "Fira Mono".into();
                // Font metrics are logical pixels, independent of pane bounds.
                // Keep this small value across hidden-pane cache reclamation;
                // only font changes require another text-system measurement.
                let cells = *this.measured_cells.get_or_insert_with(||
                    CellMetrics::measured(this.font_size, family.clone(), window));
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
                    this.engine.write(&this.session.display(columns));
                    this.refresh_frame();
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
            Ok(encoded) => match self.session.paste(&encoded, self.frame.columns, |text| crate::terminal_engine::graphemes(text).ok()) {
                Ok(output) => {
                    self.input_error = None;
                    crate::fonts::ensure(&text, cx);
                    if !output.is_empty() {
                        self.engine.scroll("BOTTOM", 0);
                        self.engine.write(&output);
                        self.refresh_frame();
                        cx.emit(SearchChanged);
                    }
                }
                Err(error) => self.input_error = Some(error.into()),
            },
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

impl crate::terminal_actions::TerminalCommandHost for TerminalSample {
    fn resize_terminal_font(&mut self, delta: Option<f32>, cx: &mut Context<Self>) {
        let next = CellMetrics::new(delta.map_or(14., |delta| self.font_size + delta)).font_size;
        if next == self.font_size { return; }
        self.font_size = next;
        self.measured_cells = None;
        self.rows.release();
        self.cursor_cache = Default::default();
        self.selection_geometry = None;
        self.selecting = false;
        self.selection_pointer = None;
        self.selection_scroll = None;
        self.scrollbar_drag = None;
        self.scroll_remainder = 0.;
        cx.notify();
    }
    fn copy_terminal(&mut self, _: &mut Window, cx: &mut Context<Self>) { self.copy_selection(true, cx); }
    fn copy_terminal_with_line_breaks(&mut self, _: &mut Window, cx: &mut Context<Self>) { self.copy_selection(false, cx); }
    fn select_terminal(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.selection_scroll = None;
        self.selecting = false;
        self.selection_pointer = None;
        self.input_error = self.engine.select_all().err().map(|_| "Could not select terminal text.".into());
        self.refresh_frame();
        cx.notify();
    }
    fn clear_terminal_history(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.selection_scroll = None;
        self.selecting = false;
        self.selection_pointer = None;
        self.input_error = self.engine.clear_scrollback().err().map(|_| "Could not clear scrollback.".into());
        self.refresh_frame();
        cx.emit(SearchChanged);
        cx.notify();
    }
}

impl TerminalSample {
    fn copy_selection(&mut self, unwrap: bool, cx: &mut Context<Self>) {
        match if unwrap { self.engine.selected_text() } else { self.engine.selected_text_with_wraps() } {
            Ok(Some(text)) => {
                self.input_error = None;
                crate::clipboard::write(ClipboardItem::new_string(text), cx);
            }
            Ok(None) => {}
            Err(_) => self.input_error = Some("Could not copy selection. Try selecting less text (maximum 1 MiB).".into()),
        }
        cx.notify();
    }
}
