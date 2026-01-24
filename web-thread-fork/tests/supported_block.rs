#![cfg(test)]

#[cfg(not(target_family = "wasm"))]
use std::time;

use time::{Duration, Instant};
#[cfg(target_family = "wasm")]
use {wasm_bindgen_test::wasm_bindgen_test, web_thread::web, web_time as time};

#[cfg_attr(not(target_family = "wasm"), test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
fn park() {
	let start = Instant::now();

	let thread = web_thread::current();
	thread.unpark();

	web_thread::park();
	web_thread::park_timeout(Duration::from_secs(1));
	#[allow(deprecated)]
	web_thread::park_timeout_ms(1000);

	let elapsed = start.elapsed();
	// Geckodriver seems unable to measure the time correctly.
	assert!(elapsed.as_millis() >= 1999, "time: {elapsed:?}");
}

#[cfg_attr(not(target_family = "wasm"), test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
fn sleep() {
	let start = Instant::now();

	web_thread::sleep(Duration::from_secs(1));
	#[allow(deprecated)]
	web_thread::sleep_ms(1000);

	let elapsed = start.elapsed();
	// Geckodriver seems unable to measure the time correctly.
	assert!(elapsed.as_millis() >= 1999, "time: {elapsed:?}");
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen_test::wasm_bindgen_test]
fn has_block_support() {
	assert!(web::has_block_support());
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen_test]
async fn scope_async_join() {
	let mut test = 0;

	web::scope_async(|_| async { test = 1 })
		.into_wait()
		.await
		.join_all();

	assert_eq!(test, 1);
}
