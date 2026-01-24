//! Implementation when the atomics target feature is enabled.

// This part of the code requires the nightly toolchain.
#![allow(clippy::incompatible_msrv)]

#[cfg(feature = "audio-worklet")]
pub(super) mod audio_worklet;
mod channel;
mod js;
mod main;
mod memory;
mod oneshot;
mod parker;
mod spawn;
mod url;
mod wait_async;

use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::panic::RefUnwindSafe;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock};
use std::task::{Context, Poll};
use std::time::Duration;
use std::{io, ptr, thread};

use atomic_waker::AtomicWaker;
use js_sys::WebAssembly::{Memory, Module};
use js_sys::{Atomics, Int32Array};
use wasm_bindgen::JsCast;
#[cfg(any(feature = "audio-worklet", feature = "message"))]
use {std::io::Error, wasm_bindgen::JsValue, web_sys::DomException};

use self::oneshot::Receiver;
pub(super) use self::parker::Parker;
use super::js::GlobalExt;
use super::{ScopedJoinHandle, Thread, ThreadId, THREAD};
#[cfg(feature = "message")]
use crate::web::message::MessageSend;

thread_local! {
	/// [`Memory`] of the Wasm module.
	pub(super) static MEMORY: Memory = wasm_bindgen::memory().unchecked_into();
	/// [`Memory`] of the Wasm module as a [`Int32Array`].
	pub(super) static MEMORY_ARRAY: Int32Array = Int32Array::new(&MEMORY.with(Memory::buffer));
	/// Wasm [`Module`].
	pub(super) static MODULE: Module = wasm_bindgen::module().unchecked_into();
}

/// Implementation of [`std::thread::Builder`].
#[derive(Debug)]
pub(super) struct Builder {
	/// Name of the thread.
	name: Option<String>,
	/// Stack size of the thread.
	stack_size: Option<usize>,
}

impl Builder {
	/// Implementation of [`std::thread::Builder::new()`].
	#[allow(clippy::missing_const_for_fn, clippy::new_without_default)]
	pub(super) fn new() -> Self {
		Self {
			name: None,
			stack_size: None,
		}
	}

	/// Implementation of [`std::thread::Builder::name()`].
	pub(super) fn name(mut self, name: String) -> Self {
		self.name = Some(name);
		self
	}

	/// Implementation of [`std::thread::Builder::spawn()`].
	pub(super) fn spawn<F, T>(self, task: F) -> io::Result<JoinHandle<T>>
	where
		F: 'static + FnOnce() -> T + Send,
		T: 'static + Send,
	{
		// SAFETY: `F` and `T` are `'static`.
		unsafe { spawn::spawn(|| async { task() }, self.name, self.stack_size, None) }
	}

	/// Implementation for
	/// [`BuilderExt::spawn_async()`](crate::web::BuilderExt::spawn_async).
	pub(super) fn spawn_async_internal<F1, F2, T>(self, task: F1) -> io::Result<JoinHandle<T>>
	where
		F1: 'static + FnOnce() -> F2 + Send,
		F2: 'static + Future<Output = T>,
		T: 'static + Send,
	{
		// SAFETY: `F` and `T` are `'static`.
		unsafe { spawn::spawn(task, self.name, self.stack_size, None) }
	}

	/// Implementation for
	/// [`BuilderExt::spawn_with_message()`](crate::web::BuilderExt::spawn_with_message).
	#[cfg(feature = "message")]
	pub(super) fn spawn_with_message_internal<F1, F2, T, M: 'static + MessageSend>(
		self,
		task: F1,
		message: M,
	) -> io::Result<JoinHandle<T>>
	where
		F1: 'static + FnOnce(M) -> F2 + Send,
		F2: 'static + Future<Output = T>,
		T: 'static + Send,
	{
		// SAFETY: `F` and `T` are `'static`.
		unsafe { spawn::message::spawn(task, self.name, self.stack_size, None, message) }
	}

	/// Implementation of [`std::thread::Builder::spawn_scoped()`].
	pub(super) fn spawn_scoped<'scope, F, T>(
		self,
		scope: &'scope Scope,
		task: F,
	) -> io::Result<ScopedJoinHandle<'scope, T>>
	where
		F: 'scope + FnOnce() -> T + Send,
		T: 'scope + Send,
	{
		// SAFETY: `Scope` will prevent this thread to outlive its lifetime.
		let result = unsafe {
			spawn::spawn(
				|| async { task() },
				self.name,
				self.stack_size,
				Some(Arc::clone(&scope.0)),
			)
		};

		result.map(|handle| ScopedJoinHandle::new(handle))
	}

	/// Implementation for
	/// [`BuilderExt::spawn_scoped_async()`](crate::web::BuilderExt::spawn_scoped_async).
	pub(super) fn spawn_scoped_async_internal<'scope, F1, F2, T>(
		self,
		scope: &Scope,
		task: F1,
	) -> io::Result<ScopedJoinHandle<'scope, T>>
	where
		F1: 'scope + FnOnce() -> F2 + Send,
		F2: 'scope + Future<Output = T>,
		T: 'scope + Send,
	{
		// SAFETY: `Scope` will prevent this thread to outlive its lifetime.
		let result =
			unsafe { spawn::spawn(task, self.name, self.stack_size, Some(Arc::clone(&scope.0))) };

		result.map(|handle| ScopedJoinHandle::new(handle))
	}

	/// Implementation for
	/// [`BuilderExt::spawn_scoped_with_message()`](crate::web::BuilderExt::spawn_scoped_with_message).
	#[cfg(feature = "message")]
	pub(super) fn spawn_scoped_with_message_internal<'scope, F1, F2, T, M>(
		self,
		scope: &Scope,
		task: F1,
		message: M,
	) -> io::Result<ScopedJoinHandle<'scope, T>>
	where
		F1: 'scope + FnOnce(M) -> F2 + Send,
		F2: 'scope + Future<Output = T>,
		T: 'scope + Send,
		M: 'scope + MessageSend,
	{
		// SAFETY: `Scope` will prevent this thread to outlive its lifetime.
		let result = unsafe {
			spawn::message::spawn(
				task,
				self.name,
				self.stack_size,
				Some(Arc::clone(&scope.0)),
				message,
			)
		};

		result.map(|handle| ScopedJoinHandle::new(handle))
	}

	/// Implementation of [`std::thread::Builder::stack_size()`].
	pub(super) fn stack_size(mut self, mut size: usize) -> Self {
		/// Wasm page size according to the specification is 64 Ki.
		/// See <https://webassembly.github.io/spec/core/exec/runtime.html#page-size>.
		const PAGE_SIZE: usize = 1024 * 64;

		size = size.checked_add(PAGE_SIZE - 1).unwrap_or(usize::MAX) / PAGE_SIZE * PAGE_SIZE;

		self.stack_size = Some(size);
		self
	}
}

/// Implementation of [`std::thread::JoinHandle`].
pub(super) struct JoinHandle<T> {
	/// Receiver for the return value.
	receiver: Option<Receiver<T>>,
	/// Corresponding [`Thread`].
	thread: Thread,
}

impl<T> Debug for JoinHandle<T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("JoinHandle")
			.field("receiver", &self.receiver)
			.field("thread", &self.thread)
			.finish()
	}
}

impl<T> JoinHandle<T> {
	/// Implementation of [`std::thread::JoinHandle::is_finished()`].
	pub(super) fn is_finished(&self) -> bool {
		self.receiver.as_ref().map_or(true, Receiver::is_ready)
	}

	/// Implementation of [`std::thread::JoinHandle::join()`].
	#[allow(clippy::unnecessary_wraps)]
	pub(super) fn join(self) -> thread::Result<T> {
		assert_ne!(
			self.thread().id(),
			super::current().id(),
			"called `JoinHandle::join()` on the thread to join"
		);

		Ok(self
			.receiver
			.expect("`JoinHandle::join()` called after `JoinHandleFuture` polled to completion")
			.receive()
			.expect("thread terminated without returning"))
	}

	/// Implementation of [`std::thread::JoinHandle::thread()`].
	#[allow(clippy::missing_const_for_fn)]
	pub(super) fn thread(&self) -> &Thread {
		&self.thread
	}

	/// Implementation for
	/// [`JoinHandleFuture::poll()`](crate::web::JoinHandleFuture).
	pub(super) fn poll(&mut self, cx: &mut Context<'_>) -> Poll<thread::Result<T>> {
		assert_ne!(
			self.thread().id(),
			super::current().id(),
			"called `JoinHandle::join()` on the thread to join"
		);

		let mut receiver = self
			.receiver
			.take()
			.expect("`JoinHandleFuture` polled or created after completion");

		match Pin::new(&mut receiver).poll(cx) {
			Poll::Ready(Some(value)) => Poll::Ready(Ok(value)),
			Poll::Pending => {
				self.receiver = Some(receiver);
				Poll::Pending
			}
			Poll::Ready(None) => unreachable!("thread terminated without returning"),
		}
	}
}

impl Thread {
	/// Registers the given `thread`.
	fn register(thread: Self) {
		THREAD.with(|cell| cell.set(thread).expect("`Thread` already registered"));
	}
}

/// Implementation of [`std::thread::Scope`].
#[derive(Debug)]
pub(super) struct Scope(Arc<ScopeData>);

impl RefUnwindSafe for Scope {}

/// Shared data between [`Scope`] and scoped threads.
#[derive(Debug)]
pub(super) struct ScopeData {
	/// Number of running threads.
	threads: AtomicU64,
	/// Handle to the spawning thread.
	thread: Thread,
	/// [`Waker`](std::task::Waker) to wake up a waiting [`Scope`].
	waker: AtomicWaker,
}

impl Scope {
	/// Creates a new [`Scope`].
	pub(super) fn new() -> Self {
		Self(Arc::new(ScopeData {
			threads: AtomicU64::new(0),
			thread: super::current(),
			waker: AtomicWaker::new(),
		}))
	}

	/// Returns the number of current threads.
	pub(super) fn thread_count(&self) -> u64 {
		self.0.threads.load(Ordering::Relaxed)
	}

	/// End the scope after calling the supplied function.
	pub(super) fn finish(&self) {
		while self.0.threads.load(Ordering::Acquire) != 0 {
			super::park();
		}
	}

	/// End the scope after calling the supplied function.
	pub(super) fn finish_async(&self, cx: &Context<'_>) -> Poll<()> {
		if self.0.threads.load(Ordering::Acquire) == 0 {
			return Poll::Ready(());
		}

		self.0.waker.register(cx.waker());

		if self.0.threads.load(Ordering::Acquire) == 0 {
			Poll::Ready(())
		} else {
			Poll::Pending
		}
	}
}

/// Implementation of [`std::thread::sleep()`].
pub(super) fn sleep(dur: Duration) {
	thread::sleep(dur);
}

/// Tests if blocking is supported.
pub(super) fn test_block_support() -> bool {
	let value = Pin::new(&0);
	let index = i32_to_buffer_index(ptr::from_ref(&value));

	MEMORY_ARRAY
		.with(|array| Atomics::wait_with_timeout(array, index, 0, 0.))
		.is_ok()
}

/// Implementation for [`crate::web::has_spawn_support()`]. Make sure to
/// call at least once on the main thread!
pub(super) fn has_spawn_support() -> bool {
	/// We spawn only from the main thread, so we cache the result to be able to
	/// call it from other threads but get the result of the main thread.
	#[allow(
		clippy::disallowed_methods,
		reason = "this will be called at least once from the main thread before being cached"
	)]
	static HAS_SPAWN_SUPPORT: LazyLock<bool> = LazyLock::new(|| {
		super::has_shared_array_buffer_support() && {
			let global: GlobalExt = js_sys::global().unchecked_into();
			!global.worker().is_undefined()
		}
	});

	*HAS_SPAWN_SUPPORT
}

/// Returns the [`ThreadId`] of the current thread without cloning the
/// [`Arc`].
fn current_id() -> ThreadId {
	THREAD.with(|cell| cell.get_or_init(Thread::new).id())
}

/// Determined if the current thread is the main thread. Make sure to
/// call at least once on the main thread!
pub(super) fn is_main_thread() -> bool {
	/// Saves the [`ThreadId`] of the main thread.
	#[allow(
		clippy::disallowed_methods,
		reason = "this will be called at least once from the main thread before being cached"
	)]
	static MAIN_THREAD: LazyLock<ThreadId> = LazyLock::new(current_id);

	*MAIN_THREAD == current_id()
}

/// Converts a reference to a pointer to [`i32`] to an index into the internal
/// [`ArrayBuffer`](js_sys::ArrayBuffer) usable by methods of [`Atomics`].
fn i32_to_buffer_index(ptr: *const i32) -> u32 {
	#[allow(clippy::as_conversions)]
	let index = ptr as u32 / 4;
	index
}

/// Convert a [`JsValue`] to an [`DomException`] and then to an [`Error`].
#[cfg(any(feature = "audio-worklet", feature = "message"))]
fn error_from_exception(error: JsValue) -> Error {
	let error: DomException = error.unchecked_into();

	Error::other(format!("{}: {}", error.name(), error.message()))
}
