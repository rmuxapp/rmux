use gpui::*;
use rmux_core::terminal_frame::{Cell, Color, Frame};
use std::{cell::RefCell, sync::{Arc, Weak}};

use crate::row_host::{self, CellMetrics, CellColor, Metrics, ShapedLine, color};

/// Drop desktop-owned cell snapshots for an unmounted terminal. Keep viewport
/// dimensions for input/navigation metadata; the next prepaint fetches a fresh
/// engine frame. If the engine is busy, this placeholder paints background only.
pub fn release_frame_cells(frame: &mut Frame) {
    let rows = frame.rows.len();
    frame.rows = Arc::new(vec![Arc::new(Vec::new()); rows]);
    frame.selections = Arc::new(vec![None; rows]);
    frame.cursor = None;
}

/// One shaped glyph for the block cursor; bounded independently of row caches.
#[derive(Default)]
pub struct CursorCache {
    entry: Option<(Cell, CellMetrics, SharedString, f32, Color, ShapedLine)>,
}
impl CursorCache {
    pub fn shape(&mut self, cell: &Cell, foreground: Color, cells: CellMetrics, family: &SharedString, window: &Window, metrics: Option<&mut Metrics>) -> ShapedLine {
        if let Some((previous, cell_metrics, font_family, scale, previous_foreground, line)) = &self.entry
            && previous == cell && *cell_metrics == cells && font_family == family
            && *scale == window.scale_factor() && *previous_foreground == foreground
        {
            if let Some(metrics) = metrics { metrics.cursor_glyphs_reused += 1; }
            return line.clone();
        }
        if let Some(metrics) = metrics { metrics.cursor_glyphs_rebuilt += 1; }
        let mut cursor_font = font(family.clone());
        if cell.bold { cursor_font.weight = FontWeight::BOLD; }
        if cell.italic { cursor_font.style = FontStyle::Italic; }
        let run = row_host::run(cell.text.len(), cursor_font, color(foreground), None, None);
        let line = row_host::shape(window, cell.text.to_string().into(), cells, run);
        self.entry = Some((cell.clone(), cells, family.clone(), window.scale_factor(), foreground, line.clone()));
        line
    }
}

pub struct RowText {
    pub line: ShapedLine,
    pub x: Pixels,
}
pub struct RowBackground {
    pub x: Pixels,
    pub width: Pixels,
    pub color: Color,
}
pub struct ShapedRow {
    source: Arc<Vec<Cell>>,
    pub texts: Vec<RowText>,
    selected_texts: RefCell<Option<(CellColor, Vec<RowText>, Vec<RowBackground>)>>,
    pub backgrounds: Vec<RowBackground>,
    overlines: Vec<RowBackground>,
}

#[derive(Default)]
pub struct RowCache {
    rows: Arc<Vec<Arc<ShapedRow>>>,
    source: Weak<Vec<Arc<Vec<Cell>>>>,
    environment: Option<(Color, f32, CellMetrics, SharedString)>,
}
impl RowCache {
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn release(&mut self) -> usize {
        let released = self.rows.len();
        *self = Self::default();
        released
    }

    /// Selection variants can contain a second set of shaped glyphs per row.
    /// Keep them only while the corresponding row is selected in this frame.
    fn release_unused_selection(&self, selections: &[Option<(u16, u16)>], foreground: Option<CellColor>) {
        for (index, row) in self.rows.iter().enumerate() {
            let mut cached = row.selected_texts.borrow_mut();
            let selected = selections.get(index).is_some_and(Option::is_some);
            if !selected || cached.as_ref().is_some_and(|(color, _, _)| Some(*color) != foreground) {
                cached.take();
            }
        }
    }

    pub fn prepare_selection(&mut self, frame: &Frame, foreground: Option<CellColor>, cells: CellMetrics, family: &SharedString, window: &Window) {
        self.release_unused_selection(&frame.selections, foreground);
        let Some(foreground) = foreground else { return; };
        for (row, selection) in self.rows.iter().zip(frame.selections.iter()) {
            if selection.is_none() { continue; }
            let mut cached = row.selected_texts.borrow_mut();
            if cached.as_ref().is_none_or(|(color, _, _)| *color != foreground) {
                let mut overlines = Vec::new();
                for cell in row.source.iter().filter(|cell| cell.overline && !cell.hidden) {
                    append_span(&mut overlines, px(cell.column as f32 * cells.width), foreground.resolve(cell.foreground, cell.background), cells);
                }
                *cached = Some((foreground, shape_texts(&row.source, cells, family, window, Some(foreground)), overlines));
            }
        }
    }

    pub fn update(
        &mut self,
        frame: &Frame,
        cells: CellMetrics,
        family: &SharedString,
        window: &Window,
        mut metrics: Option<&mut Metrics>,
    ) -> Arc<Vec<Arc<ShapedRow>>> {
        let environment = (
            frame.background,
            window.scale_factor(),
            cells,
            family.clone(),
        );
        if self.environment.as_ref() != Some(&environment) {
            self.rows = Arc::default();
            self.source = Weak::new();
            self.environment = Some(environment);
        }
        // The immutable outer snapshot also identifies all row identities.
        // A weak reference prevents address reuse without retaining old row data
        // or forcing extra strong-reference COW when the engine publishes output.
        if self.source.as_ptr() == Arc::as_ptr(&frame.rows) {
            if let Some(metrics) = metrics { metrics.rows_reused += self.rows.len() as u64; }
            return self.rows.clone();
        }
        self.source = Arc::downgrade(&frame.rows);
        if self.rows.len() == frame.rows.len()
            && self
                .rows
                .iter()
                .zip(frame.rows.iter())
                .all(|(row, source)| Arc::ptr_eq(&row.source, source))
        {
            if let Some(metrics) = metrics {
                metrics.rows_reused += self.rows.len() as u64;
            }
            return self.rows.clone();
        }
        let rows = Arc::make_mut(&mut self.rows);
        rows.truncate(frame.rows.len());
        for (index, source) in frame.rows.iter().enumerate() {
            if rows
                .get(index)
                .is_some_and(|row| Arc::ptr_eq(&row.source, source))
            {
                if let Some(metrics) = metrics.as_deref_mut() {
                    metrics.rows_reused += 1;
                }
                continue;
            }
            if let Some(metrics) = metrics.as_deref_mut() {
                metrics.rows_rebuilt += 1;
            }
            let row = Arc::new(shape_row(
                source.clone(),
                frame.background,
                cells,
                family,
                window,
            ));
            if index < rows.len() {
                rows[index] = row;
            } else {
                rows.push(row);
            }
        }
        // Share the entire paint snapshot. Unchanged repaints neither allocate
        // a row vector nor clone one Arc per terminal row.
        self.rows.clone()
    }
}

fn shape_row(
    source: Arc<Vec<Cell>>,
    background: Color,
    cells: CellMetrics,
    family: &SharedString,
    window: &Window,
) -> ShapedRow {
    let mut backgrounds: Vec<RowBackground> = Vec::new();
    let mut overlines = Vec::new();
    // Visit every cell, independently of text grouping. A wide glyph's trailing
    // cell and empty colored cells still need their backgrounds painted.
    for cell in source.iter() {
        if cell.overline && !cell.hidden {
            append_span(
                &mut overlines,
                px(cell.column as f32 * cells.width),
                cell.foreground,
                cells,
            );
        }
        if cell.background == background {
            continue;
        }
        let x = px(cell.column as f32 * cells.width);
        if let Some(previous) = backgrounds.last_mut()
            && previous.color == cell.background
            && previous.x + previous.width == x
        {
            previous.width += px(cells.width);
        } else {
            backgrounds.push(RowBackground {
                x,
                width: px(cells.width),
                color: cell.background,
            });
        }
    }
    let texts = shape_texts(&source, cells, family, window, None);
    ShapedRow { source, texts, selected_texts: RefCell::default(), backgrounds, overlines }
}

fn shape_texts(source: &[Cell], cells: CellMetrics, family: &SharedString, window: &Window, foreground: Option<CellColor>) -> Vec<RowText> {
    let mut texts = Vec::new();
    let mut i = 0;
    while i < source.len() {
        let cell = &source[i];
        if cell.hidden
            || cell.width == 0
            || (cell.text.is_empty() && !cell.underline && !cell.strikethrough)
        {
            i += 1;
            continue;
        }
        let (text, end) = text_run_with_color(source, i, foreground);
        if !text.is_empty() {
            let mut run_font = font(family.clone());
            if cell.bold {
                run_font.weight = FontWeight::BOLD;
            }
            if cell.italic {
                run_font.style = FontStyle::Italic;
            }
            let run = row_host::run(text.len(), run_font, color(foreground.map(|setting| setting.resolve(cell.foreground, cell.background)).unwrap_or(cell.foreground)),
                cell.underline.then_some(UnderlineStyle {
                    thickness: px(1.),
                    color: Some(color(foreground.map(|setting| setting.resolve(cell.foreground, cell.background)).unwrap_or(cell.underline_color))),
                    wavy: cell.underline_curly,
                }),
                cell.strikethrough.then_some(StrikethroughStyle {
                    thickness: px(1.),
                    color: None,
                }),
            );
            texts.push(RowText {
                line: row_host::shape(window, text.into(), cells, run),
                x: px(cell.column as f32 * cells.width),
            });
        }
        i = end;
    }
    texts
}

/// Empty engine cells still occupy one column and may carry decorations.
#[cfg(test)]
fn text_run(source: &[Cell], start: usize) -> (String, usize) {
    text_run_with_color(source, start, None)
}
fn text_run_with_color(source: &[Cell], start: usize, foreground: Option<CellColor>) -> (String, usize) {
    rmux_core::terminal_runs::text_run(source, start, matches!(foreground, Some(CellColor::CellBackground)))
}

#[cfg(test)]
mod tests {
    use super::text_run;

    #[test]
    fn dynamic_selection_foreground_splits_runs_at_background_changes() -> anyhow::Result<()> {
        use super::*;
        let mut terminal = rmux_terminal::Terminal::new(8, 1)?;
        terminal.write(b"\x1b[31;44mab\x1b[45mcd\x1b[8mef");
        let frame = terminal.frame()?;
        let source = &frame.rows[0];
        assert_eq!(text_run_with_color(source, 0, None), ("abcd".into(), 4));
        assert_eq!(text_run_with_color(source, 0, Some(CellColor::CellBackground)), ("ab".into(), 2));
        assert_eq!(text_run_with_color(source, 2, Some(CellColor::CellBackground)), ("cd".into(), 4));
        assert_eq!(text_run_with_color(source, 0, Some(CellColor::CellForeground)), ("abcd".into(), 4));
        assert_eq!(text_run_with_color(source, 4, Some(CellColor::CellBackground)), (String::new(), 5));
        Ok(())
    }

    #[test]
    fn dynamic_selection_background_runs_follow_cells_and_clip() -> anyhow::Result<()> {
        use super::*;
        let mut terminal = rmux_terminal::Terminal::new(6, 1)?;
        terminal.write(b"\x1b[31;44mab\x1b[32mcd\x1b[45mef");
        let frame = terminal.frame()?;
        let source = &frame.rows[0];
        let collect = |selection, setting| {
            let mut runs = Vec::new();
            selection_spans(source, selection, setting, |start, end, color| runs.push((start, end, color)));
            runs
        };
        assert_eq!(collect((1, 4), Some(CellColor::CellForeground)), vec![(1,2,Some(source[1].foreground)), (2,5,Some(source[2].foreground))]);
        assert_eq!(collect((0, u16::MAX), Some(CellColor::CellBackground)), vec![(0,4,Some(source[0].background)), (4,6,Some(source[4].background))]);
        assert_eq!(collect((1,4), None), vec![(1,5,None)]);
        assert!(collect((5,2), Some(CellColor::CellForeground)).is_empty());
        assert!(collect((9,12), None).is_empty());
        Ok(())
    }


    #[test]
    fn selection_variants_release_on_clear_shrink_and_color_changes() {
        use super::*;
        let setting = CellColor::CellForeground;
        let make_row = || Arc::new(ShapedRow {
            source: Arc::new(Vec::new()), texts: Vec::new(),
            selected_texts: RefCell::new(Some((setting, Vec::with_capacity(64), Vec::with_capacity(64)))),
            backgrounds: Vec::new(), overlines: Vec::new(),
        });
        let cache = RowCache { rows: Arc::new(vec![make_row(), make_row(), make_row()]), ..Default::default() };
        // A shortened selection array must also release variants beyond its end.
        cache.release_unused_selection(&[None, Some((0, 3))], Some(setting));
        assert!(cache.rows[0].selected_texts.borrow().is_none());
        assert!(cache.rows[2].selected_texts.borrow().is_none());
        assert_eq!(cache.rows[1].selected_texts.borrow().as_ref().unwrap().1.capacity(), 64);
        cache.release_unused_selection(&[None, Some((0, 3))], Some(CellColor::CellBackground));
        assert!(cache.rows[1].selected_texts.borrow().is_none());
        let cache = RowCache { rows: Arc::new(vec![make_row()]), ..Default::default() };
        cache.release_unused_selection(&[Some((0, 3))], None);
        assert!(cache.rows[0].selected_texts.borrow().is_none());
        let cache = RowCache { rows: Arc::new(vec![make_row()]), ..Default::default() };
        cache.release_unused_selection(&[None], Some(setting));
        assert!(cache.rows[0].selected_texts.borrow().is_none());
    }

    #[test]
    fn hidden_frame_releases_cells_without_changing_engine_state() -> anyhow::Result<()> {
        use super::*;
        let mut terminal = rmux_terminal::Terminal::new(40, 3)?;
        terminal.write("retained 世界\r\nsecond line".as_bytes());
        let mut frame = terminal.frame()?;
        let original = frame.text();
        let revision = frame.revision;
        let row = Arc::downgrade(&frame.rows[0]);
        release_frame_cells(&mut frame);
        assert_eq!(frame.rows.len(), 3);
        assert_eq!(frame.columns, 40);
        assert_eq!(frame.revision, revision);
        assert!(frame.rows.iter().all(|row| row.is_empty()));
        assert!(frame.selections.iter().all(Option::is_none));
        assert!(frame.cursor.is_none());
        assert!(Arc::ptr_eq(&frame.rows[0], &frame.rows[1]));
        assert_eq!(terminal.frame()?.text(), original);
        drop(terminal);
        assert!(row.upgrade().is_none(), "desktop placeholder must not retain engine cells");
        Ok(())
    }

    #[test]
    fn releasing_cache_drops_rows_and_backing_capacity() {
        use super::*;
        let row = Arc::new(ShapedRow {
            source: Arc::new(Vec::new()),
            texts: Vec::new(),
            selected_texts: RefCell::default(),
            backgrounds: Vec::new(),
            overlines: Vec::new(),
        });
        let weak = Arc::downgrade(&row);
        let mut cache = RowCache {
            rows: Arc::new(vec![row]),
            ..Default::default()
        };
        assert!(!cache.is_empty());
        cache.release();
        assert!(cache.is_empty());
        assert_eq!(cache.rows.capacity(), 0);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn concealed_text_never_enters_shaped_runs() -> anyhow::Result<()> {
        let mut terminal = rmux_terminal::Terminal::new(20, 2)?;
        terminal.write(b"visible\x1b[8;4;9;53msecret\x1b[0mend");
        let frame = terminal.frame()?;
        let row = &frame.rows[0];
        assert_eq!(text_run(row, 0), ("visible".into(), 7));
        for index in 7..13 {
            assert!(row[index].hidden);
            assert_eq!(text_run(row, index), (String::new(), index + 1));
        }
        assert_eq!(text_run(row, 13), ("end".into(), 20));
        Ok(())
    }

    #[test]
    fn empty_decorated_cells_keep_their_columns() -> anyhow::Result<()> {
        let mut terminal = rmux_terminal::Terminal::new(8, 2)?;
        terminal.write(b"\x1b[4m   \x1b[0m");
        let frame = terminal.frame()?;
        assert!(frame.rows[0][0].underline);
        assert_eq!(text_run(&frame.rows[0], 0), ("   ".into(), 3));
        // Test the renderer's empty-cell contract directly. ECH strips these
        // decorations in Ghostty, so it is not a fixture for this representation.
        let mut row = (*frame.rows[0]).clone();
        for cell in &mut row[..3] {
            cell.text = Default::default();
        }
        assert_eq!(text_run(&row, 0), ("   ".into(), 3));
        Ok(())
    }

    #[test]
    fn batching_preserves_styles_wide_cells_and_blank_trimming() -> anyhow::Result<()> {
        let mut terminal = rmux_terminal::Terminal::new(16, 2)?;
        terminal.write("ab\x1b[31mcd\x1b[0m界x  ".as_bytes());
        let frame = terminal.frame()?;
        let row = &frame.rows[0];
        assert_eq!(text_run(row, 0), ("ab".into(), 2));
        assert_eq!(text_run(row, 2), ("cd".into(), 4));
        assert_eq!(text_run(row, 4), ("界".into(), 5));
        assert_eq!(row[5].width, 0);
        assert_eq!(text_run(row, 6), ("x".into(), 16));
        Ok(())
    }
}

// Emit half-open runs without allocating a second cell/background buffer.
fn selection_spans(source: &[Cell], selection: (u16, u16), setting: Option<CellColor>, mut emit: impl FnMut(usize, usize, Option<Color>)) {
    let start = usize::from(selection.0).min(source.len());
    let end = (usize::from(selection.1) + 1).min(source.len());
    if start >= end { return; }
    let resolve = |cell: &Cell| setting.map(|setting| setting.resolve(cell.foreground, cell.background));
    let mut run_start = start;
    let mut background = resolve(&source[start]);
    if matches!(setting, None | Some(CellColor::Literal(_))) {
        emit(start, end, background);
        return;
    }
    for index in start + 1..end {
        let next = resolve(&source[index]);
        if next != background {
            emit(run_start, index, background);
            run_start = index;
            background = next;
        }
    }
    emit(run_start, end, background);
}

pub fn paint_row(
    row: &ShapedRow,
    origin: Point<Pixels>,
    selection: Option<(u16, u16)>,
    selection_background: Option<CellColor>,
    cells: CellMetrics,
    window: &mut Window,
    cx: &mut App,
) {
    for background in &row.backgrounds {
        window.paint_quad(fill(
            Bounds::new(
                origin + point(background.x, px(0.)),
                size(background.width, px(cells.height)),
            ),
            color(background.color),
        ));
    }
    if let Some(selection) = selection {
        selection_spans(&row.source, selection, selection_background, |start, end, background| {
            window.paint_quad(fill(
                Bounds::new(
                    origin + point(px(start as f32 * cells.width), px(0.)),
                    size(px((end - start) as f32 * cells.width), px(cells.height)),
                ),
                background.map(color).unwrap_or_else(|| row_host::selection_color()),
            ));
        });
    }
    let paint = |texts: &[RowText], overlines: &[RowBackground], window: &mut Window, cx: &mut App| {
        for text in texts {
            let _ = text.line.paint(origin + point(text.x, px(0.)), px(cells.height), window, cx);
        }
        for span in overlines {
            window.paint_quad(fill(
                Bounds::new(origin + point(span.x, px(1.)), size(span.width, px(1.))),
                color(span.color),
            ));
        }
    };
    let selected_texts = row.selected_texts.borrow();
    if let (Some((start, end)), Some((_, texts, overlines))) = (selection, selected_texts.as_ref()) {
        // Clip both variants so antialiased glyph edges never blend with the
        // original foreground underneath the selected variant.
        let left = start as f32 * cells.width;
        let right = (end as f32 + 1.) * cells.width;
        let width = row.source.len() as f32 * cells.width;
        for (x, width) in [(0., left), (right, (width - right).max(0.))] {
            if width > 0. {
                window.with_content_mask(Some(row_host::mask(Bounds::new(origin + point(px(x), px(0.)), size(px(width), px(cells.height))))), |window| paint(&row.texts, &row.overlines, window, cx));
            }
        }
        window.with_content_mask(Some(row_host::mask(Bounds::new(origin + point(px(left), px(0.)), size(px(right - left), px(cells.height))))), |window| paint(texts, overlines, window, cx));
    } else {
        paint(&row.texts, &row.overlines, window, cx);
    }
}
fn append_span(spans: &mut Vec<RowBackground>, x: Pixels, color: Color, cells: CellMetrics) {
    if let Some(previous) = spans.last_mut()
        && previous.color == color
        && previous.x + previous.width == x
    {
        previous.width += px(cells.width);
    } else {
        spans.push(RowBackground {
            x,
            width: px(cells.width),
            color,
        });
    }
}
