#![cfg(target_arch = "wasm32")]

use utils::set_panic_hook;
use wasm_bindgen::prelude::wasm_bindgen;

pub mod gain;
pub mod oscillator;
mod utils;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen(start)]
pub fn main() {
    set_panic_hook();
}
