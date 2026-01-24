#![cfg(test)]
#![cfg(all(target_family = "wasm", feature = "message", feature = "audio-worklet"))]

use js_sys::{ArrayBuffer, Uint8Array};
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::OfflineAudioContext;
use web_thread::web;
use web_thread::web::audio_worklet::BaseAudioContextExt;
use web_thread::web::message::TransferableWrapper;
use web_thread::web::JoinHandleExt;

use super::util::Flag;

#[wasm_bindgen_test]
async fn message() {
	let buffer = ArrayBuffer::new(1);
	let array = Uint8Array::new(&buffer);
	array.copy_from(&[42]);
	let context =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap();

	let flag = Flag::new();
	context
		.register_thread_with_message(
			None,
			{
				let flag = flag.clone();
				move |TransferableWrapper::<ArrayBuffer>(buffer)| {
					let array = Uint8Array::new(&buffer);
					assert_eq!(array.get_index(0), 42);

					flag.signal();
				}
			},
			TransferableWrapper(buffer.clone()),
		)
		.await
		.unwrap();

	assert_eq!(buffer.byte_length(), 0);

	flag.await;
}

#[wasm_bindgen_test]
async fn nested() {
	let context =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap();

	let (sender, receiver) = async_channel::bounded(1);
	context
		.register_thread(None, move || {
			let buffer = ArrayBuffer::new(1);
			let array = Uint8Array::new(&buffer);
			array.copy_from(&[42]);

			let handle = web::spawn_with_message(
				|TransferableWrapper(buffer)| async move {
					let array = Uint8Array::new(&buffer);
					assert_eq!(array.get_index(0), 42);
				},
				TransferableWrapper(buffer.clone()),
			);
			assert_eq!(buffer.byte_length(), 0);

			sender.try_send(handle).unwrap();
		})
		.await
		.unwrap();

	receiver.recv().await.unwrap().join_async().await.unwrap();
}
