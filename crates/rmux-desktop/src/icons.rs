//! Embedded monochrome UI icons; no runtime file access or icon font required.
use gpui::{prelude::*, *};
use std::borrow::Cow;
const ASSETS: &[(&str, &[u8])] = &[
    ("icons/group.svg", include_bytes!("../assets/icons/group.svg")),
    ("icons/expand.svg", include_bytes!("../assets/icons/expand.svg")),
    ("icons/external-link.svg", include_bytes!("../assets/icons/external-link.svg")),
    ("icons/branch.svg", include_bytes!("../assets/icons/branch.svg")),
    ("icons/pin.svg", include_bytes!("../assets/icons/pin.svg")),
    ("icons/plus.svg", include_bytes!("../assets/icons/plus.svg")),
    ("icons/close.svg", include_bytes!("../assets/icons/close.svg")),
    ("icons/chevron-up.svg", include_bytes!("../assets/icons/chevron-up.svg")),
    ("icons/chevron-down.svg", include_bytes!("../assets/icons/chevron-down.svg")),
    ("icons/chevron-right.svg", include_bytes!("../assets/icons/chevron-right.svg")),
    ("icons/copy.svg", include_bytes!("../assets/icons/copy.svg")),
    ("icons/refresh.svg", include_bytes!("../assets/icons/refresh.svg")),
    ("icons/sidebar.svg", include_bytes!("../assets/icons/sidebar.svg")),
    ("icons/split-right.svg", include_bytes!("../assets/icons/split-right.svg")),
    ("icons/split-down.svg", include_bytes!("../assets/icons/split-down.svg")),
    ("icons/bell.svg", include_bytes!("../assets/icons/bell.svg")),
];
pub struct Assets;
impl AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        Ok(ASSETS.iter().find(|(name, _)| *name == path).map(|(_, data)| Cow::Borrowed(*data)))
    }
    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        Ok(ASSETS.iter().filter(|(name, _)| name.starts_with(path)).map(|(name, _)| (*name).into()).collect())
    }
}
pub fn icon(name: &'static str) -> Svg {
    // GPUI's SVG paint path reads the element's own color, not inherited text style.
    svg().path(format!("icons/{name}.svg")).size(px(14.)).flex_shrink_0().text_color(rgb(0xb8b8b8))
}
