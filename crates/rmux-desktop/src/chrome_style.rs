//! Application-wide chrome palette, derived from the effective terminal theme.
//! GPUI builds and paints UI on its main thread. Cache only resolved color values
//! here so shared presentation helpers (also used by WASM) need no host settings.
use std::cell::Cell;
use gpui::Rgba;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Palette {
    pub background: u32, pub surface: u32, pub elevated: u32, pub hover: u32,
    pub foreground: u32, pub muted: u32, pub border: u32, pub accent: u32,
    pub selected: u32, pub selected_hover: u32, pub selected_border: u32,
    pub warning: u32, pub danger: u32, pub success: u32, pub on_accent: u32, pub hover_strong: u32,
}
fn mix(a: u32, b: u32, weight: f32) -> u32 {
    [16, 8, 0].into_iter().fold(0, |value, shift| {
        let a = ((a >> shift) & 255) as f32;
        let b = ((b >> shift) & 255) as f32;
        value | (((a + (b - a) * weight).round() as u32) << shift)
    })
}
fn luminance(color: u32) -> f32 {
    let channel = |shift: u32| {
        let value = ((color >> shift) & 255u32) as f32 / 255.;
        if value <= 0.04045 { value / 12.92 } else { ((value + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0)
}
fn contrast(a: u32, b: u32) -> f32 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}
fn readable(color: u32, background: u32, minimum: f32) -> u32 {
    let end = if contrast(0, background) > contrast(0xffffff, background) { 0 } else { 0xffffff };
    for step in 0..=20 {
        let candidate = mix(color, end, step as f32 / 20.);
        if contrast(candidate, background) >= minimum { return candidate; }
    }
    end
}
impl Palette {
    pub fn new(background: u32, foreground: u32, accent: u32, warning: u32, danger: u32, success: u32) -> Self {
        let foreground = readable(foreground, background, 7.);
        let surface = background;
        let elevated = mix(background, foreground, 0.035);
        let selected = mix(background, accent, 0.14);
        let selected_hover = mix(background, accent, 0.20);
        let accent = readable(accent, background, 4.5);
        Self { on_accent: readable(background, accent, 4.5), hover_strong: mix(background, foreground, 0.16), background, surface, elevated, hover: mix(background, foreground, 0.09),
            foreground: readable(foreground, selected_hover, 4.5),
            muted: readable(mix(background, foreground, 0.65), selected_hover, 4.5),
            border: mix(background, foreground, 0.22), accent,
            selected, selected_hover, selected_border: readable(accent, background, 3.),
            warning: readable(warning, elevated, 4.5), danger: readable(danger, elevated, 4.5), success: readable(success, elevated, 4.5) }
    }
}
impl Default for Palette {
    fn default() -> Self { Self::new(0x12141a, 0xdcdfe6, 0xa99be8, 0xf0b85b, 0xf18a8a, 0x9fceb0) }
}
thread_local! { static CURRENT: Cell<Palette> = Cell::new(Palette::default()); }
pub(crate) fn set_palette(palette: Palette) { CURRENT.with(|current| current.set(palette)); }
macro_rules! colors {
    ($($name:ident),+ $(,)?) => { $(pub(crate) fn $name() -> Rgba { CURRENT.with(|current| gpui::rgb(current.get().$name)) })+ };
}
colors!(background, surface, elevated, hover, foreground, muted, border, accent, selected, selected_hover, selected_border, warning, danger, success, on_accent, hover_strong);
pub(crate) fn selected_text() -> Rgba { foreground() }
pub(crate) fn selected_detail() -> Rgba { foreground() }

/// Keep a useful terminal width when an inspector is open. Narrow windows
/// overlay the inspector instead of resizing every PTY down to a few columns.
pub(crate) fn overlay_inspector(viewport_width: f32, sidebar_width: f32) -> bool {
    viewport_width - sidebar_width < 380. + 400.
}

pub(crate) fn inspector(overlay: bool) -> gpui::Div {
    use gpui::{prelude::*, *};
    div().w(px(380.)).max_w_full().h_full().flex_shrink_0().flex().flex_col()
        .bg(crate::chrome_style::surface()).border_l_1().border_color(crate::chrome_style::border())
        .when(overlay, |panel| panel.absolute().right_0().top_0().occlude().shadow_lg())
}

pub(crate) fn foreground_rgb() -> u32 { CURRENT.with(|current| current.get().foreground) }
pub(crate) fn muted_rgb() -> u32 { CURRENT.with(|current| current.get().muted) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn light_dark_and_low_contrast_imports_keep_readable_chrome() {
        for (bg, fg, accent) in [(0xffffff,0x24292f,0x0755a8),(0xfdf6e3,0x657b83,0x268bd2),
            (0x002b36,0x839496,0x268bd2),(0x2e3440,0xd8dee9,0x81a1c1),
            (0xeff1f5,0x4c4f69,0x1e66f5),(0x1e1e2e,0xcdd6f4,0x89b4fa),
            (0x777777,0x777777,0x777777)] {
            let theme = Palette::new(bg, fg, accent, accent, accent, accent);
            assert_eq!(theme.background, bg);
            assert_eq!(theme.surface, bg);
            // The selection tint must not erase text or focus indicators.
            assert!(contrast(theme.foreground, theme.selected_hover) >= 4.5);
            assert!(contrast(theme.muted, theme.selected_hover) >= 4.5);
            assert!(contrast(theme.accent, theme.background) >= 4.5);
        }
    }
}
