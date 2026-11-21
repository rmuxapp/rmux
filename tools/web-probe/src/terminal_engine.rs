//! Browser Ghostty adapter. Sparse patches retain unchanged row Arc identities.
use rmux_core::terminal_frame::{Cell, Color, CursorShape, Frame};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = focus)]
    fn focus(id: u32, active: bool);
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = isMac)]
    pub fn is_mac() -> bool;
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = create)]
    fn create(columns: u16, rows: u16) -> u32;
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = write)]
    fn write(id: u32, text: &str);
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = resize)]
    fn resize(id: u32, columns: u16, rows: u16);
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = snapshot)]
    fn snapshot(id: u32) -> String;
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = free)]
    fn free(id: u32);
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = encode)]
    fn encode(id: u32, key: &str, mods: u8, repeat: bool, text: &str) -> String;
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = select)]
    fn select(id: u32, action: &str, x: f32, y: f32, cell_width: f32, cell_height: f32, columns: u16, rows: usize, rectangle: bool, clicks: usize);
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = selectedText)]
    fn selected_text(id: u32) -> Option<String>;
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
pub struct TerminalEngine(u32);
impl TerminalEngine {
    pub fn new(columns: u16, rows: u16) -> Self { Self(create(columns, rows)) }
    pub fn write(&self, text: &str) { write(self.0, text); }
    pub fn selected_text(&self) -> Option<String> { selected_text(self.0) }
    pub fn focus(&self, active: bool) { focus(self.0, active); }
    pub fn scroll(&self, kind: &str, delta: i32) { scroll(self.0, kind, delta); }
    pub fn select(&self, action: &str, x: f32, y: f32, cells: crate::cell_metrics::CellMetrics, frame: &Frame, rectangle: bool, clicks: usize) {
        select(self.0, action, x, y, cells.width, cells.height, frame.columns, frame.rows.len(), rectangle, clicks);
    }
    pub fn encode(&self, key: rmux_core::keyboard::Key, mods: rmux_core::keyboard::Modifiers, repeat: bool, text: &str) -> String {
        let bits = u8::from(mods.shift) | (u8::from(mods.control)<<1) | (u8::from(mods.alt)<<2);
        encode(self.0, &crate::key_adapter::name(key), bits, repeat, text)
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
