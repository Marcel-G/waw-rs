#![cfg(test)]
#![cfg(target_family = "wasm")]

use js_sys::WebAssembly::Memory;
use js_sys::{Atomics, Int32Array, Object, Reflect, SharedArrayBuffer};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::wasm_bindgen_test;
use web_thread::web;
use web_time::Duration;

#[wasm_bindgen_test]
fn park_no_op() {
	web_thread::park();
	web_thread::park_timeout(Duration::from_secs(1));
	#[allow(deprecated)]
	web_thread::park_timeout_ms(1000);
}

#[wasm_bindgen_test]
#[should_panic = "current thread type cannot be blocked"]
fn sleep() {
	web_thread::sleep(Duration::from_secs(1));
}

#[wasm_bindgen_test]
#[should_panic = "current thread type cannot be blocked"]
fn sleep_ms() {
	#[allow(deprecated)]
	web_thread::sleep_ms(1000);
}

#[wasm_bindgen_test]
#[should_panic = "current thread type cannot be blocked"]
fn join() {
	assert!(
		web::has_spawn_support(),
		"current thread type cannot be blocked"
	);

	let _ = web_thread::spawn(|| ()).join();
}

#[wasm_bindgen_test]
fn has_block_support() {
	assert!(!web::has_block_support());
}

#[wasm_bindgen_test]
fn check_failing_wait() {
	#[wasm_bindgen]
	extern "C" {
		pub(super) type HasSharedArrayBuffer;

		#[wasm_bindgen(method, getter, js_name = SharedArrayBuffer)]
		pub(super) fn shared_array_buffer(this: &HasSharedArrayBuffer) -> JsValue;
	}

	let global: HasSharedArrayBuffer = js_sys::global().unchecked_into();

	// Without cross-origin isolation `SharedArrayBuffer` is unsupported, but we
	// can still use `Atomics.wait` by using a shared Wasm memory, which is a
	// `SharedArrayBuffer` underneath.
	// See <https://github.com/w3c/ServiceWorker/pull/1545>.
	let array = if global.shared_array_buffer().is_undefined() {
		let descriptor = Object::new();
		Reflect::set(&descriptor, &"initial".into(), &1.into()).unwrap();
		Reflect::set(&descriptor, &"maximum".into(), &1.into()).unwrap();
		Reflect::set(&descriptor, &"shared".into(), &true.into()).unwrap();
		Memory::new(&descriptor)
			.map(|memory| Int32Array::new(&memory.buffer()))
			.unwrap()
	} else {
		Int32Array::new(&SharedArrayBuffer::new(4))
	};

	Atomics::wait_with_timeout(&array, 0, 0, 0.).unwrap_err();
}
