//! Exercise the actual native row cache/painter using a captured Ghostty frame.
use gpui::{prelude::*, *};
use rmux_core::terminal_frame::Frame;
use crate::{cell_metrics::CellMetrics, row_cache, row_host};
pub(crate) struct TerminalSample {
    frame: Frame,
    rows: row_cache::RowCache,
}
impl TerminalSample {
    pub fn new(frame: Frame) -> Self { Self { frame, rows: Default::default() } }
}
impl Render for TerminalSample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        canvas(move |bounds, window, cx| entity.update(cx, |this, _| {
            let family: SharedString = "Fira Mono".into();
            let cells = CellMetrics::measured(14., family.clone(), window);
            let rows = this.rows.update(&this.frame, cells, &family, window, None);
            (rows, cells, row_host::color(this.frame.background), bounds)
        }), move |_, (rows, cells, background, bounds), window, cx| {
            window.with_content_mask(Some(row_host::mask(bounds)), |window| {
                window.paint_quad(fill(bounds, background));
                for (y, row) in rows.iter().enumerate() {
                    row_cache::paint_row(row, bounds.origin + point(px(8.), px(8. + y as f32 * cells.height)), None, None, cells, window, cx);
                }
            });
        }).size_full()
    }
}
