//! GPUI CE bindings for the unchanged native row cache and painter.
use gpui::*;
pub(crate) use crate::cell_metrics::CellMetrics;
pub(crate) use rmux_core::terminal_frame::CellPaint as CellColor;
use rmux_core::terminal_frame::Color;
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
    // CE has no fixed-cell-width shaping parameter. Place ASCII glyphs explicitly
    // on the terminal grid; the shared cache retains them between unchanged frames.
    // The shared grouping code isolates non-ASCII clusters into their own runs.
    if text.is_ascii() {
        ShapedLine(text.bytes().enumerate().map(|(column, byte)| {
            let mut run = run.clone(); run.len = 1;
            (window.text_system().shape_line(char::from(byte).to_string().into(), px(cells.font_size), &[run]), px(column as f32 * cells.width))
        }).collect())
    } else {
        ShapedLine(vec![(window.text_system().shape_line(text, px(cells.font_size), &[run]), px(0.))])
    }
}
pub(crate) fn mask(bounds: Bounds<Pixels>) -> ContentMask<Pixels> { ContentMask { bounds, ..Default::default() } }
pub(crate) fn selection_color() -> Hsla { rgb_to_hsla(rgba(0x668fca80)) }
