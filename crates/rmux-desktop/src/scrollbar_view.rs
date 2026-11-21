//! Shared native/browser scrollback position indicator.
use gpui::*;
use rmux_core::terminal_frame::Frame;
use rmux_core::scroll::{Scroll, Scrollbar};

#[derive(Clone, Copy)]
pub(crate) struct Drag { grab: f32 }
impl Drag {
    pub fn target(self, bounds: Bounds<Pixels>, frame: &Frame, position: Point<Pixels>) -> Option<Scroll> {
        let bar=Scrollbar::new(frame.rows.len(),frame.scrollback_lines,frame.scrollback_offset,f32::from(bounds.size.height))?;
        Some(Scroll::Row(bar.row_at(f32::from(position.y-bounds.origin.y),self.grab)))
    }
}

pub(crate) fn begin_drag(bounds: Bounds<Pixels>, frame: &Frame, position: Point<Pixels>) -> Option<Drag> {
    if frame.scrollback_offset == 0 || !bounds.contains(&position)
        || position.x < bounds.origin.x + bounds.size.width - px(10.) { return None; }
    let bar=Scrollbar::new(frame.rows.len(),frame.scrollback_lines,frame.scrollback_offset,f32::from(bounds.size.height))?;
    let y=f32::from(position.y-bounds.origin.y);
    Some(Drag { grab: if y >= bar.top && y <= bar.top+bar.thumb { y-bar.top } else { bar.thumb/2. } })
}

pub(crate) fn indicator(bounds: Bounds<Pixels>, frame: &Frame) -> Option<PaintQuad> {
    if frame.scrollback_offset == 0 || frame.scrollback_lines == 0 { return None; }
    let bar = Scrollbar::new(frame.rows.len(),frame.scrollback_lines,frame.scrollback_offset,f32::from(bounds.size.height))?;
    Some(fill(
        Bounds::new(bounds.origin + point(bounds.size.width - px(3.), px(bar.top)), size(px(3.),px(bar.thumb))),
        rgb(0x667085),
    ))
}
