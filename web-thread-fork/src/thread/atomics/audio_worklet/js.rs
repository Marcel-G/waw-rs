//! Bindings to the JS API.

use std::ptr::NonNull;

use js_sys::Object;
use wasm_bindgen::prelude::wasm_bindgen;

use super::Data;

#[wasm_bindgen]
extern "C" {
	/// Extension for [`BaseAudioContext`](web_sys::BaseAudioContext).
	pub(super) type BaseAudioContextExt;

	/// Returns our custom `registered` property.
	#[wasm_bindgen(method, getter, js_name = __web_thread_registered)]
	pub(super) fn registered(this: &BaseAudioContextExt) -> Option<bool>;

	/// Sets our custom `registered` property.
	#[wasm_bindgen(method, setter, js_name = __web_thread_registered)]
	pub(super) fn set_registered(this: &BaseAudioContextExt, value: bool);

	/// Type for [`AudioWorkletNodeOptions.processorOptions`](https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletNode/AudioWorkletNode#processoroptions).
	#[wasm_bindgen(extends = Object)]
	#[derive(Default)]
	pub(super) type ProcessorOptions;

	/// Returns our custom `data` property.
	#[wasm_bindgen(method, getter, js_name = __web_thread_data)]
	pub(super) fn data(this: &ProcessorOptions) -> Option<NonNull<Data>>;

	/// Sets our custom `data` property.
	#[wasm_bindgen(method, setter, js_name = __web_thread_data)]
	pub(super) fn set_data(this: &ProcessorOptions, value: NonNull<Data>);
}
