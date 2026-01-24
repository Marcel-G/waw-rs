//! Implementation of [`spawn()`] and types related to it.

use std::fmt::{Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{fmt, thread};

use super::{r#impl, Builder, Thread};

/// See [`std::thread::spawn()`].
///
/// # Panics
///
/// If the main thread does not support spawning threads, see
/// [`web::has_spawn_support()`](crate::web::has_spawn_support).
#[allow(clippy::min_ident_chars, clippy::type_repetition_in_bounds)]
pub fn spawn<F, T>(f: F) -> JoinHandle<T>
where
	F: FnOnce() -> T,
	F: Send + 'static,
	T: Send + 'static,
{
	Builder::new().spawn(f).expect("failed to spawn thread")
}

/// See [`std::thread::JoinHandle`].
pub struct JoinHandle<T>(r#impl::JoinHandle<T>);

impl<T> Debug for JoinHandle<T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter.debug_tuple("JoinHandle").field(&self.0).finish()
	}
}

impl<T> JoinHandle<T> {
	/// Creates a new [`JoinHandle`].
	pub(super) const fn new(handle: r#impl::JoinHandle<T>) -> Self {
		Self(handle)
	}

	/// See [`std::thread::JoinHandle::is_finished()`].
	///
	/// # Notes
	///
	/// When this returns [`true`] it guarantees [`JoinHandle::join()`] not to
	/// block.
	#[allow(clippy::must_use_candidate)]
	pub fn is_finished(&self) -> bool {
		self.0.is_finished()
	}

	/// See [`std::thread::JoinHandle::join()`].
	///
	/// # Notes
	///
	/// When compiling with [`panic = "abort"`], which is the only option
	/// without enabling the Wasm exception-handling proposal, this can never
	/// return [`Err`].
	///
	/// # Panics
	///
	/// - If the calling thread doesn't support blocking, see
	///   [`web::has_block_support()`](crate::web::has_block_support). Though it
	///   is guaranteed to not block if [`JoinHandle::is_finished()`] returns
	///   [`true`]. Alternatively consider using
	///   [`web::JoinHandleExt::join_async()`].
	/// - If called on the thread to join.
	/// - If it was already polled to completion through
	///   [`web::JoinHandleExt::join_async()`].
	///
	/// [`panic = "abort"`]: https://doc.rust-lang.org/1.75.0/cargo/reference/profiles.html#panic
	/// [`web::JoinHandleExt::join_async()`]: crate::web::JoinHandleExt::join_async
	#[allow(clippy::missing_errors_doc)]
	pub fn join(self) -> thread::Result<T> {
		self.0.join()
	}

	/// See [`std::thread::JoinHandle::thread()`].
	#[must_use]
	pub fn thread(&self) -> &Thread {
		self.0.thread()
	}

	/// Implementation for
	/// [`JoinHandleFuture::poll()`](crate::web::JoinHandleFuture).
	pub(crate) fn poll(&mut self, cx: &mut Context<'_>) -> Poll<thread::Result<T>> {
		Pin::new(&mut self.0).poll(cx)
	}
}
