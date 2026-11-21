//! Bounded input editing for bundled sample sessions, not a real shell.
use crate::terminal_text::Grapheme;
pub const MAX_INPUT_BYTES: usize = 4096;

#[derive(Default)]
pub struct SampleInput {
    text: String,
    graphemes: Vec<Grapheme>,
}
impl SampleInput {
    pub fn text(&self) -> &str { &self.text }
    pub fn clear(&mut self) { self.text.clear(); self.graphemes.clear(); }
    pub fn append(&mut self, text: &str, measure: impl FnOnce(&str) -> Option<Vec<Grapheme>>) -> Result<(), &'static str> {
        if self.text.len().saturating_add(text.len()) > MAX_INPUT_BYTES { return Err("Sample input is limited to 4 KiB."); }
        if text.chars().any(char::is_control) { return Err("Sample input must be a single line."); }
        let mut next = self.text.clone();
        next.push_str(text);
        // Remeasure the whole draft: combining marks and ZWJ sequences can
        // extend the previously final cluster across separate input events.
        let graphemes = measure(&next).ok_or("Could not measure sample input.")?;
        let mut previous = 0;
        for grapheme in &graphemes {
            if grapheme.end <= previous || !next.is_char_boundary(grapheme.end) || grapheme.width > 2 {
                return Err("Invalid sample input measurement.");
            }
            previous = grapheme.end;
        }
        if previous != next.len() { return Err("Incomplete sample input measurement."); }
        self.text = next;
        self.graphemes = graphemes;
        Ok(())
    }
    pub fn backspace(&mut self) {
        self.graphemes.pop();
        self.text.truncate(self.graphemes.last().map_or(0, |cluster| cluster.end));
    }
    /// Keep the editable draft on one row, leaving a cell for the cursor.
    /// Narrow panes show its suffix, always on complete grapheme boundaries.
    pub fn display(&self, columns: u16) -> String {
        let available = usize::from(columns.saturating_sub(1));
        let prompt = if available >= 2 { "$ " } else if available == 1 { "$" } else { "" };
        let budget = available.saturating_sub(prompt.len());
        let mut width = 0;
        let mut start = self.text.len();
        for (index, grapheme) in self.graphemes.iter().enumerate().rev() {
            if width + usize::from(grapheme.width) > budget { break; }
            width += usize::from(grapheme.width);
            start = index.checked_sub(1).map_or(0, |previous| self.graphemes[previous].end);
        }
        format!("\r\x1b[2K{prompt}{}", &self.text[start..])
    }
}
