//! Shared native/browser scrollback position indicator.
use gpui::*;
use rmux_core::terminal_frame::Frame;

pub(crate) fn indicator(bounds: Bounds<Pixels>, frame: &Frame) -> Option<PaintQuad> {
    if frame.scrollback_offset == 0 || frame.scrollback_lines == 0 { return None; }
    let height = f32::from(bounds.size.height);
    let total = frame.scrollback_lines as f32 + frame.rows.len() as f32;
    let thumb = (height * frame.rows.len() as f32 / total).max(8.).min(height);
    let offset = frame.scrollback_lines.saturating_sub(frame.scrollback_offset) as f32
        / frame.scrollback_lines as f32;
    Some(fill(
        Bounds::new(bounds.origin + point(bounds.size.width - px(3.), px((height-thumb)*offset)), size(px(3.),px(thumb))),
        rgb(0x667085),
    ))
}
