//! Shared terminal IME overlay; shaping is delegated to the platform adapter.
use gpui::*;
use crate::{composition::Composition, cell_metrics::CellMetrics, text_layout};
pub(crate) struct Preedit {
    line: text_layout::Line,
    origin: Point<Pixels>,
    height: Pixels,
    backgrounds: Vec<PaintQuad>,
    pub cursor: Point<Pixels>,
}
impl Preedit {
    pub fn new(composition: &Composition, origin: Point<Pixels>, cells: CellMetrics,
        family: &SharedString, background: Hsla, window: &Window) -> Self {
        let line = text_layout::preedit(window, composition.text.clone().into(), family, cells.font_size);
        let selection = composition.byte_range(composition.selected.clone());
        let start = line.x_for_index(selection.start);
        let offset = line.x_for_index(selection.end);
        let height = px(cells.height);
        let mut backgrounds = vec![fill(Bounds::new(origin, size(line.width() + px(2.), height)), background)];
        if !selection.is_empty() {
            backgrounds.push(fill(Bounds::new(origin + point(start.min(offset), px(0.)),
                size((offset-start).abs(), height)), rgb(0x344b70)));
        }
        Self { line, origin, height, backgrounds, cursor: origin + point(offset, px(0.)) }
    }
    pub fn paint(&mut self, window: &mut Window, cx: &mut App) {
        for quad in self.backgrounds.drain(..) { window.paint_quad(quad); }
        self.line.paint(self.origin, self.height, window, cx);
    }
}

/// Convert platform UTF-16 ranges using the same shaped line as the overlay.
pub(crate) fn bounds_for_range(composition: &Composition, range: std::ops::Range<usize>,
    origin: Point<Pixels>, cells: CellMetrics, family: &SharedString, window: &Window) -> Bounds<Pixels> {
    let (offset, width) = if composition.text.is_empty() {
        (px(0.), px(cells.width))
    } else {
        let line = text_layout::preedit(window, composition.text.clone().into(), family, cells.font_size);
        let range = composition.byte_range(range);
        let start = line.x_for_index(range.start);
        let end = line.x_for_index(range.end);
        (start.min(end), (end-start).abs().max(px(2.)))
    };
    Bounds::new(origin + point(offset, px(0.)), size(width, px(cells.height)))
}

pub(crate) fn character_index_for_point(composition: &Composition, position: Point<Pixels>,
    origin: Point<Pixels>, cells: CellMetrics, family: &SharedString, window: &Window) -> Option<usize> {
    if composition.text.is_empty() { return Some(0); }
    if position.y < origin.y || position.y > origin.y + px(cells.height) { return None; }
    let line = text_layout::preedit(window, composition.text.clone().into(), family, cells.font_size);
    Some(composition.utf16_offset(line.closest_index_for_x(position.x-origin.x)))
}
