use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = rmuxClipboard, js_name = readText)]
    fn read_text(max_bytes: usize) -> Result<js_sys::Promise, JsValue>;
}
/// Start the browser request before yielding user activation to an async task.
pub fn read() -> Result<js_sys::Promise, JsValue> {
    read_text(rmux_core::sample_input::MAX_INPUT_BYTES)
}
