//! Owned terminal render data, independent of the engine, PTY and UI backend.
//! A host translates its VT engine output into these snapshots. Unchanged rows
//! retain Arc identity so the renderer can reuse shaping without copying cells.
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CursorShape {
    Bar,
    Block,
    Underline,
    Hollow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Color(pub u8, pub u8, pub u8);
pub const DEFAULT_FOREGROUND: Color = Color(220, 223, 230);
pub const DEFAULT_BACKGROUND: Color = Color(18, 20, 26);
pub const DEFAULT_CURSOR: Color = Color(140, 180, 255);

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Cell {
    pub text: smol_str::SmolStr,
    pub column: u16,
    pub width: u8,
    pub foreground: Color,
    pub background: Color,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub underline_color: Color,
    pub underline_curly: bool,
    pub strikethrough: bool,
    pub overline: bool,
    pub hidden: bool,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Frame {
    pub revision: u64,
    pub selections: Arc<Vec<Option<(u16, u16)>>>,
    pub columns: u16,
    pub rows: Arc<Vec<Arc<Vec<Cell>>>>,
    pub background: Color,
    pub cursor: Option<(u16, u16)>,
    pub cursor_shape: CursorShape,
    pub cursor_color: Color,
    /// True when the engine has a configured or application-provided cursor color.
    pub cursor_color_explicit: bool,
    pub scrollback_lines: u64,
    pub scrollback_offset: u64,
}
impl Frame {
    /// Logical storage represented by this snapshot. Shared allocations are not
    /// deduplicated across frames; this excludes allocator/Arc and GPU overhead.
    pub fn storage(&self) -> FrameStorage {
        let mut storage = FrameStorage::default();
        for row in self.rows.iter() {
            storage.cell_capacity_bytes += row.capacity() * size_of::<Cell>();
            for cell in row.iter() {
                storage.text_bytes += cell.text.len();
                if cell.text.is_heap_allocated() {
                    storage.heap_cells += 1;
                    storage.heap_text_payload_bytes += cell.text.len();
                } else {
                    storage.inline_cells += 1;
                }
            }
        }
        storage
    }
    pub fn text(&self) -> String {
        self.text_with_limit(usize::MAX).0
    }
    /// Visible viewport text, trimming row endings and preserving UTF-8 boundaries.
    /// Stops constructing output at the limit rather than allocating then truncating.
    pub fn text_with_limit(&self, limit: usize) -> (String, bool) {
        fn append(output: &mut String, text: &str, limit: usize) -> bool {
            let mut end = text.len().min(limit.saturating_sub(output.len()));
            while !text.is_char_boundary(end) { end -= 1; }
            output.push_str(&text[..end]);
            end == text.len()
        }
        let mut output = String::new();
        for (index, row) in self.rows.iter().enumerate() {
            if index != 0 && !append(&mut output, "\n", limit) { return (output, true); }
            let Some(last) = row.iter().rposition(|cell| cell.width != 0 && !cell.text.trim_end().is_empty()) else { continue; };
            for (column, cell) in row[..=last].iter().enumerate() {
                if cell.width == 0 { continue; }
                let text = if cell.text.is_empty() { " " } else if column == last { cell.text.trim_end() } else { &cell.text };
                if !append(&mut output, text, limit) { return (output, true); }
            }
        }
        (output, false)
    }
}

#[derive(Default, Debug, PartialEq, Eq)]
pub struct FrameStorage {
    pub cell_capacity_bytes: usize,
    pub text_bytes: usize,
    pub inline_cells: usize,
    pub heap_cells: usize,
    pub heap_text_payload_bytes: usize,
}

/// Selection paint policy, without preference-file parsing or platform services.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellPaint { Literal(Color), CellForeground, CellBackground }
impl CellPaint {
    pub fn resolve(self, foreground: Color, background: Color) -> Color {
        match self { Self::Literal(color) => color, Self::CellForeground => foreground, Self::CellBackground => background }
    }
}
