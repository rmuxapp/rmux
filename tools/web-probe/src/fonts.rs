//! Optional browser fonts stay outside WASM and load only on demand.
use gpui::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = rmuxFonts, js_name = loadCjk)]
    async fn load_cjk() -> Result<JsValue, JsValue>;
}
#[derive(Default)]
pub struct Fonts { requested: bool, pub revision: u64, pub error: Option<String> }
impl Global for Fonts {}
thread_local! {
    static FALLBACKS: FontFallbacks = FontFallbacks::from_fonts(vec!["Noto Sans CJK JP".into()]);
}
pub fn fallback(mut font: Font) -> Font {
    font.fallbacks = Some(FALLBACKS.with(Clone::clone));
    font
}
pub fn ensure(text: &str, cx: &mut App) {
    if cx.global::<Fonts>().requested || !text.chars().any(|ch| matches!(ch as u32,
        0x1100..=0x11ff | 0x2e80..=0xa4cf | 0xac00..=0xd7ff | 0xf900..=0xfaff |
        0xfe10..=0xfe6f | 0xff00..=0xffef | 0x20000..=0x323af)) { return; }
    cx.global_mut::<Fonts>().requested = true;
    cx.spawn(async move |cx| {
        let bytes = load_cjk().await.map(|bytes| js_sys::Uint8Array::new(&bytes).to_vec());
        cx.update(|cx| {
            let loaded = bytes.ok().is_some_and(|bytes|
                cx.text_system().add_fonts(vec![std::borrow::Cow::Owned(bytes)]).is_ok());
            let fonts = cx.global_mut::<Fonts>();
            if loaded { fonts.revision += 1; }
            else { fonts.error = Some("CJK font could not load. Reload the demo to retry.".into()); }
            cx.refresh_windows();
        });
    }).detach();
}
