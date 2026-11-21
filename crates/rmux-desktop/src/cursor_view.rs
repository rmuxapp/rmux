//! Shared terminal cursor geometry and paint, independent of preference policy.
use gpui::*;
use rmux_core::terminal_frame::CursorShape;
pub(crate) fn quads(shape: CursorShape, origin: Point<Pixels>, width: f32, height: f32, color: Hsla) -> Vec<PaintQuad> {
    let paint = |x, y, w, h| fill(Bounds::new(origin + point(px(x), px(y)), size(px(w), px(h))), color);
    match shape {
        CursorShape::Bar => vec![paint(0., 0., 2., height)],
        CursorShape::Underline => vec![paint(0., height - 2., width, 2.)],
        CursorShape::Block => vec![paint(0., 0., width, height)],
        CursorShape::Hollow => vec![
            paint(0., 0., width, 1.), paint(0., height - 1., width, 1.),
            paint(0., 0., 1., height), paint(width - 1., 0., 1., height),
        ],
    }
}
