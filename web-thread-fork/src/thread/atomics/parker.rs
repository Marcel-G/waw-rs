//! Parker implementation copied from Std.

#![allow(warnings)]

// See <https://github.com/rust-lang/rust/blob/1.75.0/library/std/src/sys_common/thread_parking/futex.rs>.

use std::pin::Pin;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering::{Acquire, Release};
use std::time::Duration;

use super::super::ThreadId;

const PARKED: u32 = u32::MAX;
const EMPTY: u32 = 0;
const NOTIFIED: u32 = 1;

// CHANGED: Derived `Debug`.
#[derive(Debug)]
pub struct Parker {
	// CHANGED: Added corresponding [`ThreadId`].
	id: ThreadId,
	state: AtomicU32,
}

// Notes about memory ordering:
//
// Memory ordering is only relevant for the relative ordering of operations
// between different variables. Even Ordering::Relaxed guarantees a
// monotonic/consistent order when looking at just a single atomic variable.
//
// So, since this parker is just a single atomic variable, we only need to look
// at the ordering guarantees we need to provide to the "outside world".
//
// The only memory ordering guarantee that parking and unparking provide, is
// that things which happened before `unpark()` are visible on the thread
// returning from park() afterwards. Otherwise, it was effectively unparked
// before `unpark()` was called while still consuming the "token".
//
// In other words, `unpark()` needs to synchronize with the part of `park()`
// that consumes the token and returns.
//
// This is done with a release-acquire synchronization, by using
// Ordering::Release when writing NOTIFIED (the "token") in `unpark()`, and
// using Ordering::Acquire when checking for this state in `park()`.
impl Parker {
	// MODIFIED: This can be safe.
	pub fn new(id: ThreadId) -> Self {
		Self {
			id,
			state: AtomicU32::new(EMPTY),
		}
	}

	// Assumes this is only called by the thread that owns the Parker,
	// which means that `self.state != PARKED`.
	// CHANGED: Remove `unsafe` requirement.
	pub fn park(self: Pin<&Self>) {
		// CHANGED: Ascertain safety requirements during runtime.
		assert_eq!(
			self.id,
			super::current_id(),
			"called `park()` not from its corresponding thread"
		);

		// Change `NOTIFIED=>EMPTY` or `EMPTY=>PARKED`, and directly return in the
		// first case.
		if self.state.fetch_sub(1, Acquire) == NOTIFIED {
			return;
		}
		loop {
			// Wait for something to happen, assuming it's still set to PARKED.
			futex_wait(&self.state, PARKED, None);
			// Change `NOTIFIED=>EMPTY` and return in that case.
			if self
				.state
				.compare_exchange(NOTIFIED, EMPTY, Acquire, Acquire)
				.is_ok()
			{
				return;
			} else {
				// Spurious wake up. We loop to try again.
			}
		}
	}

	// Assumes this is only called by the thread that owns the Parker,
	// which means that `self.state != PARKED`.
	// CHANGED: Remove `unsafe` requirement.
	pub fn park_timeout(self: Pin<&Self>, timeout: Duration) {
		// CHANGED: Ascertain safety requirements during runtime.
		assert_eq!(
			self.id,
			super::current_id(),
			"called `park_timeout()` not from its corresponding thread"
		);

		// Change `NOTIFIED=>EMPTY` or `EMPTY=>PARKED`, and directly return in the
		// first case.
		if self.state.fetch_sub(1, Acquire) == NOTIFIED {
			return;
		}
		// Wait for something to happen, assuming it's still set to PARKED.
		futex_wait(&self.state, PARKED, Some(timeout));
		// This is not just a store, because we need to establish a
		// release-acquire ordering with `unpark()`.
		if self.state.swap(EMPTY, Acquire) == NOTIFIED {
			// Woke up because of `unpark()`.
		} else {
			// Timeout or spurious wake up.
			// We return either way, because we can't easily tell if it was the
			// timeout or not.
		}
	}

	#[inline]
	pub fn unpark(self: Pin<&Self>) {
		// Change `PARKED=>NOTIFIED`, `EMPTY=>NOTIFIED`, or `NOTIFIED=>NOTIFIED`, and
		// wake the thread in the first case.
		//
		// Note that even `NOTIFIED=>NOTIFIED` results in a write. This is on
		// purpose, to make sure every `unpark()` has a release-acquire ordering
		// with `park()`.
		if self.state.swap(NOTIFIED, Release) == PARKED {
			futex_wake(&self.state);
		}
	}
}

// See <https://github.com/rust-lang/rust/blob/1.75.0/library/std/src/sys/wasm/atomics/futex.rs>.

/// Wait for a `futex_wake` operation to wake us.
///
/// Returns directly if the futex doesn't hold the expected value.
///
/// Returns false on timeout, and true in all other cases.
pub fn futex_wait(futex: &AtomicU32, expected: u32, timeout: Option<Duration>) -> bool {
	let timeout = timeout
		.and_then(|t| t.as_nanos().try_into().ok())
		.unwrap_or(-1);
	unsafe {
		std::arch::wasm32::memory_atomic_wait32(
			futex as *const AtomicU32 as *mut i32,
			expected as i32,
			timeout,
		) < 2
	}
}

/// Wake up one thread that's blocked on `futex_wait` on this futex.
///
/// Returns true if this actually woke up such a thread,
/// or false if no thread was waiting on this futex.
pub fn futex_wake(futex: &AtomicU32) -> bool {
	unsafe { std::arch::wasm32::memory_atomic_notify(futex as *const AtomicU32 as *mut i32, 1) > 0 }
}
