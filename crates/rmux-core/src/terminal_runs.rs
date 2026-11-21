//! Terminal glyph grouping shared by rendering backends.
//! Maintains column boundaries, style changes and non-ASCII cluster isolation.
use crate::terminal_frame::Cell;

/// Prepare one style run. `start` must identify an existing cell. Background
/// changes split the run when selected text derives its color from backgrounds.
pub fn text_run(source: &[Cell], start: usize, foreground_from_background: bool) -> (String, usize) {
    let cell = &source[start];
    if cell.hidden {
        return (String::new(), start + 1);
    }
    let mut end = start + 1;
    let mut bytes = cell.text.len().max(1);
    if cell.text.is_ascii() {
        while let Some(next) = source.get(end) {
            if next.hidden
                || next.width != 1
                || next.column != source[end - 1].column + 1
                || !next.text.is_ascii()
                || next.foreground != cell.foreground
                || (foreground_from_background && next.background != cell.background)
                || next.bold != cell.bold
                || next.italic != cell.italic
                || next.underline != cell.underline
                || next.underline_color != cell.underline_color
                || next.underline_curly != cell.underline_curly
                || next.strikethrough != cell.strikethrough
            {
                break;
            }
            bytes += next.text.len().max(1);
            end += 1;
        }
    }
    // Determine the style run before allocating, avoiding geometric growth for
    // long ASCII rows without over-reserving for short, differently styled runs.
    let mut text = String::with_capacity(bytes);
    for cell in &source[start..end] {
        text.push_str(if cell.text.is_empty() { " " } else { &cell.text });
    }
    if text.is_ascii() {
        // Backgrounds and overlines are painted independently.
        if !cell.underline && !cell.strikethrough {
            text.truncate(text.trim_end_matches(' ').len());
        }
    }
    (text, end)
}
