//! GPUI CE adapter; the text field and editing behavior remain shared with desktop.
use gpui::*;
pub struct Line(ShapedLine);
impl Line {
    pub fn x_for_index(&self, index: usize) -> Pixels {
        self.0.platform_layout.caret_bounds(CaretPosition::attached_to_next_cluster(index), px(24.))
            .map_or(self.0.width(), |bounds| bounds.origin.x)
    }
    pub fn closest_index_for_x(&self, x: Pixels) -> usize {
        let caret = self.0.platform_layout.caret_from_pixel_point(point(x, px(12.)), px(24.));
        caret.unwrap_or_else(|caret| caret).index
    }
    pub fn paint(&self, origin: Point<Pixels>, height: Pixels, window: &mut Window, cx: &mut App) {
        let _ = self.0.paint(origin, height, TextAlign::Left, None, window, cx);
    }
}
pub fn shape(window: &Window, text: SharedString, font: Font, color: u32) -> Line {
    let run = TextRun { len: text.len(), font, color: rgb_to_hsla(rgb(color)), ..Default::default() };
    Line(window.text_system().shape_line(text, px(14.), &[run]))
}
pub fn focus(handle: &FocusHandle, window: &mut Window, cx: &mut App) { handle.focus(window, cx); }
pub fn mask(bounds: Bounds<Pixels>) -> ContentMask<Pixels> { ContentMask { bounds, ..Default::default() } }
