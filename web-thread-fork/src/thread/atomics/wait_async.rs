//! Polyfill for `Atomics.waitAsync`.

use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::task::{ready, Context, Poll, Waker};

use js_sys::{Array, Atomics};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::Worker;

use super::js::{self, WaitAsyncResult};
use super::{MEMORY, MEMORY_ARRAY};

/// Arbitrary limited amount of workers to cache.
const POLYFILL_WORKER_CACHE: usize = 10;

/// Mimics the interface we need from [`Atomics`].
#[derive(Debug)]
pub(super) struct WaitAsync(Option<State>);

/// State for [`WaitAsync`] [`Future`] implementation.
#[derive(Debug)]
enum State {
	/// Atomic request was ready immediately.
	Ready,
	/// [`Promise`](js_sys::Promise) returned by [`Atomics::wait_async()`].
	WaitAsync(JsFuture),
	/// Polyfill implementation of [`Atomics::wait_async()`].
	Polyfill(Rc<Shared>),
}

/// Shared state for polyfill implementation.
#[derive(Debug)]
struct Shared {
	/// [`true`] when finished.
	finished: Cell<bool>,
	/// Stores [`Waker`] for callback.
	waker: RefCell<Option<Waker>>,
}

impl Future for WaitAsync {
	type Output = ();

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let state = self
			.0
			.as_mut()
			.expect("`WaitAsync` polled after completion");

		match state {
			State::Ready => {
				self.0.take();
				Poll::Ready(())
			}
			State::WaitAsync(future) => {
				ready!(Pin::new(future).poll(cx))
					.expect("`Promise` returned by `Atomics.waitAsync` should never throw");
				self.0.take();
				Poll::Ready(())
			}
			State::Polyfill(shared) => {
				if shared.finished.get() {
					self.0.take();
					Poll::Ready(())
				} else {
					*shared.waker.borrow_mut() = Some(cx.waker().clone());
					Poll::Pending
				}
			}
		}
	}
}

impl WaitAsync {
	/// Mimics the interface we need from [`Atomics::wait_async`].
	pub(super) fn wait(value: &AtomicI32, check: i32) -> Self {
		thread_local! {
			static HAS_WAIT_ASYNC: bool = !js::HAS_WAIT_ASYNC.with(JsValue::is_undefined);
		}

		// Short-circuit before having to go through FFI.
		if value.load(Ordering::Relaxed) != check {
			return Self(Some(State::Ready));
		}

		let index = super::i32_to_buffer_index(value.as_ptr());

		if HAS_WAIT_ASYNC.with(bool::clone) {
			let result: WaitAsyncResult = MEMORY_ARRAY
				.with(|array| Atomics::wait_async(array, index, check))
				.expect("`Atomics.waitAsync` is not expected to fail")
				.unchecked_into();

			Self(Some(if result.async_() {
				State::WaitAsync(JsFuture::from(result.value()))
			} else {
				State::Ready
			}))
		} else {
			Self::wait_polyfill(index, check)
		}
	}

	/// Polyfills [`Atomics::wait_async`] if not available.
	fn wait_polyfill(index: u32, check: i32) -> Self {
		thread_local! {
			/// Object URL to the worker script.
			static URL: String = wasm_bindgen::link_to!(module = "/src/thread/atomics/script/wait_async.min.js");
			/// Holds cached workers.
			static WORKERS: RefCell<Vec<Worker>> = const { RefCell::new(Vec::new()) };
		}

		let worker = WORKERS.with(|workers| {
			if let Some(worker) = workers.borrow_mut().pop() {
				return worker;
			}

			URL.with(|url| Worker::new(url))
				.expect("`new Worker()` is not expected to fail with a local script")
		});

		let shared = Rc::new(Shared {
			finished: Cell::new(false),
			waker: RefCell::new(None),
		});

		let onmessage_callback = Closure::once_into_js({
			let shared = Rc::clone(&shared);
			let worker = worker.clone();

			move || {
				WORKERS.with(move |workers| {
					let mut workers = workers.borrow_mut();
					workers.push(worker);
					workers.truncate(POLYFILL_WORKER_CACHE);
				});

				shared.finished.set(true);

				if let Some(waker) = shared.waker.borrow_mut().take() {
					waker.wake();
				}
			}
		});
		worker.set_onmessage(Some(onmessage_callback.unchecked_ref()));

		let message =
			MEMORY.with(|memory| Array::of3(memory, &JsValue::from(index), &JsValue::from(check)));

		worker
			.post_message(&message)
			.expect("`Worker.postMessage` is not expected to fail without a `transfer` object");

		Self(Some(State::Polyfill(shared)))
	}
}
