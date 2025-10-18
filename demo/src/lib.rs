use utils::set_panic_hook;
use wasm_bindgen::prelude::wasm_bindgen;

use wasm_bindgen_futures::JsFuture;
use web_sys::AudioContext;

pub mod filter;
pub mod oscillator;
mod utils;

async fn polyfill(ctx: &AudioContext) {
    JsFuture::from(
        ctx.audio_worklet()
            .unwrap()
            .add_module(&wasm_bindgen::link_to!(module = "/src/polyfill.min.js"))
            .unwrap(),
    )
    .await
    .unwrap();
}

#[wasm_bindgen(js_name = registerContext)]
/// Create audio context with waw-rs worklets registered
pub async fn register_context() -> AudioContext {
    let ctx = AudioContext::new().unwrap();
    polyfill(&ctx).await;
    waw::register_all(&ctx).await.unwrap();

    ctx
}

#[wasm_bindgen(start)]
pub fn main() {
    set_panic_hook();
}
