//! Bindings to the JS API.

use js_sys::Object;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
extern "C" {
	/// Dictionary type of [`MemoryDescriptor`](https://developer.mozilla.org/en-US/docs/WebAssembly/JavaScript_interface/Memory/Memory#memorydescriptor).
	#[wasm_bindgen(extends = Object)]
	pub(super) type MemoryDescriptor;

	/// Setter for [`MemoryDescriptor.initial`](https://developer.mozilla.org/en-US/docs/WebAssembly/JavaScript_interface/Memory/Memory#initial) property.
	#[wasm_bindgen(method, setter, js_name = initial)]
	pub(super) fn set_initial(this: &MemoryDescriptor, value: i32);

	/// Setter for [`MemoryDescriptor.maximum`](https://developer.mozilla.org/en-US/docs/WebAssembly/JavaScript_interface/Memory/Memory#maximum) property.
	#[wasm_bindgen(method, setter, js_name = maximum)]
	pub(super) fn set_maximum(this: &MemoryDescriptor, value: i32);

	/// Setter for [`MemoryDescriptor.shared`](https://developer.mozilla.org/en-US/docs/WebAssembly/JavaScript_interface/Memory/Memory#shared) property.
	#[wasm_bindgen(method, setter, js_name = shared)]
	pub(super) fn set_shared(this: &MemoryDescriptor, value: bool);
}
