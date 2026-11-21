//! Shared terminal wheel accumulation in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scroll {
    Row(u64),
    Lines(isize),
    Prompts(isize),
    LastPrompt,
    Top,
    Bottom,
}

#[derive(Clone, Copy, Debug)]
pub struct Scrollbar { pub top: f32, pub thumb: f32, pub height: f32, pub history: u64 }
impl Scrollbar {
    pub fn new(rows: usize, history: u64, offset: u64, height: f32) -> Option<Self> {
        if history == 0 || !height.is_finite() || height <= 0. { return None; }
        let thumb = (height * rows as f32 / (history as f32 + rows as f32)).max(8.).min(height);
        let top = (height-thumb) * history.saturating_sub(offset) as f32 / history as f32;
        Some(Self { top, thumb, height, history })
    }
    pub fn row_at(self, pointer: f32, grab: f32) -> u64 {
        let travel = self.height - self.thumb;
        if travel <= 0. { return 0; }
        (((pointer-grab)/travel).clamp(0.,1.) as f64 * self.history as f64).round() as u64
    }
}

/// Shift navigation leaves one overlapping row between adjacent pages.
pub fn navigation(key: &str, rows: usize) -> Option<Scroll> {
    let page = rows.saturating_sub(1).max(1) as isize;
    match key {
        "pageup" => Some(Scroll::Lines(-page)),
        "pagedown" => Some(Scroll::Lines(page)),
        "home" => Some(Scroll::Top),
        "end" => Some(Scroll::Bottom),
        _ => None,
    }
}
pub fn wheel_lines(remainder: &mut f32, delta_y: f32, cell_height: f32) -> isize {
    *remainder -= delta_y / cell_height;
    let lines = (remainder.trunc() as isize).clamp(-32, 32);
    *remainder -= lines as f32;
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scrollbar_preserves_grab_and_clamps_absolute_targets() {
        let bar=Scrollbar::new(20,80,40,100.).unwrap();
        assert_eq!((bar.top,bar.thumb),(40.,20.));
        assert_eq!(bar.row_at(45.,5.),40);
        assert_eq!(bar.row_at(-10.,5.),0);
        assert_eq!(bar.row_at(200.,5.),80);
        assert_eq!(Scrollbar::new(20,80,80,4.).unwrap().row_at(2.,1.),0);
        assert!(Scrollbar::new(20,0,0,100.).is_none());
    }
    #[test]
    fn navigation_overlaps_pages_and_handles_small_viewports() {
        assert_eq!(navigation("pageup", 24), Some(Scroll::Lines(-23)));
        assert_eq!(navigation("pagedown", 24), Some(Scroll::Lines(23)));
        for rows in [0, 1, 2] { assert_eq!(navigation("pageup", rows), Some(Scroll::Lines(-1))); }
        assert_eq!(navigation("home", 24), Some(Scroll::Top));
        assert_eq!(navigation("end", 24), Some(Scroll::Bottom));
        assert_eq!(navigation("up", 24), None);
    }
    #[test]
    fn preserves_fractional_motion_and_bounds_each_dispatch() {
        let mut remainder = 0.;
        assert_eq!(wheel_lines(&mut remainder, 8., 20.), 0);
        assert_eq!(wheel_lines(&mut remainder, 16., 20.), -1);
        assert!((remainder + 0.2).abs() < 0.001);
        assert_eq!(wheel_lines(&mut remainder, -24., 20.), 1);
        assert!(remainder.abs() < 0.001);
        assert_eq!(wheel_lines(&mut remainder, 1000., 20.), -32);
        assert_eq!(wheel_lines(&mut remainder, 0., 20.), -18);
    }
}
