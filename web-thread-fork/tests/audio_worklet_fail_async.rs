#![cfg(test)]
#![cfg(all(target_family = "wasm", feature = "audio-worklet"))]

use web_sys::BaseAudioContext;
use web_thread::web::audio_worklet::BaseAudioContextExt;

#[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
use super::test_processor::TestProcessor;
use crate::test_audio;

#[cfg(any(not(target_feature = "atomics"), unsupported_spawn))]
async fn test_register(context: BaseAudioContext) {
	context.register_thread(None, || ()).await.unwrap();
}

#[cfg(any(not(target_feature = "atomics"), unsupported_spawn))]
test_audio!(
	register,
	should_panic = "operation not supported on this platform without the atomics target feature \
	                and cross-origin isolation"
);

#[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
async fn test_register_twice(context: BaseAudioContext) {
	context.clone().register_thread(None, || ()).await.unwrap();
	context.register_thread(None, || ()).await.unwrap();
}

#[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
test_audio!(
	register_twice,
	should_panic = "`BaseAudioContext` already registered a thread"
);

#[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
async fn test_not_registered_node(context: BaseAudioContext) {
	context.clone().register_thread(None, || ()).await.unwrap();
	context
		.audio_worklet_node::<TestProcessor>("test", Box::new(|_| None), None)
		.unwrap();
}

#[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
test_audio!(not_registered_node, should_panic = "name");

#[wasm_bindgen_test::wasm_bindgen_test]
#[cfg(any(not(target_feature = "atomics"), unsupported_spawn))]
async fn check_failing_spawn() {
	use js_sys::Array;
	use wasm_bindgen_futures::JsFuture;
	use web_sys::{
		AudioWorkletNode, AudioWorkletNodeOptions, Blob, BlobPropertyBag, OfflineAudioContext, Url,
	};

	let context =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap();

	let sequence = Array::of1(
		&"registerProcessor('test', class extends AudioWorkletProcessor { constructor() { } \
		  process() { } })"
			.into(),
	);
	let property = BlobPropertyBag::new();
	property.set_type("text/javascript");
	let blob = Blob::new_with_str_sequence_and_options(&sequence, &property).unwrap();
	let url = Url::create_object_url_with_blob(&blob).unwrap();

	JsFuture::from(context.audio_worklet().unwrap().add_module(&url).unwrap())
		.await
		.unwrap();

	let options = AudioWorkletNodeOptions::new();
	options.set_processor_options(Some(&Array::of1(&wasm_bindgen::memory())));

	AudioWorkletNode::new_with_options(&context, "'test'", &options).unwrap_err();
}
