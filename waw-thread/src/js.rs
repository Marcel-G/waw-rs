//! JavaScript bindings for audio worklet support.

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    /// Type of [`import.meta`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/import.meta).
    pub type Meta;

    /// Returns [`import.meta`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/import.meta).
    #[wasm_bindgen(thread_local, js_namespace = import, js_name = meta)]
    pub static META: Meta;

    /// See [`import.meta.url`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/import.meta#url).
    #[wasm_bindgen(method, getter)]
    pub fn url(this: &Meta) -> String;

    /// Extension for [`BaseAudioContext`] to track registration state.
    #[wasm_bindgen(extends = web_sys::BaseAudioContext)]
    pub type BaseAudioContextExt;

    /// Get the registered state.
    #[wasm_bindgen(method, getter, js_name = __waw_thread_registered)]
    pub fn registered(this: &BaseAudioContextExt) -> Option<bool>;

    /// Set the registered state.
    #[wasm_bindgen(method, setter, js_name = __waw_thread_registered)]
    pub fn set_registered(this: &BaseAudioContextExt, value: bool);
}
