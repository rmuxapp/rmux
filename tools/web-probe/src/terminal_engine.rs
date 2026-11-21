//! Browser Ghostty adapter. Sparse patches retain unchanged row Arc identities.
use rmux_core::terminal_frame::{Cell, Color, CursorShape, Frame};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = rmuxEngine, js_name = searchQuery)]
    fn search_query(id: u32, query: &str) -> Result<String, JsValue>;
    #[wasm_bindgen(catch, js_namespace = rmuxEngine, js_name = searchStep)]
    fn search_step(id: u32) -> Result<String, JsValue>;
    #[wasm_bindgen(catch, js_namespace = rmuxEngine, js_name = searchRefresh)]
    fn search_refresh(id: u32) -> Result<String, JsValue>;
    #[wasm_bindgen(catch, js_namespace = rmuxEngine, js_name = searchNavigate)]
    fn search_navigate(id: u32, older: bool) -> Result<String, JsValue>;
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = searchClose)]
    fn search_close(id: u32);
    #[wasm_bindgen(catch, js_namespace = rmuxEngine, js_name = graphemes)]
    fn measure_graphemes(text: &str) -> Result<String, JsValue>;
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = focus)]
    fn focus(id: u32, active: bool);
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = isMac)]
    pub fn is_mac() -> bool;
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = create)]
    fn create(columns: u16, rows: u16, foreground: u32, background: u32, scrollback: u32) -> u32;
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = write)]
    fn write(id: u32, text: &str);
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = resize)]
    fn resize(id: u32, columns: u16, rows: u16);
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = snapshot)]
    fn snapshot(id: u32) -> String;
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = free)]
    fn free(id: u32);
    #[wasm_bindgen(catch, js_namespace = rmuxEngine, js_name = encodePaste)]
    fn encode_paste(id: u32, text: &str) -> Result<String, JsValue>;
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = encode)]
    fn encode(id: u32, key: &str, mods: u8, action: &str, text: &str) -> String;
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = select)]
    fn select(id: u32, action: &str, x: f32, y: f32, cell_width: f32, cell_height: f32, columns: u16, rows: usize, rectangle: bool, clicks: usize) -> bool;
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = selectedText, catch)]
    fn selected_text(id: u32, unwrap: bool) -> Result<Option<String>, JsValue>;
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = selectAll, catch)]
    fn select_all(id: u32) -> Result<(), JsValue>;
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = clearScrollback, catch)]
    fn clear_scrollback(id: u32) -> Result<(), JsValue>;
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = selectedSearchQuery, catch)]
    fn selected_search_query(id: u32) -> Result<Option<String>, JsValue>;
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = scroll)]
    fn scroll(id: u32, kind: &str, delta: i32);
}
#[derive(serde::Deserialize)]
struct WireCell(smol_str::SmolStr, u8, Color, Color, u8, Color, bool, bool);
impl WireCell {
    fn into_cell(self, column: usize) -> Cell {
        let Self(text, width, foreground, background, flags, underline_color, underline_curly, overline) = self;
        Cell { text, column: column as u16, width, foreground, background,
            bold: flags & 1 != 0, italic: flags & 2 != 0, underline: flags & 4 != 0,
            strikethrough: flags & 8 != 0, hidden: flags & 32 != 0,
            underline_color, underline_curly, overline }
    }
}
#[derive(serde::Deserialize)]
struct Patch {
    revision: u64, columns: u16, height: usize, rows: Vec<(usize, Vec<WireCell>)>,
    background: Color, cursor: Option<(u16,u16)>, cursor_shape: CursorShape,
    cursor_color: Color, cursor_color_explicit: bool, scrollback_lines: u64,
    selections: Vec<(usize, Option<(u16,u16)>)>, scrollback_offset: u64,
}
#[derive(serde::Deserialize)]
pub struct SearchProgress { pub matches: usize, pub selected: Option<usize>, pub pending: bool }
fn search_result(result: Result<String, JsValue>) -> Result<SearchProgress, String> {
    let json = result.map_err(|error| format!("Search failed: {error:?}"))?;
    serde_json::from_str(&json).map_err(|error| error.to_string())
}
pub struct TerminalEngine(u32);
impl TerminalEngine {
    pub fn selected_text_with_wraps(&self) -> Result<Option<String>, JsValue> { selected_text(self.0, false) }
    pub fn select_all(&self) -> Result<(), JsValue> { select_all(self.0) }
    pub fn clear_scrollback(&self) -> Result<(), JsValue> { clear_scrollback(self.0) }
    pub fn search_query(&self, query: &str) -> Result<SearchProgress, String> { search_result(search_query(self.0, query)) }
    pub fn search_step(&self) -> Result<SearchProgress, String> { search_result(search_step(self.0)) }
    pub fn search_refresh(&self) -> Result<SearchProgress, String> { search_result(search_refresh(self.0)) }
    pub fn search_navigate(&self, older: bool) -> Result<SearchProgress, String> { search_result(search_navigate(self.0, older)) }
    pub fn search_close(&self) { search_close(self.0); }

    pub fn new(columns: u16, rows: u16) -> Self {
        use rmux_core::terminal_frame::{DEFAULT_FOREGROUND, DEFAULT_BACKGROUND};
        let rgb = |Color(r,g,b)| (u32::from(r)<<16) | (u32::from(g)<<8) | u32::from(b);
        Self(create(columns, rows, rgb(DEFAULT_FOREGROUND), rgb(DEFAULT_BACKGROUND), rmux_core::scroll::DEFAULT_SCROLLBACK_BYTES as u32))
    }
    pub fn encode_paste(&self, text: &str) -> Result<String, JsValue> { encode_paste(self.0, text) }
    pub fn write(&self, text: &str) { write(self.0, text); }
    pub fn selected_search_query(&self) -> Option<String> { selected_search_query(self.0).ok().flatten() }
    pub fn selected_text(&self) -> Result<Option<String>, JsValue> { selected_text(self.0, true) }
    pub fn focus(&self, active: bool) { focus(self.0, active); }
    pub fn scroll(&self, kind: &str, delta: i32) { scroll(self.0, kind, delta); }
    pub fn select(&self, action: &str, x: f32, y: f32, cells: crate::cell_metrics::CellMetrics, frame: &Frame, rectangle: bool, clicks: usize) -> bool {
        select(self.0, action, x, y, cells.width, cells.height, frame.columns, frame.rows.len(), rectangle, clicks)
    }
    pub fn encode(&self, key: rmux_core::keyboard::Key, mods: rmux_core::keyboard::Modifiers, action: rmux_core::keyboard::Action, text: &str) -> String {
        let bits = u8::from(mods.shift) | (u8::from(mods.control)<<1) | (u8::from(mods.alt)<<2);
        let action = match action { rmux_core::keyboard::Action::Press => "PRESS", rmux_core::keyboard::Action::Repeat => "REPEAT", rmux_core::keyboard::Action::Release => "RELEASE" };
        encode(self.0, &crate::key_adapter::name(key), bits, action, text)
    }
    pub fn resize(&self, columns: u16, rows: u16) { resize(self.0, columns, rows); }
    pub fn frame(&self, previous: Option<&Frame>) -> Frame {
        let patch: Patch = serde_json::from_str(&snapshot(self.0)).expect("browser Ghostty frame");
        let mut rows = previous.map(|f| f.rows.as_ref().clone()).unwrap_or_default();
        rows.resize_with(patch.height, || Arc::new(Vec::new()));
        let mut selections = previous.map(|f| f.selections.as_ref().clone()).unwrap_or_default();
        selections.resize(patch.height, None);
        for (y, selection) in patch.selections { selections[y] = selection; }
        for (y, cells) in patch.rows {
            rows[y] = Arc::new(cells.into_iter().enumerate().map(|(x, cell)| cell.into_cell(x)).collect());
        }
        Frame { revision: patch.revision, columns: patch.columns, rows: Arc::new(rows),
            selections: Arc::new(selections), background: patch.background,
            cursor: patch.cursor, cursor_shape: patch.cursor_shape, cursor_color: patch.cursor_color,
            cursor_color_explicit: patch.cursor_color_explicit, scrollback_lines: patch.scrollback_lines,
            scrollback_offset: patch.scrollback_offset }
    }
}
impl Drop for TerminalEngine { fn drop(&mut self) { free(self.0); } }

pub fn graphemes(text: &str) -> Result<Vec<rmux_core::terminal_text::Grapheme>, JsValue> {
    serde_json::from_str(&measure_graphemes(text)?).map_err(|error| JsValue::from_str(&error.to_string()))
}
