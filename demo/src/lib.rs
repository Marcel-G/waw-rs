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
/// Create audio context with waw-rs worklets registered.
///
/// # Arguments
///
/// * `shim_url` - Optional custom URL for the wasm-bindgen JS shim.
///   Use this when bundlers like Vite change the location of the JS shim file.
///   Pass `undefined` to use the default `import.meta.url` detection.
///
/// # Example
///
/// ```js
/// import init, { registerContext } from './pkg/waw_demo';
///
/// await init();
///
/// // Default usage (works in development)
/// const context = await registerContext();
///
/// // With custom shim URL (for production builds with bundlers)
/// const context = await registerContext('/assets/waw_demo.js');
/// ```
pub async fn register_context(shim_url: Option<String>) -> AudioContext {
    let ctx = AudioContext::new().unwrap();
    polyfill(&ctx).await;
    waw::register_all(&ctx, shim_url.as_deref()).await.unwrap();

    ctx
}

#[wasm_bindgen(start)]
pub fn main() {
    set_panic_hook();
}
