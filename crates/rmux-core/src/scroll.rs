//! Shared terminal wheel accumulation in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scroll {
    Lines(isize),
    Prompts(isize),
    LastPrompt,
    Top,
    Bottom,
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
