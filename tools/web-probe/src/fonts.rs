//! Optional browser fonts stay outside WASM and load only on demand.
use gpui::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = rmuxFonts, js_name = loadFont)]
    async fn load_font(kind: &str) -> Result<JsValue, JsValue>;
}
#[derive(Default)]
pub struct Fonts { requested: [bool; 2], pub revision: u64, pub error: Option<String> }
impl Global for Fonts {}
thread_local! {
    static FALLBACKS: FontFallbacks = FontFallbacks::from_fonts(vec!["Noto Sans CJK JP".into(), "Noto Emoji".into()]);
}
pub fn fallback(mut font: Font) -> Font {
    font.fallbacks = Some(FALLBACKS.with(Clone::clone));
    font
}
pub fn ensure(text: &str, cx: &mut App) {
    let cjk = text.chars().any(|ch| matches!(ch as u32,
        0x1100..=0x11ff | 0x2e80..=0xa4cf | 0xac00..=0xd7ff | 0xf900..=0xfaff |
        0xfe10..=0xfe6f | 0xff00..=0xffef | 0x20000..=0x323af));
    let emoji = text.chars().any(|ch| matches!(ch as u32,
        0x1f000..=0x1faff | 0x2600..=0x27bf | 0x2194..=0x21ff | 0x2300..=0x23ff |
        0x2b00..=0x2bff | 0xfe0f | 0x20e3 | 0xa9 | 0xae | 0x203c | 0x2049 | 0x2122 | 0x2139));
    for (index, kind, needed) in [(0, "cjk", cjk), (1, "emoji", emoji)] {
        if !needed || cx.global::<Fonts>().requested[index] { continue; }
        cx.global_mut::<Fonts>().requested[index] = true;
        cx.spawn(async move |cx| {
            let bytes = load_font(kind).await.map(|bytes| js_sys::Uint8Array::new(&bytes).to_vec());
            cx.update(|cx| {
                let loaded = bytes.ok().is_some_and(|bytes|
                    cx.text_system().add_fonts(vec![std::borrow::Cow::Owned(bytes)]).is_ok());
                let fonts = cx.global_mut::<Fonts>();
                if loaded { fonts.revision += 1; }
                else { fonts.error = Some(format!("The {kind} font could not load. Reload the demo to retry.")); }
                cx.refresh_windows();
            });
        }).detach();
    }
}
