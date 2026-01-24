#![cfg(test)]
#![cfg(target_family = "wasm")]

use web_thread::web;

#[wasm_bindgen_test::wasm_bindgen_test]
async fn yield_now() {
	use std::cell::Cell;
	use std::rc::Rc;

	use wasm_bindgen::closure::Closure;
	use wasm_bindgen::{JsCast, JsValue};
	use web_sys::MessageChannel;
	use web_thread::web::YieldTime;

	let channel = MessageChannel::new().unwrap();
	let received = Rc::new(Cell::new(false));

	let callback = Closure::<dyn Fn()>::new({
		let received = Rc::clone(&received);
		move || received.set(true)
	});
	channel
		.port1()
		.set_onmessage(Some(callback.as_ref().unchecked_ref()));

	channel.port2().post_message(&JsValue::UNDEFINED).unwrap();
	assert!(!received.get());

	web::yield_now_async(YieldTime::UserVisible).await;
	// Order of events with `MessagePort.postMessage()` is undefined, so we wait a
	// second time.
	web::yield_now_async(YieldTime::UserVisible).await;

	assert!(received.get());
}
