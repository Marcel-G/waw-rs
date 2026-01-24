#![cfg(test)]
#![cfg(all(target_family = "wasm", target_feature = "atomics"))]

use std::sync::mpsc;

use futures_util::future::{self, Either};
use wasm_bindgen_test::wasm_bindgen_test;
use web_thread::web::{JoinHandleExt, ScopedJoinHandleExt};
use web_thread::{web, JoinHandle};

use super::util::{self, Flag, SIGNAL_DURATION};

#[wasm_bindgen_test]
#[should_panic = "`JoinHandle::join()` called after `JoinHandleFuture` polled to completion"]
async fn join_after_await() {
	let mut handle = web_thread::spawn(|| ());
	handle.join_async().await.unwrap();
	let _ = handle.join();
}

#[wasm_bindgen_test]
async fn join_circular() {
	let flag = Flag::new();
	let (sender, receiver) = mpsc::channel();
	let handle = web_thread::spawn({
		let flag = flag.clone();
		move || {
			let handle: JoinHandle<()> = receiver.recv().unwrap();
			let _ = handle.join();
			flag.signal();
		}
	});
	sender.send(handle).unwrap();

	assert!(matches!(
		future::select(flag, util::sleep(SIGNAL_DURATION)).await,
		Either::Right(_)
	));
}

#[wasm_bindgen_test]
async fn join_async_circular() {
	let flag = Flag::new();
	let (sender, receiver) = mpsc::channel();
	let handle = web::spawn_async({
		let flag = flag.clone();
		move || async move {
			let mut handle: JoinHandle<()> = receiver.recv().unwrap();
			let _ = handle.join_async().await;
			flag.signal();
		}
	});
	sender.send(handle).unwrap();

	assert!(matches!(
		future::select(flag, util::sleep(SIGNAL_DURATION)).await,
		Either::Right(_)
	));
}

#[wasm_bindgen_test]
#[should_panic = "`JoinHandleFuture` polled or created after completion"]
async fn join_async() {
	let mut handle = web_thread::spawn(|| ());
	handle.join_async().await.unwrap();
	let _ = handle.join_async().await;
}

#[wasm_bindgen_test]
#[should_panic = "`JoinHandleFuture` polled or created after completion"]
async fn scope_join_async() {
	web::scope_async(|scope| async {
		let mut handle = scope.spawn(|| ());
		handle.join_async().await.unwrap();
		let _ = handle.join_async().await;
	})
	.await;
}
