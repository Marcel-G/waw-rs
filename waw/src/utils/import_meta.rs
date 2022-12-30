use js_sys::JsString;
use regex::Regex;
use wasm_bindgen::prelude::*;

// This is a not-so-clean approach to get the current bindgen ES module URL
// in Rust. This will fail at run time on bindgen targets not using ES modules.
// https://github.com/rustwasm/wasm-bindgen/pull/3032
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen]
    type ImportMeta;

    #[wasm_bindgen(method, getter)]
    fn url(this: &ImportMeta) -> JsString;

    #[wasm_bindgen(js_namespace = import, js_name = meta)]
    static IMPORT_META: ImportMeta;
}

pub fn url_js() -> String {
    IMPORT_META.url().as_string().unwrap()
}

// Get the wasm module url by replacing the current `.js` module with `_bg.wasm`
// This is pretty delecate as its based on naming convention
pub fn url_wasm() -> String {
    let js_extension = Regex::new(r"\.js.*$").unwrap();
    js_extension.replace(&url_js(), "_bg.wasm").to_string()
}
