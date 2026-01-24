#![cfg(test)]
#![cfg(all(target_family = "wasm", feature = "audio-worklet"))]

use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::{AudioContext, OfflineAudioContext};
use web_thread::web::audio_worklet::BaseAudioContextExt;

use super::test_processor::TestProcessor;

#[wasm_bindgen_test]
#[should_panic = "`register_thread()` has to be called on this context first"]
fn node() {
	AudioContext::new()
		.unwrap()
		.audio_worklet_node::<TestProcessor>("test", Box::new(|_| None), None)
		.unwrap();
}

#[wasm_bindgen_test]
#[should_panic = "`register_thread()` has to be called on this context first"]
fn offline_node() {
	OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
		.unwrap()
		.audio_worklet_node::<TestProcessor>("test", Box::new(|_| None), None)
		.unwrap();
}
