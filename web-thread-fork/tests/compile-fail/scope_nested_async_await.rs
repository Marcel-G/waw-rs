//@only-target-wasm32-unknown-unknown

async fn scope_nested_async_await() {
	let mut test = String::new();

	let _future = web_thread::web::scope_async(|scope| async {
		scope.spawn(|| test.push_str("test"));
	})
	.into_wait()
	.await;

	drop(test);
    //~^ ERROR: cannot move out of `test` because it is borrowed
}
