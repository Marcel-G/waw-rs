#![cfg(target_family = "wasm")]

use std::pin::pin;

use wasm_bindgen_test::wasm_bindgen_test;
use web_thread::web;

#[wasm_bindgen_test]
#[should_panic = "`ScopeFuture` polled after completion"]
async fn scope_async() {
	let mut handle = pin!(web::scope_async(|_| async {}));
	(&mut handle).await;
	handle.await;
}

#[wasm_bindgen_test]
#[should_panic = "`ScopeFuture` polled after completion"]
async fn scope_async_into_wait() {
	let mut handle = pin!(web::scope_async(|_| async {}).into_wait());
	let _future = (&mut handle).await;
	let _future = handle.await;
}

#[wasm_bindgen_test]
#[should_panic = "`ScopeFuture` polled after completion"]
async fn scope_async_into_wait_wait() {
	let mut into_handle = pin!(web::scope_async(|_| async {}).into_wait());
	let wait_handle = (&mut into_handle).await;
	wait_handle.await;
	let _future = into_handle.await;
}

#[wasm_bindgen_test]
#[should_panic = "`ScopeFuture` polled after completion"]
async fn scope_async_wait() {
	let mut handle = web::scope_async(|_| async {}).into_wait().await;
	(&mut handle).await;
	handle.await;
}

#[wasm_bindgen_test]
#[should_panic = "called after `ScopeJoinFuture` was polled to completion"]
async fn scope_async_join() {
	let mut handle = web::scope_async(|_| async {}).into_wait().await;
	(&mut handle).await;
	handle.join_all();
}
