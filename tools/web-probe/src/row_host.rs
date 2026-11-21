//! GPUI CE bindings for the unchanged native row cache and painter.
use gpui::*;
pub(crate) use crate::cell_metrics::CellMetrics;
pub(crate) use rmux_core::terminal_frame::CellPaint as CellColor;
use rmux_core::terminal_frame::Color;
thread_local! {
    static BATCHED: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static FALLBACK: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}
pub(crate) fn shaping_summary() -> String {
    format!("ASCII runs: {} batched · {} fallback", BATCHED.get(), FALLBACK.get())
}
#[derive(Default)]
pub(crate) struct Metrics {
    pub rows_reused: u64, pub rows_rebuilt: u64,
    pub cursor_glyphs_reused: u64, pub cursor_glyphs_rebuilt: u64,
}
#[derive(Clone)]
pub(crate) struct ShapedLine(Vec<(gpui::ShapedLine, Pixels)>);
impl ShapedLine {
    pub fn paint(&self, origin: Point<Pixels>, height: Pixels, window: &mut Window, cx: &mut App) {
        for (line, x) in &self.0 {
            let _ = line.paint(origin + point(*x, px(0.)), height, TextAlign::Left, None, window, cx);
        }
    }
}
pub(crate) fn color(value: Color) -> Hsla {
    rgb_to_hsla(rgb((u32::from(value.0) << 16) | (u32::from(value.1) << 8) | u32::from(value.2)))
}
pub(crate) fn run(len: usize, font: Font, color: Hsla, underline: Option<UnderlineStyle>, strikethrough: Option<StrikethroughStyle>) -> TextRun {
    TextRun { len, font, color, underline, strikethrough, ..Default::default() }
}
pub(crate) fn shape(window: &Window, text: SharedString, cells: CellMetrics, run: TextRun) -> ShapedLine {
    // CE has no fixed-cell-width shaping parameter. Use a single shaped run only
    // when every ASCII caret boundary matches our terminal grid (including the
    // trailing edge). This rejects proportional fonts and collapsed ligatures.
    let line = window.text_system().shape_line(text.clone(), px(cells.font_size), &[run.clone()]);
    if text.is_ascii() {
        let on_grid = (f32::from(line.width()) - text.len() as f32 * cells.width).abs() <= 0.01
            && (0..text.len()).all(|column| line.platform_layout
                .caret_bounds(CaretPosition::attached_to_next_cluster(column), px(cells.height))
                .is_some_and(|bounds| (f32::from(bounds.origin.x) - column as f32 * cells.width).abs() <= 0.01));
        if on_grid {
            BATCHED.set(BATCHED.get() + 1);
            return ShapedLine(vec![(line, px(0.))]);
        }
        FALLBACK.set(FALLBACK.get() + 1);
        // Preserve exact columns for unsupported font shaping. The shared row
        // cache retains these glyphs until the row or font environment changes.
        ShapedLine(text.bytes().enumerate().map(|(column, byte)| {
            let mut run = run.clone(); run.len = 1;
            (window.text_system().shape_line(char::from(byte).to_string().into(), px(cells.font_size), &[run]), px(column as f32 * cells.width))
        }).collect())
    } else {
        // Shared grouping isolates non-ASCII clusters into their own runs.
        ShapedLine(vec![(line, px(0.))])
    }
}
pub(crate) fn mask(bounds: Bounds<Pixels>) -> ContentMask<Pixels> { ContentMask { bounds, ..Default::default() } }
pub(crate) fn selection_color() -> Hsla { rgb_to_hsla(rgba(0x668fca80)) }
