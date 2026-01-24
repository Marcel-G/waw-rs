//! Bindings to the JS API.

#[cfg(feature = "audio-worklet")]
use js_sys::Array;
use js_sys::WebAssembly::Global;
use js_sys::{Object, Promise};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use super::super::js::GlobalExt;

#[wasm_bindgen]
extern "C" {
	/// Returns the constructor of [`Worker`](https://developer.mozilla.org/en-US/docs/Web/API/Worker).
	#[wasm_bindgen(method, getter, js_name = Worker)]
	pub(super) fn worker(this: &GlobalExt) -> JsValue;

	/// Type of [`import.meta`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/import.meta).
	pub(super) type Meta;

	/// Returns [`import.meta`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/import.meta).
	#[wasm_bindgen(thread_local, js_namespace = import, js_name = meta)]
	pub(super) static META: Meta;

	/// See [`import.meta.url`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/import.meta#url).
	#[wasm_bindgen(method, getter)]
	pub(super) fn url(this: &Meta) -> String;

	/// Returns [`Atomics.waitAsync`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Atomics/waitAsync).
	#[wasm_bindgen(thread_local, js_namespace = Atomics, js_name = waitAsync)]
	pub(super) static HAS_WAIT_ASYNC: JsValue;

	/// Type for [`Atomics.waitAsync`s return value](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Atomics/waitAsync#return_value).
	pub(super) type WaitAsyncResult;

	/// [`async`] property of [`Atomics.waitAsync()`s return value].
	///
	/// [`async`]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Atomics/waitAsync#async
	/// [`Atomics.waitAsync()`s return value]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Atomics/waitAsync#return_value
	#[wasm_bindgen(method, getter, js_name = async)]
	pub(super) fn async_(this: &WaitAsyncResult) -> bool;

	/// [`value`] property of [`Atomics.waitAsync`s return value].
	///
	/// [`value`]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Atomics/waitAsync#value_2
	/// [`Atomics.waitAsync`s return value]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Atomics/waitAsync#return_value
	#[wasm_bindgen(method, getter)]
	pub(super) fn value(this: &WaitAsyncResult) -> Promise;

	/// Type of [`WebAssembly.Module.exports()`s return value](https://developer.mozilla.org/en-US/docs/WebAssembly/JavaScript_interface/Module/exports_static).
	pub(super) type Exports;

	/// [`wasm-bindgen`](wasm_bindgen)s thread destruction function.
	///
	/// # Safety
	///
	/// - The thread is not allowed to be used while or after this function is
	///   executed.
	/// - Must not be called twice for the same thread.
	#[wasm_bindgen(method, js_name = __wbindgen_thread_destroy)]
	pub(super) unsafe fn thread_destroy(
		this: &Exports,
		tls_base: &Global,
		stack_alloc: &Global,
		stack_size: Option<usize>,
	);

	/// Base address of [`wasm-bindgen`](wasm_bindgen)s TLS memory.
	#[wasm_bindgen(method, getter, js_name = __tls_base)]
	pub(super) fn tls_base(this: &Exports) -> Global;

	/// Base address of [`wasm-bindgen`](wasm_bindgen)s thread stack memory.
	#[wasm_bindgen(method, getter, js_name = __stack_alloc)]
	pub(super) fn stack_alloc(this: &Exports) -> Global;

	/// Dictionary type of [`GlobalDescriptor`](https://developer.mozilla.org/en-US/docs/WebAssembly/JavaScript_interface/Global/Global#descriptor).
	#[wasm_bindgen(extends = Object)]
	pub(super) type GlobalDescriptor;

	/// Setter for [`GlobalDescriptor.value`](https://developer.mozilla.org/en-US/docs/WebAssembly/JavaScript_interface/Global/Global#descriptor) property.
	#[wasm_bindgen(method, setter, js_name = value)]
	pub(super) fn set_value(this: &GlobalDescriptor, value: &str);
}

#[cfg(feature = "audio-worklet")]
#[wasm_bindgen]
extern "C" {
	/// Extension for [`Array`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array).
	#[wasm_bindgen(js_name = Array)]
	pub(super) type ArrayExt;

	/// [`Array.of()`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array/of)
	/// with six arguments.
	#[wasm_bindgen(static_method_of = ArrayExt, js_class = Array, js_name = of)]
	pub fn of6(
		a: &JsValue,
		b: &JsValue,
		c: &JsValue,
		d: &JsValue,
		e: &JsValue,
		f: &JsValue,
	) -> Array;
}
