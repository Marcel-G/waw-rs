//! Implementation without the atomics target feature enabled.

#[cfg(feature = "audio-worklet")]
pub(super) mod audio_worklet;
mod js;
mod parker;

use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::marker::PhantomData;
use std::task::{Context, Poll};
use std::time::Duration;
use std::{io, thread};

use js::MemoryDescriptor;
use js_sys::WebAssembly::Memory;
use js_sys::{Atomics, Int32Array, Object, SharedArrayBuffer};
use wasm_bindgen::JsCast;

pub(super) use self::parker::Parker;
use super::js::CROSS_ORIGIN_ISOLATED;
use super::ScopedJoinHandle;

/// Implementation of [`std::thread::Builder`].
#[derive(Debug)]
pub(super) struct Builder;

impl Builder {
	/// Implementation of [`std::thread::Builder::new()`].
	#[allow(clippy::missing_const_for_fn)]
	pub(super) fn new() -> Self {
		Self
	}

	/// Implementation of [`std::thread::Builder::name()`].
	pub(super) fn name(self, _: String) -> Self {
		self
	}

	/// Implementation of [`std::thread::Builder::spawn()`].
	#[allow(clippy::unused_self)]
	pub(super) fn spawn<F, T>(self, _: F) -> io::Result<JoinHandle<T>>
	where
		F: FnOnce() -> T,
	{
		unreachable!("reached `spawn()` without atomics target feature")
	}

	/// Implementation for
	/// [`BuilderExt::spawn_async()`](crate::web::BuilderExt::spawn_async).
	#[allow(clippy::unused_self)]
	pub(super) fn spawn_async_internal<F1, F2, T>(self, _: F1) -> io::Result<JoinHandle<T>>
	where
		F1: FnOnce() -> F2,
		F2: Future<Output = T>,
	{
		unreachable!("reached `spawn()` without atomics target feature")
	}

	/// Implementation for
	/// [`BuilderExt::spawn_with_message()`](crate::web::BuilderExt::spawn_with_message).
	#[cfg(feature = "message")]
	#[allow(clippy::unused_self)]
	pub(super) fn spawn_with_message_internal<F1, F2, T, M>(
		self,
		_: F1,
		_: M,
	) -> io::Result<JoinHandle<T>>
	where
		F1: FnOnce(M) -> F2,
		F2: Future<Output = T>,
	{
		unreachable!("reached `spawn_with_message_internal()` without atomics target feature")
	}

	/// Implementation of [`std::thread::Builder::spawn_scoped()`].
	#[allow(clippy::unused_self)]
	pub(super) fn spawn_scoped<F, T>(self, _: &Scope, _: F) -> io::Result<ScopedJoinHandle<'_, T>> {
		unreachable!("reached `spawn_scoped()` without atomics target feature")
	}

	/// Implementation for
	/// [`BuilderExt::spawn_scoped_async()`](crate::web::BuilderExt::spawn_scoped_async).
	#[allow(clippy::unused_self)]
	pub(super) fn spawn_scoped_async_internal<F1, F2, T>(
		self,
		_: &Scope,
		_: F1,
	) -> io::Result<ScopedJoinHandle<'_, T>>
	where
		F1: FnOnce() -> F2,
		F2: Future<Output = T>,
	{
		unreachable!("reached `spawn()` without atomics target feature")
	}

	/// Implementation for
	/// [`BuilderExt::spawn_scoped_with_message()`](crate::web::BuilderExt::spawn_scoped_with_message).
	#[cfg(feature = "message")]
	#[allow(clippy::unused_self)]
	pub(super) fn spawn_scoped_with_message_internal<F1, F2, T, M>(
		self,
		_: &Scope,
		_: F1,
		_: M,
	) -> io::Result<ScopedJoinHandle<'_, T>>
	where
		F1: FnOnce(M) -> F2,
		F2: Future<Output = T>,
	{
		unreachable!(
			"reached `spawn_scoped_with_message_internal()` without atomics target feature"
		)
	}

	/// Implementation of [`std::thread::Builder::stack_size()`].
	#[allow(clippy::missing_const_for_fn)]
	pub(super) fn stack_size(self, _: usize) -> Self {
		self
	}
}

/// Implementation of [`std::thread::JoinHandle`].
pub(super) struct JoinHandle<T>(
	#[allow(clippy::absolute_paths)] PhantomData<thread::JoinHandle<T>>,
);

impl<T> Unpin for JoinHandle<T> {}

impl<T> Debug for JoinHandle<T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter.debug_tuple("JoinHandle").finish()
	}
}

impl<T> JoinHandle<T> {
	/// Implementation of [`std::thread::JoinHandle::is_finished()`].
	#[allow(clippy::unused_self)]
	pub(super) fn is_finished(&self) -> bool {
		unreachable!("found instanced `JoinHandle` without threading support")
	}

	/// Implementation of [`std::thread::JoinHandle::join()`].
	#[allow(clippy::unused_self)]
	pub(super) fn join(self) -> thread::Result<T> {
		unreachable!("found instanced `JoinHandle` without threading support")
	}

	/// Implementation of [`std::thread::JoinHandle::thread()`].
	#[allow(clippy::unused_self)]
	pub(super) fn thread(&self) -> &super::Thread {
		unreachable!("found instanced `JoinHandle` without threading support")
	}

	/// Implementation for
	/// [`JoinHandleFuture::poll()`](crate::web::JoinHandleFuture).
	#[allow(clippy::unused_self)]
	pub(super) fn poll(&self, _: &mut Context<'_>) -> Poll<thread::Result<T>> {
		unreachable!("found instanced `JoinHandle` without threading support")
	}
}

/// Implementation of [`std::thread::Scope`].
#[derive(Debug)]
pub(super) struct Scope;

impl Scope {
	/// Create a [`Scope`].
	#[allow(clippy::missing_const_for_fn)]
	pub(super) fn new() -> Self {
		Self
	}

	/// Returns the number of current threads.
	#[allow(clippy::missing_const_for_fn, clippy::unused_self)]
	pub(super) fn thread_count(&self) -> u64 {
		0
	}

	/// End the scope after calling the supplied function.
	#[allow(clippy::missing_const_for_fn, clippy::unused_self)]
	pub(super) fn finish(&self) {}

	/// End the scope after calling the supplied function.
	#[allow(clippy::missing_const_for_fn, clippy::unused_self)]
	pub(super) fn finish_async(&self, _: &Context<'_>) -> Poll<()> {
		Poll::Ready(())
	}
}

/// Implementation of [`std::thread::sleep()`].
pub(super) fn sleep(dur: Duration) {
	let timeout = duration_to_f64_millis(dur);
	let result = ZERO_ARRAY
		.with(|array| Atomics::wait_with_timeout(array, 0, 0, timeout))
		.expect("`Atomics.wait` is not expected to fail");
	debug_assert_eq!(
		result, "timed-out",
		"unexpected return value from `Atomics.wait"
	);
}

/// Tests if blocking is supported.
pub(super) fn test_block_support() -> bool {
	ZERO_ARRAY.with(|array| Atomics::wait_with_timeout(array, 0, 0, 0.).is_ok())
}

/// Implementation for [`crate::web::has_spawn_support()`].
#[allow(clippy::missing_const_for_fn)]
pub(super) fn has_spawn_support() -> bool {
	false
}

thread_local! {
	static ZERO_ARRAY: Int32Array = {
		if super::has_shared_array_buffer_support() {
			Int32Array::new(&SharedArrayBuffer::new(4))
		} else {
			// Without cross-origin isolation `SharedArrayBuffer` is unsupported, but we
			// can still use `Atomics.wait` by using a shared Wasm memory, which is a
			// `SharedArrayBuffer` underneath.
			// See <https://github.com/w3c/ServiceWorker/pull/1545>.
			let descriptor: MemoryDescriptor = Object::new().unchecked_into();
			descriptor.set_initial(1);
			descriptor.set_maximum(1);
			descriptor.set_shared(true);
			let memory = Memory::new(&descriptor).expect("`new Memory` is not expected to fail");
			Int32Array::new(&memory.buffer())
		}
	};
}

/// Converts [`Duration`] to amount of milliseconds as [`f64`].
fn duration_to_f64_millis(duration: Duration) -> f64 {
	duration
		.checked_mul(1000)
		.map_or(f64::INFINITY, |duration| duration.as_secs_f64())
}
