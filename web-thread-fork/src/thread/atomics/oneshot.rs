//! Async oneshot channel.

use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, TryLockError, Weak};
use std::task::{Context, Poll};
use std::{any, mem};

use atomic_waker::AtomicWaker;

/// Creates the oneshot channel.
pub(super) fn channel<T>() -> (Sender<T>, Receiver<T>) {
	let shared = Arc::new(Shared {
		value: Mutex::new(State::Waiting),
		cvar: Condvar::new(),
		waker: AtomicWaker::new(),
	});

	(
		Sender(Some(Arc::downgrade(&shared))),
		Receiver(Some(shared)),
	)
}

/// Shared state between [`Sender`] and [`Receiver`].
struct Shared<T> {
	/// [`Mutex`] holding the returned value.
	value: Mutex<State<T>>,
	/// [`Condvar`] to wake up any thread waiting on the return value.
	cvar: Condvar,
	/// Registered [`Waker`](std::task::Waker) to be notified when the thread is
	/// finished.
	waker: AtomicWaker,
}

/// Current state of the value.
enum State<T> {
	/// Waiting for a value to be delivered.
	Waiting,
	/// [`Sender`] dropped.
	Dropped,
	/// Value taken.
	Taken,
	/// Value arrived.
	Result(T),
}

impl<T> Debug for Shared<T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("Shared")
			.field("value", &any::type_name_of_val(&self.value))
			.field("cvar", &self.cvar)
			.field("waker", &self.waker)
			.finish()
	}
}

impl<T> State<T> {
	/// Takes the current state if there is one.
	fn take(&mut self) -> Option<Self> {
		match self {
			Self::Waiting => None,
			Self::Dropped | Self::Result(_) => Some(mem::replace(self, Self::Taken)),
			Self::Taken => Some(Self::Taken),
		}
	}
}

/// Sender.
pub(super) struct Sender<T>(Option<Weak<Shared<T>>>);

impl<T> Debug for Sender<T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter.debug_tuple("Sender").field(&self.0).finish()
	}
}

impl<T> Drop for Sender<T> {
	fn drop(&mut self) {
		#[allow(clippy::significant_drop_in_scrutinee)]
		self.take_with(|shared, mut state| match state.deref() {
			State::Waiting => {
				*state = State::Dropped;
				drop(state);
				shared.cvar.notify_one();
				shared.waker.wake();
			}
			State::Taken | State::Result(_) => unreachable!("left state intact after sending"),
			State::Dropped => unreachable!("somehow dropped twice"),
		});
	}
}

impl<T> Sender<T> {
	/// Blocks or spinloops depending on support to get the inner [`State`].
	#[allow(clippy::significant_drop_tightening)]
	fn take_with(&mut self, task: impl FnOnce(&Shared<T>, MutexGuard<'_, State<T>>)) {
		if let Some(shared) = self.0.take().and_then(|shared| shared.upgrade()) {
			loop {
				let inner = match shared.value.try_lock() {
					Ok(inner) => inner,
					Err(TryLockError::Poisoned(error)) => error.into_inner(),
					Err(TryLockError::WouldBlock) => continue,
				};
				task(&shared, inner);
				break;
			}
		}
	}

	/// Send `value` to [`Receiver`].
	pub(super) fn send(mut self, value: T) {
		self.take_with(move |shared, mut state| {
			*state = State::Result(value);
			drop(state);
			shared.cvar.notify_one();
			shared.waker.wake();
		});
	}
}

/// Receiver.
pub(super) struct Receiver<T>(Option<Arc<Shared<T>>>);

impl<T> Debug for Receiver<T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter.debug_tuple("Receiver").field(&self.0).finish()
	}
}

impl<T> Future for Receiver<T> {
	type Output = Option<T>;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let Some(state) = self.0.take() else {
			panic!("polled after completion")
		};

		let mut value = match state.value.try_lock() {
			Ok(mut value) => value.take(),
			Err(TryLockError::Poisoned(error)) => error.into_inner().take(),
			Err(TryLockError::WouldBlock) => None,
		};

		if value.is_none() {
			state.waker.register(cx.waker());

			value = match state.value.try_lock() {
				Ok(mut value) => value.take(),
				Err(TryLockError::Poisoned(error)) => error.into_inner().take(),
				Err(TryLockError::WouldBlock) => None,
			};
		}

		if let Some(state) = value {
			match state {
				State::Result(value) => Poll::Ready(Some(value)),
				State::Dropped => Poll::Ready(None),
				State::Waiting => unreachable!("wrong state returns by `State::take()`"),
				State::Taken => unreachable!("falsely inserted wrong state"),
			}
		} else {
			self.0 = Some(state);
			Poll::Pending
		}
	}
}

impl<T> Receiver<T> {
	/// Returns [`true`] if value is ready to be received.
	pub(super) fn is_ready(&self) -> bool {
		let Some(state) = self.0.as_ref() else {
			return true;
		};

		loop {
			#[allow(clippy::significant_drop_in_scrutinee)]
			match state.value.try_lock().as_deref() {
				Ok(State::Result(_) | State::Dropped | State::Taken) => return true,
				Err(TryLockError::Poisoned(error)) => {
					return !matches!(error.get_ref().deref(), State::Waiting)
				}
				Ok(State::Waiting) => return false,
				Err(TryLockError::WouldBlock) => (),
			}
		}
	}

	/// Block until value is received.
	pub(super) fn receive(self) -> Option<T> {
		let state = self.0.expect("value already taken by polling");

		let mut value = if super::super::has_block_support() {
			state.value.lock().unwrap_or_else(PoisonError::into_inner)
		} else {
			match state.value.try_lock() {
				Ok(value) => value,
				Err(TryLockError::Poisoned(error)) => error.into_inner(),
				Err(TryLockError::WouldBlock) => panic!("current thread type cannot be blocked"),
			}
		};

		loop {
			#[allow(clippy::significant_drop_in_scrutinee)]
			match value.take() {
				Some(State::Result(value)) => return Some(value),
				None => (),
				Some(State::Dropped) => return None,
				Some(State::Waiting) => unreachable!("wrong state returns by `State::take()`"),
				Some(State::Taken) => unreachable!("polled future left state intact"),
			}

			assert!(
				super::super::has_block_support(),
				"current thread type cannot be blocked"
			);

			value = state
				.cvar
				.wait(value)
				.unwrap_or_else(PoisonError::into_inner);
		}
	}
}

#[cfg(test)]
mod test {
	use wasm_bindgen_test::wasm_bindgen_test;

	wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

	#[wasm_bindgen_test]
	fn drop() {
		let (.., receiver) = super::channel::<()>();
		assert!(receiver.receive().is_none());
	}

	#[wasm_bindgen_test]
	async fn drop_async() {
		let (.., receiver) = super::channel::<()>();
		assert!(receiver.await.is_none());
	}
}
