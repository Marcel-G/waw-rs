#![cfg(test)]
#![cfg(target_family = "wasm")]

use wasm_bindgen_test::wasm_bindgen_test;
use web_thread::web::{BuilderExt, ScopeExt};
use web_thread::{web, Builder};

#[wasm_bindgen_test]
#[should_panic = "operation not supported on this platform without the atomics target feature and \
                  cross-origin isolation"]
fn spawn() {
	web_thread::spawn(|| ());
}

#[wasm_bindgen_test]
#[should_panic = "operation not supported on this platform without the atomics target feature and \
                  cross-origin isolation"]
fn spawn_async() {
	web::spawn_async(|| async {});
}

#[wasm_bindgen_test]
#[should_panic = "operation not supported on this platform without the atomics target feature and \
                  cross-origin isolation"]
fn builder() {
	Builder::new()
		.stack_size(usize::MAX)
		.name(String::from("test"))
		.spawn(|| ())
		.unwrap();
}

#[wasm_bindgen_test]
#[should_panic = "operation not supported on this platform without the atomics target feature and \
                  cross-origin isolation"]
fn builder_async() {
	Builder::new()
		.stack_size(usize::MAX)
		.name(String::from("test"))
		.spawn_async(|| async {})
		.unwrap();
}

#[wasm_bindgen_test]
#[should_panic = "operation not supported on this platform without the atomics target feature and \
                  cross-origin isolation"]
fn scope() {
	web_thread::scope(|scope| {
		scope.spawn(|| ());
	});
}

#[wasm_bindgen_test]
#[should_panic = "operation not supported on this platform without the atomics target feature and \
                  cross-origin isolation"]
fn scope_builder() {
	web_thread::scope(|scope| {
		Builder::new().spawn_scoped(scope, || ()).unwrap();
	});
}

#[wasm_bindgen_test]
#[should_panic = "operation not supported on this platform without the atomics target feature and \
                  cross-origin isolation"]
fn scope_async() {
	web_thread::scope(|scope| {
		scope.spawn_async(|| async {});
	});
}

#[wasm_bindgen_test]
#[should_panic = "operation not supported on this platform without the atomics target feature and \
                  cross-origin isolation"]
fn scope_builder_async() {
	web_thread::scope(|scope| {
		Builder::new()
			.spawn_scoped_async(scope, || async {})
			.unwrap();
	});
}

#[wasm_bindgen_test]
fn has_thread_support() {
	assert!(!web::has_spawn_support());
}

#[wasm_bindgen_test]
#[cfg(target_feature = "atomics")]
fn check_failing_spawn() {
	use js_sys::Array;
	use wasm_bindgen::prelude::wasm_bindgen;
	use wasm_bindgen::{JsCast, JsValue};
	use web_sys::Worker;

	#[wasm_bindgen]
	extern "C" {
		pub(super) type HasWorker;

		#[wasm_bindgen(method, getter, js_name = Worker)]
		pub(super) fn worker(this: &HasWorker) -> JsValue;
	}

	let global: HasWorker = js_sys::global().unchecked_into();

	if !global.worker().is_undefined() {
		let worker = Worker::new("data:,").unwrap();
		worker
			.post_message_with_transfer(
				&wasm_bindgen::memory(),
				&Array::of1(&wasm_bindgen::memory()),
			)
			.unwrap_err();
	}
}
