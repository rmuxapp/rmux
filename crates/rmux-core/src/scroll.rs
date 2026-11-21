//! Shared terminal wheel accumulation in logical pixels.
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
