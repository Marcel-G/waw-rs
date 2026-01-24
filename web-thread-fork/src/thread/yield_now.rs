//! Implementation of [`yield_now()`] and [`YieldNowFuture`].

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{ready, Context, Poll, Waker};
use std::thread;

use js_sys::Object;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{AbortController, MessageChannel, MessagePort};

use super::global::Global;
use super::js::{SchedulerPostTaskOptions, TaskPriority, WindowExt};
use crate::web::YieldTime;

/// See [`std::thread::yield_now()`].
///
/// # Notes
///
/// This call is no-op. Alternatively consider using
/// [`web::yield_now_async()`](crate::web::yield_now_async).
pub fn yield_now() {
	thread::yield_now();
}

/// Waits for yielding to the event loop to happen.
#[derive(Debug)]
pub(crate) struct YieldNowFuture(Option<State>);

/// State of [`YieldNowFuture`].
#[derive(Debug)]
enum State {
	/// Used [`Scheduler.postTask()`](https://developer.mozilla.org/en-US/docs/Web/API/Scheduler/postTask).
	Scheduler {
		/// [`Future`].
		future: JsFuture,
		/// Abort when dropped.
		controller: AbortController,
	},
	/// Used [`Window.requestIdleCallback()`](https://developer.mozilla.org/en-US/docs/Web/API/Window/requestIdleCallback).
	Idle {
		/// [`WakerData`].
		waker: Rc<RefCell<WakerData>>,
		/// Callback to wake up the [`Future`].
		_callback: Closure<dyn FnMut()>,
		/// Abort when dropped.
		handle: u32,
	},
	/// Used [`MessagePort.postMessage()`](https://developer.mozilla.org/en-US/docs/Web/API/MessagePort/postMessage).
	Channel {
		/// [`WakerData`].
		waker: Rc<RefCell<WakerData>>,
		/// Callback to wake up the [`Future`].
		_callback: Closure<dyn FnMut()>,
		/// Abort when dropped.
		port: MessagePort,
	},
	/// Yielding to the event loop not supported.
	None,
}

impl Drop for YieldNowFuture {
	fn drop(&mut self) {
		if let Some(state) = self.0.take() {
			match state {
				State::Scheduler { controller, .. } => controller.abort(),
				State::Idle { handle, .. } => Global::with(|global| {
					let Global::Window(window) = global else {
						unreachable!("expected `Window`")
					};
					window.cancel_idle_callback(handle);
				}),
				State::Channel { port, .. } => {
					port.set_onmessage(None);
					port.close();
				}
				State::None => (),
			}
		}
	}
}

impl Future for YieldNowFuture {
	type Output = ();

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		match self
			.0
			.as_mut()
			.expect("`YieldNowFuture` polled after completion")
		{
			State::Scheduler { future, .. } => {
				ready!(Pin::new(future).poll(cx)).expect("unexpected failure in empty `Promise`");
			}
			State::Idle { waker, .. } | State::Channel { waker, .. } => {
				ready!(waker.borrow_mut().poll(cx));
			}
			State::None => (),
		}

		self.0.take().expect("found empty `State`");
		Poll::Ready(())
	}
}

impl YieldNowFuture {
	/// Implementation for [`crate::web::yield_now_async()`].
	pub(crate) fn new(time: YieldTime) -> Self {
		thread_local! {
			static HAS_SCHEDULER: bool = Global::with_window_or_worker(|global| !global.has_scheduler().is_undefined()).unwrap_or(false);
			static HAS_REQUEST_IDLE_CALLBACK: bool = Global::with(|global| {
				if let Global::Window(window) = global {
					let window: &WindowExt = window.unchecked_ref();
					!window.has_request_idle_callback().is_undefined()
				} else {
					false
				}
			});
			static CATCH_CALLBACK: Closure<dyn FnMut(JsValue)> = Closure::new(|_| ());
		}

		match time {
			YieldTime::UserBlocking | YieldTime::UserVisible | YieldTime::Background
				if HAS_SCHEDULER.with(bool::clone) =>
			{
				Global::with_window_or_worker(|global| {
					let options: SchedulerPostTaskOptions = Object::new().unchecked_into();
					let controller = AbortController::new()
						.expect("`new AbortController` is not expected to fail");
					options.set_signal(&controller.signal());

					match time {
						YieldTime::UserBlocking => {
							options.set_priority(TaskPriority::UserBlocking);
						}
						YieldTime::UserVisible => (),
						YieldTime::Background => {
							options.set_priority(TaskPriority::Background);
						}
						YieldTime::Idle => unreachable!("found invalid `YieldTime`"),
					}

					let future = JsFuture::from(CATCH_CALLBACK.with(|closure| {
						global
							.scheduler()
							.post_task_with_options(closure.as_ref().unchecked_ref(), &options)
							.catch(closure)
					}));

					Self(Some(State::Scheduler { future, controller }))
				})
				.expect("found invalid global context despite previous check")
			}
			YieldTime::Idle if HAS_REQUEST_IDLE_CALLBACK.with(bool::clone) => {
				Global::with(|global| {
					let Global::Window(window) = global else {
						unreachable!("expected `Window`")
					};

					let (waker, callback) = WakerData::new();
					let handle = window
						.request_idle_callback(callback.as_ref().unchecked_ref())
						.expect("`requestIdleCallback()` is not expected to fail");

					Self(Some(State::Idle {
						waker,
						_callback: callback,
						handle,
					}))
				})
			}
			// `MessageChannel` can't be instantiated in a worklet.
			_ => Global::with_window_or_worker(|_| {
				let channel =
					MessageChannel::new().expect("`new MessageChannel` is not expected to fail");
				let port1 = channel.port1();

				let (waker, callback) = WakerData::new();
				port1.set_onmessage(Some(callback.as_ref().unchecked_ref()));
				channel
					.port2()
					.post_message(&JsValue::UNDEFINED)
					.expect("failed to send empty message");

				Self(Some(State::Channel {
					waker,
					_callback: callback,
					port: port1,
				}))
			})
			.unwrap_or(Self(Some(State::None))),
		}
	}
}

/// Data required to wake up the [`Future`].
#[derive(Debug)]
struct WakerData {
	/// Represents if it has completed or not.
	completed: bool,
	/// Stores the [`Waker`].
	waker: Option<Waker>,
}

impl WakerData {
	/// Creates a new [`WakerData`] and a corresponding [`Closure`].
	fn new() -> (Rc<RefCell<Self>>, Closure<dyn FnMut()>) {
		let this = Rc::new(RefCell::new(Self {
			completed: false,
			waker: None,
		}));
		let callback = Closure::once({
			let this = Rc::clone(&this);
			move || this.borrow_mut().complete()
		});
		(this, callback)
	}

	/// Completes the [`Future`] and wakes up a registered [`Waker`].
	fn complete(&mut self) {
		self.completed = true;

		if let Some(waker) = self.waker.take() {
			waker.wake();
		}
	}

	/// Polls the [`WakerData`].
	fn poll(&mut self, cx: &Context<'_>) -> Poll<()> {
		if self.completed {
			Poll::Ready(())
		} else {
			self.waker = Some(cx.waker().clone());
			Poll::Pending
		}
	}
}
