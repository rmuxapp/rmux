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
