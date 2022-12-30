use js_sys::{global, Reflect};
use web_sys::window;

/// Will panic if the function is called outside of the browsers main thread
pub fn assert_main() {
    window().expect("Expecting browser main environment.");
}

/// Will panic if the function is called outside of AudioWorkletGlobalScope
pub fn assert_worklet() {
    Reflect::get(&global(), &"registerProcessor".into())
        .ok()
        .filter(|v| v.is_function())
        .expect("Expecting AudioWorklet environment.");
}
