//! Bounded input editing for bundled sample sessions, not a real shell.
use crate::terminal_text::Grapheme;
pub const MAX_INPUT_BYTES: usize = 4096;

#[derive(Clone, Default)]
pub struct SampleInput {
    text: String,
    graphemes: Vec<Grapheme>,
    cursor: usize,
}
impl SampleInput {
    pub fn text(&self) -> &str { &self.text }
    pub fn clear(&mut self) { self.text.clear(); self.graphemes.clear(); self.cursor = 0; }
    pub fn append(&mut self, text: &str, measure: impl FnOnce(&str) -> Option<Vec<Grapheme>>) -> Result<(), &'static str> {
        if self.text.len().saturating_add(text.len()) > MAX_INPUT_BYTES { return Err("Sample input is limited to 4 KiB."); }
        if text.chars().any(char::is_control) { return Err("Sample input must be a single line."); }
        // Printable ASCII is one byte, one cell and one grapheme. Keep this
        // common path in Rust: no draft clone or engine/JSON round trip. A
        // non-ASCII draft still uses the engine, since its last cluster may
        // combine with later input (including an ASCII character).
        if self.cursor == self.text.len() && self.text.is_ascii() && text.is_ascii() {
            let start = self.text.len();
            self.text.push_str(text);
            self.graphemes.extend((start + 1..=self.text.len()).map(|end| Grapheme { end, width: 1 }));
            self.cursor = self.text.len();
            return Ok(());
        }
        self.replace(self.cursor..self.cursor, text, measure)
    }
    fn replace(&mut self, range: std::ops::Range<usize>, text: &str,
        measure: impl FnOnce(&str) -> Option<Vec<Grapheme>>) -> Result<(), &'static str> {
        let mut next = self.text.clone();
        let cursor = range.start + text.len();
        next.replace_range(range, text);
        // Remeasure the whole draft: combining marks and ZWJ sequences can
        // extend the previously final cluster across separate input events.
        let graphemes = if next.is_ascii() {
            (1..=next.len()).map(|end| Grapheme { end, width: 1 }).collect()
        } else { measure(&next).ok_or("Could not measure sample input.")? };
        let mut previous = 0;
        for grapheme in &graphemes {
            if grapheme.end <= previous || !next.is_char_boundary(grapheme.end) || grapheme.width > 2 {
                return Err("Invalid sample input measurement.");
            }
            previous = grapheme.end;
        }
        if previous != next.len() { return Err("Incomplete sample input measurement."); }
        self.text = next;
        self.cursor = if cursor == 0 { 0 } else { graphemes.iter().find(|g| g.end >= cursor).map_or(self.text.len(), |g| g.end) };
        self.graphemes = graphemes;
        Ok(())
    }
    fn boundary_before(&self, position: usize) -> usize {
        self.graphemes.iter().take_while(|g| g.end < position).last().map_or(0, |g| g.end)
    }
    fn boundary_after(&self, position: usize) -> usize {
        self.graphemes.iter().find(|g| g.end > position).map_or(self.text.len(), |g| g.end)
    }
    pub fn move_cursor(&mut self, backward: bool, word: bool) {
        if !word {
            self.cursor = if backward { self.boundary_before(self.cursor) } else { self.boundary_after(self.cursor) };
            return;
        }
        let mut in_word = false;
        loop {
            let next = if backward { self.boundary_before(self.cursor) } else { self.boundary_after(self.cursor) };
            if next == self.cursor { break; }
            let span = &self.text[self.cursor.min(next)..self.cursor.max(next)];
            let word = span.chars().any(|c| c.is_alphanumeric() || c == '_');
            if in_word && !word { break; }
            in_word |= word;
            self.cursor = next;
        }
    }
    pub fn edge(&mut self, end: bool) { self.cursor = if end { self.text.len() } else { 0 }; }
    pub fn delete(&mut self, backward: bool, measure: impl FnOnce(&str) -> Option<Vec<Grapheme>>) -> Result<(), &'static str> {
        let range = if backward { self.boundary_before(self.cursor)..self.cursor }
            else { self.cursor..self.boundary_after(self.cursor) };
        if range.is_empty() { return Ok(()); }
        self.replace(range, "", measure)
    }
    pub fn erase_to_edge(&mut self, end: bool, measure: impl FnOnce(&str) -> Option<Vec<Grapheme>>) -> Result<(), &'static str> {
        let range = if end { self.cursor..self.text.len() } else { 0..self.cursor };
        if range.is_empty() { return Ok(()); }
        self.replace(range, "", measure)
    }
    /// Keep the editable draft on one row, leaving a cell for the cursor.
    /// Narrow panes follow the cursor on complete grapheme boundaries.
    pub fn display(&self, columns: u16) -> String {
        let available = usize::from(columns.saturating_sub(1));
        let prompt = if available >= 2 { "$ " } else if available == 1 { "$" } else { "" };
        let budget = available.saturating_sub(prompt.len());
        let mut width = 0;
        let mut start = self.cursor;
        for (index, grapheme) in self.graphemes.iter().enumerate().rev().filter(|(_, g)| g.end <= self.cursor) {
            if width + usize::from(grapheme.width) > budget { break; }
            width += usize::from(grapheme.width);
            start = index.checked_sub(1).map_or(0, |previous| self.graphemes[previous].end);
        }
        let mut end = self.cursor;
        let mut tail_width = 0;
        for grapheme in self.graphemes.iter().filter(|g| g.end > self.cursor) {
            if width + usize::from(grapheme.width) > budget { break; }
            width += usize::from(grapheme.width);
            tail_width += usize::from(grapheme.width);
            end = grapheme.end;
        }
        let mut output = format!("\r\x1b[2K{prompt}{}", &self.text[start..end]);
        if tail_width > 0 { output.push_str(&format!("\x1b[{tail_width}D")); }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn measure(text: &str) -> Option<Vec<Grapheme>> {
        use unicode_segmentation::UnicodeSegmentation;
        Some(text.grapheme_indices(true).map(|(offset, g)| Grapheme {
            end: offset + g.len(), width: if g.contains('界') { 2 } else { 1 },
        }).collect())
    }
    #[test]
    fn unicode_cursor_edits_stay_on_clusters_and_scroll_with_the_cursor() {
        let mut input = SampleInput::default();
        input.append("界e\u{301}z", measure).unwrap();
        input.edge(false);
        input.move_cursor(false, false);
        assert_eq!(input.display(6), "\r\x1b[2K$ 界e\u{301}\x1b[1D");
        input.delete(false, measure).unwrap();
        assert_eq!(input.text(), "界z");
        input.append("a", measure).unwrap();
        assert_eq!(input.text(), "界az");
        let before = input.display(5);
        assert!(input.append("界", |_| None).is_err());
        assert_eq!(input.display(5), before);
        input.delete(true, measure).unwrap();
        input.delete(true, measure).unwrap();
        assert_eq!(input.text(), "z");
        assert_eq!(input.display(4), "\r\x1b[2K$ z\x1b[1D");
    }
    #[test]
    fn ascii_typing_avoids_engine_measurement_and_respects_bounds() {
        let no_engine = |_: &str| -> Option<Vec<Grapheme>> { panic!("ASCII invoked the engine") };
        let mut input = SampleInput::default();
        for _ in 0..MAX_INPUT_BYTES { input.append("x", no_engine).unwrap(); }
        assert_eq!(input.text().len(), MAX_INPUT_BYTES);
        assert!(input.append("x", no_engine).is_err());
        assert!(input.append("\n", no_engine).is_err());
        input.delete(true, no_engine).unwrap();
        input.append("y", no_engine).unwrap();
        assert_eq!(input.display(5), "\r\x1b[2K$ xy");
        input.clear();
        input.append("", no_engine).unwrap();
        assert_eq!(input.text(), "");
    }
}
