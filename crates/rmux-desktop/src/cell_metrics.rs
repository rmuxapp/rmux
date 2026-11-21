pub fn content_bounds(bounds: gpui::Bounds<gpui::Pixels>, padding: u8) -> gpui::Bounds<gpui::Pixels> {
    use gpui::{Bounds, point, px, size};
    let inset_x = f32::from(bounds.size.width).max(0.).min(f32::from(padding) * 2.) / 2.;
    let inset_y = f32::from(bounds.size.height).max(0.).min(f32::from(padding) * 2.) / 2.;
    Bounds::new(
        bounds.origin + point(px(inset_x), px(inset_y)),
        size((bounds.size.width - px(inset_x * 2.)).max(px(0.)), (bounds.size.height - px(inset_y * 2.)).max(px(0.))),
    )
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellMetrics {
    pub font_size: f32,
    pub width: f32,
    pub height: f32,
}
impl CellMetrics {
    pub fn grid_size(self, size: gpui::Size<gpui::Pixels>) -> (u16, u16) {
        (((f32::from(size.width) / self.width) as u16).max(1),
         ((f32::from(size.height) / self.height) as u16).max(1))
    }
    pub fn measured(font_size: f32, family: gpui::SharedString, window: &gpui::Window) -> Self {
        let mut cells = Self::new(font_size);
        let system = window.text_system();
        let id = system.resolve_font(&gpui::font(family));
        if let Ok(advance) = system.advance(id, gpui::px(cells.font_size), '0') {
            let width = f32::from(advance.width);
            if width.is_finite() && width > 0. {
                cells.width = width;
            }
        }
        let height = f32::from(system.ascent(id, gpui::px(cells.font_size)))
            + f32::from(system.descent(id, gpui::px(cells.font_size))).abs();
        if height.is_finite() {
            cells.height = cells.height.max(height.ceil());
        }
        cells
    }
    pub fn new(font_size: f32) -> Self {
        let font_size = if font_size.is_finite() {
            font_size.clamp(8., 40.)
        } else {
            14.
        };
        Self {
            font_size,
            width: 8.5 * font_size / 14.,
            height: 20. * font_size / 14.,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::CellMetrics;
    #[test]
    fn grid_dimensions_floor_partial_cells_and_keep_one_cell() {
        use gpui::{px, size};
        let cells = CellMetrics::new(14.);
        assert_eq!(cells.grid_size(size(px(84.), px(59.))), (9, 2));
        assert_eq!(cells.grid_size(size(px(85.), px(60.))), (10, 3));
        assert_eq!(cells.grid_size(size(px(0.), px(0.))), (1, 1));
    }
    #[test]
    fn padding_preserves_origin_and_bounds_small_panes() {
        use gpui::{Bounds, point, px, size};
        let bounds = Bounds::new(point(px(30.), px(50.)), size(px(200.), px(100.)));
        assert_eq!(super::content_bounds(bounds, 0), bounds);
        assert_eq!(super::content_bounds(bounds, 8), Bounds::new(point(px(38.), px(58.)), size(px(184.), px(84.))));
        let small = Bounds::new(bounds.origin, size(px(6.), px(10.)));
        assert_eq!(super::content_bounds(small, 8), Bounds::new(point(px(33.), px(55.)), size(px(0.), px(0.))));
    }
    #[test]
    fn zoom_scales_both_axes_and_bounds_input() {
        let base = CellMetrics::new(14.);
        let zoom = CellMetrics::new(28.);
        assert_eq!(
            (zoom.width, zoom.height),
            (base.width * 2., base.height * 2.)
        );
        assert_eq!(CellMetrics::new(0.).font_size, 8.);
        assert_eq!(CellMetrics::new(100.).font_size, 40.);
        assert_eq!(CellMetrics::new(f32::NAN), base);
    }
}
