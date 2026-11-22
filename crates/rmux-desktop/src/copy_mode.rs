//! Stable, bounded history inspection; no per-frame history formatting.
use super::*;
use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};
use unicode_width::UnicodeWidthStr;
const MAX_BYTES: usize = 8 * 1024 * 1024;
const MAX_LINES: usize = 65_536;

struct Buffer {
    text: String,
    lines: Vec<Range<usize>>,
    truncated: bool,
    column_marks: Vec<(u32, u32)>,
    column_ranges: Vec<Range<usize>>,
}
impl Buffer {
    fn new(mut text: String, mut truncated: bool) -> Self {
        let mut lines = Vec::new();
        let mut start = 0;
        for (index, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                lines.push(start..index);
                start = index + 1;
                if lines.len() == MAX_LINES { break; }
            }
        }
        if lines.len() == MAX_LINES {
            truncated |= start < text.len();
            text.truncate(start);
            if text.capacity() > text.len().saturating_mul(2) { text.shrink_to_fit(); }
        } else { lines.push(start..text.len()); }
        // Sparse checkpoints are built with the snapshot on the worker. Avoid
        // rescanning long prefixes for every vertical key press.
        let mut column_marks = Vec::new();
        let mut column_ranges = Vec::with_capacity(lines.len());
        for range in &lines {
            let first = column_marks.len();
            // Printable ASCII has one cell per byte and needs no checkpoints.
            // Keep controls on the Unicode path (e.g. tab is not width one).
            if text[range.clone()].bytes().all(|byte| (b' '..=b'~').contains(&byte)) {
                column_ranges.push(first..first);
                continue;
            }
            column_marks.push((0, 0));
            let mut cells = 0;
            let mut previous = 0;
            for (byte, grapheme) in text[range.clone()].grapheme_indices(true) {
                if byte - previous >= 256 {
                    column_marks.push((byte as u32, cells as u32));
                    previous = byte;
                }
                cells += UnicodeWidthStr::width(grapheme);
            }
            column_ranges.push(first..column_marks.len());
        }
        Self { text, lines, truncated, column_marks, column_ranges }
    }
    fn find_lines(&self, query: &str, canceled: &AtomicBool) -> Option<Vec<usize>> {
        let mut matches = Vec::new();
        if query.is_empty() { return Some(matches); }
        for (index, range) in self.lines.iter().enumerate() {
            if canceled.load(Ordering::Acquire) { return None; }
            if self.text[range.clone()].contains(query) { matches.push(index); }
        }
        Some(matches)
    }
    fn line(&self, row: usize) -> &str { &self.text[self.lines[row].clone()] }
    fn visual_column(&self, row: usize, byte: usize) -> usize {
        let marks = &self.column_marks[self.column_ranges[row].clone()];
        if marks.is_empty() { return byte; }
        let (start, cells) = marks[marks.partition_point(|(start, _)| *start as usize <= byte).saturating_sub(1)];
        cells as usize + UnicodeWidthStr::width(&self.line(row)[start as usize..byte])
    }
    fn byte_at_column(&self, row: usize, goal: usize) -> usize {
        let marks = &self.column_marks[self.column_ranges[row].clone()];
        if marks.is_empty() { return goal.min(self.line(row).len()); }
        let (start, cells) = marks[marks.partition_point(|(_, cells)| (*cells as usize) < goal).saturating_sub(1)];
        let line = self.line(row);
        let mut cells = cells as usize;
        for (byte, grapheme) in line[start as usize..].grapheme_indices(true) {
            let width = UnicodeWidthStr::width(grapheme);
            if cells >= goal || cells + width > goal { return start as usize + byte; }
            cells += width;
        }
        line.len()
    }
    fn rectangle(&self, row: usize, column: usize, anchor: Option<(usize, usize)>) -> (Range<usize>, Range<usize>) {
        let (other_row, other_column) = anchor.unwrap_or((row, column));
        let here = self.visual_column(row, column);
        let there = self.visual_column(other_row, other_column);
        let here_end = self.visual_column(row, next_grapheme(self.line(row), column));
        let there_end = self.visual_column(other_row, next_grapheme(self.line(other_row), other_column));
        (row.min(other_row)..row.max(other_row) + 1, here.min(there)..here_end.max(there_end))
    }
    fn rectangle_row(&self, row: usize, rows: &Range<usize>, columns: &Range<usize>) -> Range<usize> {
        if !rows.contains(&row) { return 0..0; }
        let start = self.byte_at_column(row, columns.start);
        let mut end = self.byte_at_column(row, columns.end);
        // Include a wide grapheme touched by the right edge rather than split it.
        if self.visual_column(row, end) < columns.end { end = next_grapheme(self.line(row), end); }
        self.lines[row].start + start..self.lines[row].start + end
    }
    fn rectangle_text(&self, row: usize, column: usize, anchor: Option<(usize, usize)>, current: impl Fn() -> bool) -> Option<String> {
        let (rows, columns) = self.rectangle(row, column, anchor);
        let mut text = String::new();
        for row in rows.clone() {
            if !current() { return None; }
            if row != rows.start { text.push('\n'); }
            text.push_str(&self.text[self.rectangle_row(row, &rows, &columns)]);
        }
        Some(text)
    }
    fn character_range(&self, row: usize, column: usize, anchor: Option<(usize, usize)>) -> Range<usize> {
        let here = self.lines[row].start + column;
        let (anchor_row, anchor_column) = anchor.unwrap_or((row, column));
        let there = self.lines[anchor_row].start + anchor_column;
        let (last_row, last_column) = if here >= there { (row, column) } else { (anchor_row, anchor_column) };
        here.min(there)..self.lines[last_row].start + next_grapheme(self.line(last_row), last_column)
    }
    fn selected(&self, cursor: usize, anchor: Option<usize>) -> &str {
        let anchor = anchor.unwrap_or(cursor);
        let first = cursor.min(anchor).min(self.lines.len() - 1);
        let last = cursor.max(anchor).min(self.lines.len() - 1);
        &self.text[self.lines[first].start..self.lines[last].end]
    }
}

fn next_grapheme(text: &str, offset: usize) -> usize {
    GraphemeCursor::new(offset, text.len(), true).next_boundary(text, 0).expect("complete grapheme context").unwrap_or(text.len())
}
fn previous_grapheme(text: &str, offset: usize) -> usize {
    GraphemeCursor::new(offset, text.len(), true).prev_boundary(text, 0).expect("complete grapheme context").unwrap_or(0)
}

pub(super) struct CopyMode {
    pub focus: FocusHandle,
    buffer: Option<Arc<Buffer>>,
    search: Option<Entity<text_field::TextField>>,
    query: String,
    buffer_name: Option<Entity<text_field::TextField>>,
    save_job: Option<Task<()>>,
    save_cancel: Arc<AtomicBool>,
    matches: Vec<usize>,
    search_message: String,
    search_cancel: Arc<AtomicBool>,
    search_job: Option<Task<()>>,
    message: String,
    copy_message: Option<String>,
    copy_job: Option<Task<()>>,
    cursor: usize,
    anchor: Option<usize>,
    characters: bool,
    rectangular: bool,
    column: usize,
    preferred_column: Option<usize>,
    character_anchor: Option<(usize, usize)>,
    scroll: ListState,
    _load: Task<()>,
}
impl Drop for CopyMode {
    fn drop(&mut self) { self.search_cancel.store(true, Ordering::Release); self.save_cancel.store(true, Ordering::Release); }
}
fn match_line(matches: &[usize], cursor: usize, reverse: bool, inclusive: bool) -> Option<usize> {
    if matches.is_empty() { return None; }
    if reverse {
        let end = matches.partition_point(|line| *line < cursor || (inclusive && *line == cursor));
        Some(matches[if end == 0 { matches.len() - 1 } else { end - 1 }])
    } else {
        let start = matches.partition_point(|line| *line < cursor || (!inclusive && *line == cursor));
        Some(matches[start % matches.len()])
    }
}
pub(super) struct Close;
impl EventEmitter<Close> for CopyMode {}
impl CopyMode {
    pub fn new(session: &Session, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let export = session.history_export(MAX_BYTES);
        let executor = cx.background_executor().clone();
        let load = cx.spawn(async move |entity, cx| {
            let result = executor.spawn(async move {
                export().map(|value| value.map(|(text, truncated)| Buffer::new(text, truncated)))
            }).await;
            let _ = entity.update(cx, |this, cx| {
                match result {
                    Ok(Some(buffer)) => {
                        this.cursor = buffer.lines.len().saturating_sub(1);
                        this.scroll.reset(buffer.lines.len());
                        crate::list_navigation::reveal(&this.scroll, this.cursor);
                        this.message = if buffer.truncated { "Snapshot truncated · First 8 MiB / 65,536 lines" } else { "Snapshot · Output continues in the terminal" }.into();
                        this.buffer = Some(Arc::new(buffer));
                    }
                    Ok(None) => this.message = "Terminal busy. Close and reopen copy mode to retry.".into(),
                    Err(error) => this.message = error.to_string(),
                }
                cx.notify();
            });
        });
        let focus = cx.focus_handle();
        focus.focus(window);
        Self { focus, buffer: None, search: None, query: String::new(), buffer_name: None, save_job: None, save_cancel: Arc::new(AtomicBool::new(false)), matches: Vec::new(),
            search_message: String::new(), search_cancel: Arc::new(AtomicBool::new(false)), search_job: None, copy_message: None, copy_job: None, message: "Reading terminal history…".into(), cursor: 0,
            anchor: None, characters: false, rectangular: false, column: 0, preferred_column: None, character_anchor: None, scroll: ListState::new(0, ListAlignment::Bottom, px(128.)), _load: load }
    }
    fn open_buffer_name(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.buffer.is_none() { self.copy_message = Some("No history snapshot is available yet".into()); cx.notify(); return; }
        self.search = None;
        if let Some(input) = &self.buffer_name { input.read(cx).focus.focus(window); return; }
        let input = cx.new(|cx| text_field::TextField::new("default", cx));
        cx.subscribe_in(&input, window, |this, _, event: &text_field::EditEvent, window, cx| {
            match event {
                text_field::EditEvent::Submitted { text, .. } => {
                    if let Err(error) = copy_buffers::validate_name(text) {
                        this.copy_message = Some(error.into());
                    } else {
                        this.save_buffer(text.clone(), cx);
                        this.buffer_name = None;
                        this.focus.focus(window);
                    }
                }
                text_field::EditEvent::Cancelled => { this.buffer_name = None; this.focus.focus(window); }
                text_field::EditEvent::Changed(_) => {},
            }
            cx.notify();
        }).detach();
        input.read(cx).focus.focus(window);
        self.buffer_name = Some(input);
        cx.notify();
    }
    fn save_buffer(&mut self, name: String, cx: &mut Context<Self>) {
        let Some(buffer) = self.buffer.clone() else { return; };
        self.save_cancel.store(true, Ordering::Release);
        self.save_job = None;
        self.save_cancel = Arc::new(AtomicBool::new(false));
        let canceled = self.save_cancel.clone();
        let revision = cx.global::<copy_buffers::Store>().revision();
        let (cursor, column, anchor, character_anchor, characters, rectangular) =
            (self.cursor, self.column, self.anchor, self.character_anchor, self.characters, self.rectangular);
        let executor = cx.background_executor().clone();
        self.copy_message = Some(format!("Saving buffer {name}…"));
        self.save_job = Some(cx.spawn(async move |entity, cx| {
            let result = executor.spawn(async move {
                if canceled.load(Ordering::Acquire) { return Ok(None); }
                let text = if rectangular {
                    buffer.rectangle_text(cursor, column, character_anchor, || !canceled.load(Ordering::Acquire))
                } else {
                    let text = if characters { &buffer.text[buffer.character_range(cursor, column, character_anchor)] }
                        else { buffer.selected(cursor, anchor) };
                    if text.len() > copy_buffers::MAX_BUFFER { return Err("Selection exceeds the 1 MiB buffer limit"); }
                    Some(text.to_owned())
                };
                if text.as_ref().is_some_and(|text| text.len() > copy_buffers::MAX_BUFFER) { return Err("Selection exceeds the 1 MiB buffer limit"); }
                Ok(text)
            }).await;
            let _ = entity.update(cx, |this, cx| {
                this.save_job = None;
                match result {
                    Ok(Some(text)) => {
                        let bytes = text.len();
                        this.copy_message = Some(match cx.global_mut::<copy_buffers::Store>().save_if_unchanged(name.clone(), text, revision) {
                            Ok(_) => format!("Saved {bytes} bytes to buffer {name}"),
                            Err(error) => error.into(),
                        });
                    }
                    Err(error) => this.copy_message = Some(error.into()),
                    Ok(None) => {},
                }
                cx.notify();
            });
        }));
        cx.notify();
    }
    fn open_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.buffer_name = None;
        if let Some(search) = &self.search { search.read(cx).focus.focus(window); return; }
        let search = cx.new(|cx| text_field::TextField::new(&self.query, cx));
        cx.subscribe_in(&search, window, |this, _, event: &text_field::EditEvent, window, cx| {
            match event {
                text_field::EditEvent::Submitted { text, reverse } => {
                    this.run_search(text.clone(), *reverse, cx);
                    this.focus.focus(window);
                }
                text_field::EditEvent::Cancelled => { this.search = None; this.focus.focus(window); }
                text_field::EditEvent::Changed(_) => {},
            }
            cx.notify();
        }).detach();
        search.read(cx).focus.focus(window);
        self.search = Some(search);
        cx.notify();
    }
    fn jump_match(&mut self, reverse: bool, inclusive: bool) {
        if let Some(index) = match_line(&self.matches, self.cursor, reverse, inclusive) {
            self.cursor = index;
            self.column = 0; self.preferred_column = None;
            self.character_anchor = None;
            self.anchor = None;
            crate::list_navigation::reveal(&self.scroll, index);
        }
    }
    fn run_search(&mut self, query: String, reverse: bool, cx: &mut Context<Self>) {
        let Some(buffer) = self.buffer.clone() else { return; };
        if query.len() > 4096 { self.search_message = "Search is limited to 4,096 bytes.".into(); return; }
        if query == self.query && self.search_job.is_none() && !query.is_empty() {
            self.jump_match(reverse, false);
            return;
        }
        self.search_cancel.store(true, Ordering::Release);
        self.search_job = None;
        self.search_cancel = Arc::new(AtomicBool::new(false));
        self.query = query.clone();
        self.matches.clear();
        if query.is_empty() { self.search_message.clear(); return; }
        self.search_message = "Searching snapshot…".into();
        let canceled = self.search_cancel.clone();
        let executor = cx.background_executor().clone();
        self.search_job = Some(cx.spawn(async move |entity, cx| {
            let result = executor.spawn(async move { buffer.find_lines(&query, &canceled) }).await;
            let _ = entity.update(cx, |this, cx| {
                this.search_job = None;
                if let Some(matches) = result {
                    this.search_message = format!("{} matching lines · Case-sensitive · n / Shift-N to navigate", matches.len());
                    this.matches = matches;
                    this.jump_match(reverse, true);
                    cx.notify();
                }
            });
        }));
    }
    fn copy(&mut self, cx: &mut Context<Self>) {
        let Some(buffer) = self.buffer.clone() else { return; };
        // Use the same application-wide ordering as terminal exports. A later
        // copy from another pane, field or window must win over this worker.
        let ticket = clipboard::begin();
        self.copy_job = None;
        let (cursor, column, anchor, character_anchor, characters, rectangular) =
            (self.cursor, self.column, self.anchor, self.character_anchor, self.characters, self.rectangular);
        let executor = cx.background_executor().clone();
        self.copy_message = Some("Copying selection…".into());
        self.copy_job = Some(cx.spawn(async move |entity, cx| {
            let text = executor.spawn(async move {
                if !clipboard::current(ticket) { return None; }
                if rectangular {
                    return buffer.rectangle_text(cursor, column, character_anchor, || clipboard::current(ticket));
                }
                let selected = if characters { &buffer.text[buffer.character_range(cursor, column, character_anchor)] }
                    else { buffer.selected(cursor, anchor) };
                let owned = selected.to_owned();
                clipboard::current(ticket).then_some(owned)
            }).await;
            let _ = entity.update(cx, |this, cx| {
                this.copy_job = None;
                if !clipboard::current(ticket) {
                    this.copy_message = Some("Copy superseded by a newer clipboard action".into());
                } else if let Some(text) = text {
                    if text.is_empty() { this.copy_message = Some("Selection is empty".into()); }
                    else {
                        this.copy_message = Some(format!("Copied {} bytes", text.len()));
                        clipboard::write(ClipboardItem::new_string(text), cx);
                    }
                }
                cx.notify();
            });
        }));
        cx.notify();
    }
    fn key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.focus.is_focused(window) { return; }
        let key = &event.keystroke;
        self.copy_message = None;
        if key.key == "escape" { cx.emit(Close); }
        else if !key.modifiers.platform && !key.modifiers.control && !key.modifiers.alt {
            let count = self.buffer.as_ref().map_or(0, |buffer| buffer.lines.len());
            if count > 0 {
                if matches!(key.key.as_str(), "left" | "right" | "h" | "l") {
                    if !self.characters { self.characters = true; self.anchor = None; }
                    if key.modifiers.shift && self.character_anchor.is_none() { self.character_anchor = Some((self.cursor, self.column)); }
                    let buffer = self.buffer.as_ref().unwrap();
                    let line = buffer.line(self.cursor);
                    self.column = if matches!(key.key.as_str(), "left" | "h") { previous_grapheme(line, self.column) } else { next_grapheme(line, self.column) };
                    self.preferred_column = None;
                    cx.stop_propagation(); cx.notify(); return;
                }
                let next = match key.key.as_str() {
                    "up" | "k" => Some(self.cursor.saturating_sub(1)),
                    "down" | "j" => Some((self.cursor + 1).min(count - 1)),
                    "pageup" | "pagedown" => command_palette::paged_result(self.cursor, count, key.key == "pageup",
                        f32::from(self.scroll.viewport_bounds().size.height), |index| {
                            self.scroll.bounds_for_item(index).map(|bounds| f32::from(bounds.size.height)).unwrap_or(22.)
                        }),
                    "home" => Some(0), "end" => Some(count - 1), _ => None,
                };
                if let Some(next) = next {
                    if self.characters {
                        if key.modifiers.shift && self.character_anchor.is_none() { self.character_anchor = Some((self.cursor, self.column)); }
                        let buffer = self.buffer.as_ref().unwrap();
                        let goal = *self.preferred_column.get_or_insert_with(|| buffer.visual_column(self.cursor, self.column));
                        self.column = buffer.byte_at_column(next, goal);
                    } else if key.modifiers.shift && self.anchor.is_none() { self.anchor = Some(self.cursor); }
                    self.cursor = next;
                    crate::list_navigation::reveal(&self.scroll, next);
                } else { match key.key.as_str() {
                    "/" => self.open_search(window, cx),
                    "b" => self.open_buffer_name(window, cx),
                    "n" | "N" => self.jump_match(key.modifiers.shift || key.key == "N", false),
                    "r" => { self.rectangular = !self.rectangular; self.characters = true; self.anchor = None; }
                    "c" => { self.rectangular = false; self.characters = !self.characters; self.anchor = None; self.character_anchor = None; self.column = 0; self.preferred_column = None; }
                    "space" | "v" => {
                        if self.characters { self.character_anchor = if self.character_anchor.is_some() { None } else { Some((self.cursor, self.column)) }; }
                        else { self.anchor = if self.anchor.is_some() { None } else { Some(self.cursor) }; }
                    },
                    "enter" | "y" => self.copy(cx),
                    _ => {},
                } }
            }
        }
        cx.stop_propagation();
        cx.notify();
    }
    fn row(&self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(buffer) = &self.buffer else { return div().into_any_element(); };
        let range = &buffer.lines[index];
        let text = &buffer.text[range.clone()];
        // A pathological single line must not cause unbounded shaping work.
        let mut start = if self.characters && index == self.cursor { self.column.saturating_sub(2048) } else { 0 };
        while !text.is_char_boundary(start) { start -= 1; }
        let mut end = text.len().min(start + 4096);
        while !text.is_char_boundary(end) { end -= 1; }
        let prefix = if start > 0 { "…" } else { "" };
        let suffix = if end < text.len() { "… [display clipped]" } else { "" };
        let display = format!("{prefix}{}{suffix}", &text[start..end]);
        let highlights = if self.characters {
            let selected = if self.rectangular {
                let (rows, columns) = buffer.rectangle(self.cursor, self.column, self.character_anchor);
                buffer.rectangle_row(index, &rows, &columns)
            } else { buffer.character_range(self.cursor, self.column, self.character_anchor) };
            let from = selected.start.max(range.start + start);
            let to = selected.end.min(range.start + end);
            (from < to).then(|| (from - range.start - start + prefix.len()..to - range.start - start + prefix.len(),
                HighlightStyle { background_color: Some(rgb(0x41638c).into()), ..Default::default() }))
        } else { None };
        let anchor = self.anchor.unwrap_or(self.cursor);
        let selected = (self.cursor.min(anchor)..=self.cursor.max(anchor)).contains(&index);
        div().id(("copy-line", index)).w_full().min_h(px(22.)).px_2().py_1().text_size(px(12.)).font_family(FONT_FAMILY)
            .when(selected && !self.characters, |row| row.bg(rgb(crate::chrome_style::SELECTED)))
            .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                if event.standard_click() {
                    this.copy_message = None;
                    this.characters = false; this.rectangular = false; this.character_anchor = None; this.column = 0; this.preferred_column = None;
                    if event.modifiers().shift && this.anchor.is_none() { this.anchor = Some(this.cursor); }
                    else if !event.modifiers().shift { this.anchor = None; }
                    this.cursor = index;
                    this.focus.focus(window);
                    cx.notify();
                }
            }))
            .flex().gap_2()
            .child(div().w(px(54.)).flex_shrink_0().text_color(rgb(0x909090)).child((index + 1).to_string()))
            .child(div().min_w_0().flex_1().child(StyledText::new(display).with_highlights(highlights))).into_any_element()
    }
}
impl Render for CopyMode {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        div().absolute().inset_0().size_full().occlude().bg(rgb(0x242424)).text_color(rgb(0xd6d6d6))
            .flex().flex_col().track_focus(&self.focus).key_context("CopyMode")
            .on_key_down(cx.listener(Self::key))
            .on_action(cx.listener(|this, _: &Find, window, cx| { this.open_search(window, cx); cx.stop_propagation(); }))
            .on_action(|_: &Paste, _, cx| cx.stop_propagation())
            .on_action(cx.listener(|this, _: &EditorSelectAll, _, cx| {
                if let Some(buffer) = &this.buffer {
                    this.characters = false; this.rectangular = false; this.character_anchor = None; this.column = 0; this.preferred_column = None;
                    this.anchor = Some(0);
                    this.cursor = buffer.lines.len() - 1;
                    crate::list_navigation::reveal(&this.scroll, this.cursor);
                    cx.notify();
                }
                cx.stop_propagation();
            }))
            .on_action(cx.listener(|this, _: &CopySelectionWithLineBreaks, _, cx| { this.copy(cx); cx.stop_propagation(); }))
            .on_action(cx.listener(|this, _: &CopySelection, _, cx| { this.copy(cx); cx.stop_propagation(); }))
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| { this.focus.focus(window); cx.stop_propagation(); }))
            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
            .on_mouse_down(MouseButton::Middle, |_, _, cx| cx.stop_propagation())
            .on_mouse_move(|_, _, cx| cx.stop_propagation())
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .child(div().px_3().py_2().flex().items_center().gap_2().border_b_1().border_color(rgb(0x383838))
                .child(div().flex_1().min_w_0().text_sm().child("Copy mode"))
                .when_some(self.buffer.as_ref(), |header, buffer| header.child(div().text_xs().text_color(rgb(0xa0a0a0))
                    .child(format!("Line {} / {}", self.cursor + 1, buffer.lines.len()))))
                .child(div().id("save-copy-buffer").px_2().py_1().rounded_sm().text_xs().cursor_pointer()
                    .bg(rgb(0x353535)).hover(|s| s.bg(rgb(0x454545)))
                    .on_click(cx.listener(|this, event: &ClickEvent, window, cx| { if event.standard_click() { this.open_buffer_name(window, cx); } }))
                    .child("Save Buffer…"))
                .child(pane_view::pane_control("close-copy-mode", "Close Copy Mode")
                    .on_click(cx.listener(|_, event: &ClickEvent, _, cx| { if event.standard_click() { cx.emit(Close); } }))
                    .child(icons::icon("close"))))
            .child(div().px_3().py_1().text_xs().child(self.message.clone()))
            .when_some(self.buffer_name.clone(), |root, input| root.child(div().px_3().py_2()
                .child(div().text_xs().mb_1().child("Buffer name · Enter saves/replaces · Esc cancels · Kept until app quits"))
                .child(input)))
            .when_some(self.search.clone(), |root, search| root.child(div().px_3().py_2()
                .child(div().text_xs().mb_1().child("Search snapshot · Enter to search · Shift-Enter previous"))
                .child(search)))
            .when(!self.search_message.is_empty(), |root| root.child(div().px_3().py_1().text_xs().child(self.search_message.clone())))
            .child(list(self.scroll.clone(), move |index, _, cx| entity.update(cx, |this, cx| this.row(index, cx)))
                .flex_1().min_h_0().w_full())
            .when_some(self.copy_message.clone(), |root, message| root.child(div().px_3().py_1().text_xs().text_color(rgb(0x9fceb0)).child(message)))
            .child(div().px_3().py_2().text_xs().border_t_1().border_color(rgb(0x383838))
                .child(if self.rectangular { "Rectangle mode · Arrows move · Shift / Space select · r characters · Enter copy · Esc close" }
                    else if self.characters { "Character mode · ←→ / h l move · Shift / Space select · r rectangle · c lines · Enter copy · Esc close" }
                    else { "Line mode · ↑↓ move · Shift / Space select · c characters · Enter copy · / search · Esc close" }))
    }
}

#[cfg(test)]
mod tests {
    use super::{Buffer, MAX_LINES, match_line, next_grapheme, previous_grapheme};
    use std::sync::atomic::AtomicBool;
    #[test]
    #[ignore = "explicit bounded copy-mode performance sample"]
    fn copy_mode_snapshot_costs() {
        use std::{hint::black_box, time::Instant, mem::size_of};
        for (name, text) in [
            ("ascii_long", "x".repeat(super::MAX_BYTES)),
            ("unicode_long", "界a".repeat(super::MAX_BYTES / 4)),
            ("line_limit", "x\n".repeat(super::MAX_BYTES / 2)),
        ] {
            let start = Instant::now();
            let buffer = Buffer::new(text, false);
            let build = start.elapsed();
            let start = Instant::now();
            for index in 0..10_000 {
                let row = index % buffer.lines.len();
                let goal = index * 7919 % buffer.line(row).len().max(1);
                let byte = buffer.byte_at_column(row, goal);
                black_box(buffer.visual_column(row, byte));
            }
            let allocated = buffer.text.capacity() + buffer.lines.capacity() * size_of::<std::ops::Range<usize>>()
                + buffer.column_ranges.capacity() * size_of::<std::ops::Range<usize>>()
                + buffer.column_marks.capacity() * size_of::<(u32, u32)>();
            println!("{{\"case\":\"{name}\",\"build_ms\":{:.3},\"lookup_10k_ms\":{:.3},\"allocated_bytes\":{allocated},\"retained_bytes\":{},\"lines\":{}}}",
                build.as_secs_f64() * 1000., start.elapsed().as_secs_f64() * 1000., buffer.text.len(), buffer.lines.len());
        }
    }
    #[test]
    fn snapshot_search_matches_unicode_lines_and_wraps_in_both_directions() {
        let buffer = Buffer::new("世界\nHello 世界\nhello\n世界".into(), false);
        let matches = buffer.find_lines("世界", &AtomicBool::new(false)).unwrap();
        assert_eq!(matches, vec![0, 1, 3]);
        assert_eq!(buffer.find_lines("Hello", &AtomicBool::new(false)).unwrap(), vec![1]);
        assert!(buffer.find_lines("世界", &AtomicBool::new(true)).is_none());
        assert_eq!(match_line(&matches, 3, false, false), Some(0));
        assert_eq!(match_line(&matches, 0, true, false), Some(3));
        assert_eq!(match_line(&matches, 1, false, true), Some(1));
        assert_eq!(match_line(&matches, 2, true, true), Some(1));
        assert_eq!(match_line(&[], 0, false, false), None);
    }
    #[test]
    fn ascii_fast_path_excludes_controls_and_releases_discarded_text() {
        let buffer = Buffer::new("hello\n\tworld\n".into(), false);
        assert!(buffer.column_ranges[0].is_empty());
        assert!(!buffer.column_ranges[1].is_empty());
        assert_eq!(buffer.visual_column(0, 3), 3);
        assert_eq!(buffer.byte_at_column(0, 99), 5);
        let limited = Buffer::new("x\n".repeat(MAX_LINES * 4), false);
        assert_eq!(limited.text.capacity(), limited.text.len());
    }
    #[test]
    fn vertical_columns_preserve_width_across_unicode_short_and_long_lines() {
        let buffer = Buffer::new("a界z\n👩‍💻e\u{301}x\nx\nabcdef".into(), false);
        assert_eq!(buffer.visual_column(0, "a界".len()), 3);
        assert_eq!(buffer.byte_at_column(1, 3), "👩‍💻e\u{301}".len());
        assert_eq!(buffer.byte_at_column(1, 1), 0, "inside wide grapheme selects its start");
        assert_eq!(buffer.byte_at_column(2, 3), 1);
        assert_eq!(buffer.byte_at_column(3, 3), 3, "original goal survives a short line");
        let long = Buffer::new("界a".repeat(10_000), false);
        for column in [0, 255, 256, 257, 9999, 29_999, 30_000] {
            let byte = long.byte_at_column(0, column);
            let actual = long.visual_column(0, byte);
            assert!(actual <= column && column - actual <= 1);
        }
        assert!(long.column_marks.len() < 200, "index must stay sparse");
    }
    #[test]
    fn rectangle_selection_uses_cell_columns_without_splitting_graphemes() {
        let buffer = Buffer::new("abcdef\na界de\nx\n👩‍💻e\u{301}xyz".into(), false);
        // Inclusive cells 1..3 touch CJK and emoji boundaries; short rows remain empty.
        let selected = buffer.rectangle_text(0, 1, Some((3, "👩‍💻e\u{301}".len())), || true).unwrap();
        assert_eq!(selected, "bcd\n界d\n\n👩‍💻e\u{301}x");
        assert_eq!(selected, buffer.rectangle_text(3, "👩‍💻e\u{301}".len(), Some((0, 1)), || true).unwrap());
        assert!(buffer.rectangle_text(0, 1, Some((3, 0)), || false).is_none());
        let empty = Buffer::new(String::new(), false);
        assert_eq!(empty.rectangle_text(0, 0, None, || true).unwrap(), "");
    }
    #[test]
    fn character_selection_preserves_graphemes_across_lines_and_reverse_ranges() {
        let text = "a👩‍💻e\u{301}界";
        let woman_end = next_grapheme(text, 1);
        assert_eq!(&text[1..woman_end], "👩‍💻");
        let accent_end = next_grapheme(text, woman_end);
        assert_eq!(&text[woman_end..accent_end], "e\u{301}");
        assert_eq!(previous_grapheme(text, accent_end), woman_end);
        let buffer = Buffer::new(format!("{text}\n界"), false);
        let range = buffer.character_range(0, 1, Some((1, 0)));
        assert_eq!(&buffer.text[range.clone()], "👩‍💻e\u{301}界\n界");
        assert_eq!(range, buffer.character_range(1, 0, Some((0, 1))));
        assert_eq!(&buffer.text[buffer.character_range(0, woman_end, None)], "e\u{301}");
    }
    #[test]
    fn snapshot_selection_keeps_unicode_empty_lines_and_reverse_ranges() {
        let buffer = Buffer::new("first 世界\n\nlast 👩‍💻".into(), false);
        assert_eq!(buffer.selected(2, Some(0)), "first 世界\n\nlast 👩‍💻");
        assert_eq!(buffer.selected(0, Some(2)), buffer.selected(2, Some(0)));
        assert_eq!(buffer.selected(1, None), "");
        assert_eq!(Buffer::new(String::new(), false).selected(0, None), "");
    }
    #[test]
    fn snapshot_line_limit_discards_uncopyable_tail() {
        let buffer = Buffer::new("x\n".repeat(MAX_LINES + 1), false);
        assert!(buffer.truncated);
        assert_eq!(buffer.lines.len(), MAX_LINES);
        assert_eq!(buffer.text.len(), MAX_LINES * 2);
        assert!(buffer.column_marks.is_empty());
        assert_eq!(buffer.selected(MAX_LINES - 1, None), "x");
    }
}
