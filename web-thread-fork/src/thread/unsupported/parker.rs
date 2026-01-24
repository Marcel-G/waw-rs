//! Parker implementation inspired by Std but adapted to non-threaded
//! environment.

use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use js_sys::Atomics;

use super::super::ThreadId;
use super::ZERO_ARRAY;

/// Parker implementation.
#[derive(Debug)]
pub(in super::super) struct Parker(AtomicBool);

impl Parker {
	/// Creates a new [`Parker`].
	#[allow(clippy::missing_const_for_fn)]
	pub(in super::super) fn new(_: ThreadId) -> Self {
		Self(AtomicBool::new(false))
	}

	/// Parks the thread.
	pub(in super::super) fn park(self: Pin<&Self>) {
		if self.0.swap(false, Ordering::Relaxed) {
			return;
		}

		wait(None);
		unreachable!("thread should have never woken up");
	}

	/// Parks the thread with a timeout.
	pub(in super::super) fn park_timeout(self: Pin<&Self>, timeout: Duration) {
		if self.0.swap(false, Ordering::Relaxed) {
			return;
		}

		wait(Some(timeout));
	}

	/// Unparks the thread.
	pub(in super::super) fn unpark(self: Pin<&Self>) {
		self.0.store(true, Ordering::Relaxed);
	}
}

/// Wait a specified duration.
fn wait(timeout: Option<Duration>) {
	let timeout = timeout.map_or(f64::INFINITY, super::duration_to_f64_millis);

	let result = ZERO_ARRAY
		.with(|array| Atomics::wait_with_timeout(array, 0, 0, timeout))
		.expect("`Atomic.wait` is not expected to fail");
	debug_assert_eq!(
		result, "timed-out",
		"unexpected return value from `Atomics.wait"
	);
}
