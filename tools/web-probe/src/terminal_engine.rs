//! Browser Ghostty adapter. Sparse patches retain unchanged row Arc identities.
use rmux_core::terminal_frame::{Cell, Color, CursorShape, Frame};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = create)]
    fn create(columns: u16, rows: u16) -> u32;
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = write)]
    fn write(id: u32, text: &str);
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = snapshot)]
    fn snapshot(id: u32) -> String;
    #[wasm_bindgen(js_namespace = rmuxEngine, js_name = free)]
    fn free(id: u32);
}
#[derive(serde::Deserialize)]
struct Patch {
    revision: u64, columns: u16, height: usize, rows: Vec<(usize, Vec<Cell>)>,
    background: Color, cursor: Option<(u16,u16)>, cursor_shape: CursorShape,
    cursor_color: Color, cursor_color_explicit: bool, scrollback_lines: u64,
}
pub struct TerminalEngine(u32);
impl TerminalEngine {
    pub fn new(columns: u16, rows: u16) -> Self { Self(create(columns, rows)) }
    pub fn write(&self, text: &str) { write(self.0, text); }
    pub fn frame(&self, previous: Option<&Frame>) -> Frame {
        let patch: Patch = serde_json::from_str(&snapshot(self.0)).expect("browser Ghostty frame");
        let mut rows = previous.map(|f| f.rows.as_ref().clone()).unwrap_or_default();
        rows.resize_with(patch.height, || Arc::new(Vec::new()));
        for (y, cells) in patch.rows { rows[y] = Arc::new(cells); }
        Frame { revision: patch.revision, columns: patch.columns, rows: Arc::new(rows),
            selections: Arc::new(vec![None; patch.height]), background: patch.background,
            cursor: patch.cursor, cursor_shape: patch.cursor_shape, cursor_color: patch.cursor_color,
            cursor_color_explicit: patch.cursor_color_explicit, scrollback_lines: patch.scrollback_lines,
            scrollback_offset: 0 }
    }
}
impl Drop for TerminalEngine { fn drop(&mut self) { free(self.0); } }
