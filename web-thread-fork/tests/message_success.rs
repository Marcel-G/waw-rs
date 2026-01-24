#![cfg(test)]
#![cfg(all(target_family = "wasm", feature = "message"))]

use js_sys::{ArrayBuffer, Uint8Array};
use wasm_bindgen_test::wasm_bindgen_test;
use web_thread::web;
use web_thread::web::message::TransferableWrapper;
use web_thread::web::{JoinHandleExt, ScopeExt};

#[wasm_bindgen_test]
async fn spawn() {
	let buffer = ArrayBuffer::new(1);
	let array = Uint8Array::new(&buffer);
	array.copy_from(&[42]);
	web::spawn_with_message(
		|TransferableWrapper(buffer)| async move {
			let array = Uint8Array::new(&buffer);
			assert_eq!(array.get_index(0), 42);
		},
		TransferableWrapper(buffer.clone()),
	)
	.join_async()
	.await
	.unwrap();

	assert_eq!(buffer.byte_length(), 0);
}

#[wasm_bindgen_test]
async fn nested() {
	web::spawn_async(|| async {
		let buffer = ArrayBuffer::new(1);
		let array = Uint8Array::new(&buffer);
		array.copy_from(&[42]);
		web::spawn_with_message(
			|TransferableWrapper(buffer)| async move {
				let array = Uint8Array::new(&buffer);
				assert_eq!(array.get_index(0), 42);
			},
			TransferableWrapper(buffer.clone()),
		)
		.join_async()
		.await
		.unwrap();

		assert_eq!(buffer.byte_length(), 0);
	})
	.join_async()
	.await
	.unwrap();
}

#[wasm_bindgen_test]
async fn scope() {
	let buffer = ArrayBuffer::new(1);
	let array = Uint8Array::new(&buffer);
	array.copy_from(&[42]);
	web::scope_async(|scope| async {
		scope.spawn_with_message(
			|TransferableWrapper(buffer)| async move {
				let array = Uint8Array::new(&buffer);
				assert_eq!(array.get_index(0), 42);
			},
			TransferableWrapper(buffer.clone()),
		);
	})
	.await;

	assert_eq!(buffer.byte_length(), 0);
}
