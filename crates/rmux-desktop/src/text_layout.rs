//! Small host API boundary for the shared text field.
use gpui::*;
pub struct Line(ShapedLine);
impl Line {
    pub fn x_for_index(&self, index: usize) -> Pixels { self.0.x_for_index(index) }
    pub fn closest_index_for_x(&self, x: Pixels) -> usize { self.0.closest_index_for_x(x) }
    pub fn paint(&self, origin: Point<Pixels>, height: Pixels, window: &mut Window, cx: &mut App) {
        let _ = self.0.paint(origin, height, window, cx);
    }
}
pub fn shape(window: &Window, text: SharedString, font: Font, color: u32) -> Line {
    let run = TextRun { len: text.len(), font, color: rgb(color).into(),
        background_color: None, underline: None, strikethrough: None };
    Line(window.text_system().shape_line(text, px(14.), &[run], None))
}
pub fn focus(handle: &FocusHandle, window: &mut Window, _: &mut App) { handle.focus(window); }
pub fn mask(bounds: Bounds<Pixels>) -> ContentMask<Pixels> { ContentMask { bounds } }
