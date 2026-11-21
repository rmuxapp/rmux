//! Native GPUI bindings for the shared terminal row renderer.
use gpui::*;
pub(crate) use crate::{cell_metrics::CellMetrics, diagnostics::Metrics, color};
pub(crate) use gpui::ShapedLine;
pub(crate) fn run(len: usize, font: Font, color: Hsla, underline: Option<UnderlineStyle>, strikethrough: Option<StrikethroughStyle>) -> TextRun {
    TextRun { len, font, color, background_color: None, underline, strikethrough }
}
pub(crate) fn shape(window: &Window, text: SharedString, cells: CellMetrics, run: TextRun) -> ShapedLine {
    window.text_system().shape_line(text, px(cells.font_size), &[run], Some(px(cells.width)))
}
pub(crate) fn mask(bounds: Bounds<Pixels>) -> ContentMask<Pixels> { ContentMask { bounds } }
pub(crate) fn selection_color() -> Hsla { rgba(0x668fca80).into() }

pub(crate) use rmux_core::terminal_frame::CellPaint as CellColor;
