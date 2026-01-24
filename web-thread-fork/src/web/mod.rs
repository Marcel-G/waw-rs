//! Platform-specific extensions for [`web-thread`](crate) on the Web platform.

#[cfg(any(feature = "audio-worklet", docsrs))]
pub mod audio_worklet;
#[cfg(any(feature = "message", docsrs))]
pub mod message;

use std::fmt::{self, Debug, Formatter};
use std::future::{Future, Ready};
use std::io;
use std::panic::RefUnwindSafe;
use std::pin::Pin;
use std::task::{Context, Poll};

#[cfg(any(feature = "message", docsrs))]
use self::message::MessageSend;

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
mod thread {
	pub(super) struct ScopeFuture<'scope, 'env, F, T>(&'scope &'env (F, T));
	pub(super) struct YieldNowFuture;
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use pin_project::pin_project;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use crate::thread;
use crate::{Builder, JoinHandle, Scope, ScopedJoinHandle};

/// Returns [`true`] if the current thread supports blocking.
///
/// # Notes
///
/// Very notably, the thread containing [`Window`] (often called the main thread
/// on Web), does not support blocking.
///
/// Currently known thread types to support blocking:
/// - [Dedicated worker].
/// - [Shared worker] (currently only on Chromium based browsers).
///
/// Currently known thread types to **not** support blocking:
/// - [`Window`] (often called the main thread on Web).
/// - [Service worker].
/// - [Worklet].
///
/// [Dedicated worker]: https://developer.mozilla.org/en-US/docs/Web/API/Worker
/// [Service worker]: https://developer.mozilla.org/en-US/docs/Web/API/Service_Worker_API
/// [Shared worker]: https://developer.mozilla.org/en-US/docs/Web/API/SharedWorker
/// [Worklet]: https://developer.mozilla.org/en-US/docs/Web/API/Worklet
/// [`Window`]: https://developer.mozilla.org/en-US/docs/Web/API/Window
///
/// # Example
///
/// ```
/// # #[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
/// # wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
/// # #[cfg_attr(all(target_feature = "atomics", not(unsupported_spawn)), wasm_bindgen_test::wasm_bindgen_test)]
/// # async fn test() {
/// use web_thread::web::{self, JoinHandleExt};
///
/// let mut handle = web_thread::spawn(|| String::from("test"));
///
/// let result = if web::has_block_support() {
/// 	handle.join().unwrap()
/// } else {
/// 	handle.join_async().await.unwrap()
/// };
/// # let _ = result;
/// # }
/// # #[cfg(not(all(target_feature = "atomics", not(unsupported_spawn))))]
/// # let _ = test();
/// ```
#[must_use]
pub fn has_block_support() -> bool {
	thread::has_block_support()
}

/// Returns [`true`] if the main thread supports spawning threads.
///
/// # Notes
///
/// [`web-thread`](crate) will consider the first thread it finds itself in the
/// "main thread". If Wasm is instantiated in a [dedicated worker], it will
/// consider it as the "main thread".
///
/// Currently only two thread types are known to support spawning threads:
/// - [`Window`] (often called the main thread on Web).
/// - [Dedicated worker].
///
/// Additionally, the following is required to allow spawning threads:
/// - The atomics target feature is enabled.
/// - The site needs to be [cross-origin isolated].
///
/// [cross-origin isolated]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/SharedArrayBuffer#security_requirements
/// [dedicated worker]: https://developer.mozilla.org/en-US/docs/Web/API/Worker
/// [`Window`]: https://developer.mozilla.org/en-US/docs/Web/API/Window
///
/// # Example
///
/// ```
/// # wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
/// # #[wasm_bindgen_test::wasm_bindgen_test]
/// # fn test() {
/// fn schedule_fun(f: impl 'static + FnOnce() + Send) {
/// 	if web_thread::web::has_spawn_support() {
/// 		web_thread::spawn(f);
/// 	} else {
/// 		wasm_bindgen_futures::spawn_local(async { f() });
/// 	}
/// }
///
/// schedule_fun(|| web_sys::console::log_1(&"Are we having fun yet?".into()));
/// # }
/// ```
#[must_use]
pub fn has_spawn_support() -> bool {
	thread::has_spawn_support()
}

/// Web-specific extension for [`web_thread::JoinHandle`](crate::JoinHandle).
pub trait JoinHandleExt<T> {
	/// Async version of [`JoinHandle::join()`].
	///
	/// # Panics
	///
	/// - If called on the thread to join.
	/// - If it was already polled to completion by another call to
	///   [`JoinHandleExt::join_async()`].
	///
	/// # Example
	///
	/// ```
	/// # #[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
	/// # wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
	/// # #[cfg_attr(all(target_feature = "atomics", not(unsupported_spawn)), wasm_bindgen_test::wasm_bindgen_test)]
	/// # async fn test() {
	/// use web_thread::web::JoinHandleExt;
	///
	/// web_thread::spawn(|| ()).join_async().await.unwrap();
	/// # }
	/// # #[cfg(not(all(target_feature = "atomics", not(unsupported_spawn))))]
	/// # let _ = test();
	/// ```
	fn join_async(&mut self) -> JoinHandleFuture<'_, T>;
}

impl<T> JoinHandleExt<T> for JoinHandle<T> {
	fn join_async(&mut self) -> JoinHandleFuture<'_, T> {
		JoinHandleFuture(self)
	}
}

/// Waits for the associated thread to finish. See
/// [`JoinHandleExt::join_async()`].
#[must_use = "does nothing if not polled"]
pub struct JoinHandleFuture<'handle, T>(&'handle mut JoinHandle<T>);

impl<T> Debug for JoinHandleFuture<'_, T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter
			.debug_tuple("JoinHandleFuture")
			.field(&self.0)
			.finish()
	}
}

impl<T> Future for JoinHandleFuture<'_, T> {
	type Output = crate::Result<T>;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		JoinHandle::poll(self.0, cx)
	}
}

/// Async version of [`scope()`](crate::scope).
///
/// # Notes
///
/// Keep in mind that if [`ScopeFuture`] is dropped it will block, or spinloop
/// if blocking is not supported on this thread (see
/// [`has_block_support()`]), until all threads are joined but does not continue
/// polling the passed [`Future`].
///
/// # Example
///
/// ```
/// # #[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
/// # wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
/// # #[cfg_attr(all(target_feature = "atomics", not(unsupported_spawn)), wasm_bindgen_test::wasm_bindgen_test)]
/// # async fn test() {
/// # use std::sync::atomic::{AtomicUsize, Ordering};
/// #
/// let value = AtomicUsize::new(0);
///
/// web_thread::web::scope_async(|scope| async {
/// 	(0..3).for_each(|_| {
/// 		scope.spawn(|| value.fetch_add(1, Ordering::Relaxed));
/// 	});
///
/// 	value.fetch_add(1, Ordering::Relaxed);
/// }).await;
///
/// assert_eq!(value.load(Ordering::Relaxed), 4);
/// # }
/// # #[cfg(not(all(target_feature = "atomics", not(unsupported_spawn))))]
/// # let _ = test();
/// ```
pub fn scope_async<'scope, 'env: 'scope, F1, F2, T>(
	#[allow(clippy::min_ident_chars)] f: F1,
) -> ScopeFuture<'scope, 'env, F2, T>
where
	F1: FnOnce(&'scope Scope<'scope, 'env>) -> F2,
	F2: Future<Output = T>,
{
	ScopeFuture(thread::scope_async(f))
}

/// Waits for the associated scope to finish. See [`scope_async()`].
///
/// # Notes
///
/// Keep in mind that if dropped it will block, or spinloop if blocking is not
/// supported on this thread (see [`has_block_support()`]), until all threads
/// are joined but does not continue polling the passed [`Future`].
#[must_use = "will block until all spawned threads are finished if not polled to completion"]
#[cfg_attr(all(target_family = "wasm", target_os = "unknown"), pin_project)]
pub struct ScopeFuture<'scope, 'env, F, T>(
	#[cfg_attr(all(target_family = "wasm", target_os = "unknown"), pin)]
	thread::ScopeFuture<'scope, 'env, F, T>,
);

impl<F, T> Debug for ScopeFuture<'_, '_, F, T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter.debug_tuple("ScopeFuture").field(&self.0).finish()
	}
}

impl<F, T> Future for ScopeFuture<'_, '_, F, T>
where
	F: Future<Output = T>,
{
	type Output = T;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		self.project().0.poll(cx)
	}
}

impl<'scope, 'env, F, T> ScopeFuture<'scope, 'env, F, T> {
	/// Converts this [`ScopeFuture`] to a [`ScopeJoinFuture`] by waiting until
	/// the given [`Future`] to [`scope_async()`] is finished.
	///
	/// This is useful to:
	/// - Use [`ScopeJoinFuture::join_all()`].
	/// - Be able to drop [`ScopeJoinFuture`] while guaranteeing that the given
	///   [`Future`] to [`scope_async()`] is finished.
	/// - Get rid of `F` which often prevents [`ScopeFuture`] from implementing
	///   [`Unpin`].
	///
	/// # Example
	///
	/// ```
	/// # #[cfg(all(target_feature = "atomics", not(unsupported_spawn), not(unsupported_spawn_then_block)))]
	/// # wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_dedicated_worker);
	/// # #[cfg_attr(all(target_feature = "atomics", not(unsupported_spawn), not(unsupported_spawn_then_block)), wasm_bindgen_test::wasm_bindgen_test)]
	/// # async fn test() {
	/// # use std::sync::atomic::{AtomicUsize, Ordering};
	/// #
	/// let value = AtomicUsize::new(0);
	///
	/// let future = web_thread::web::scope_async(|scope| async {
	/// 	(0..3).for_each(|_| {
	/// 		scope.spawn(|| value.fetch_add(1, Ordering::Relaxed));
	/// 	});
	///
	/// 	value.fetch_add(1, Ordering::Relaxed);
	/// }).into_wait().await;
	///
	/// // This will block until all threads are done.
	/// drop(future);
	///
	/// assert_eq!(value.load(Ordering::Relaxed), 4);
	/// # }
	/// # #[cfg(not(all(target_feature = "atomics", not(unsupported_spawn), not(unsupported_spawn_then_block))))]
	/// # let _ = test();
	/// ```
	pub const fn into_wait(self) -> ScopeIntoJoinFuture<'scope, 'env, F, T> {
		ScopeIntoJoinFuture(self)
	}
}

/// Web-specific extension for
/// [`web_thread::ScopedJoinHandle`](crate::ScopedJoinHandle).
pub trait ScopedJoinHandleExt<'scope, T> {
	/// Async version of [`ScopedJoinHandle::join()`].
	///
	/// # Panics
	///
	/// - If called on the thread to join.
	/// - If it was already polled to completion by another call to
	///   [`ScopedJoinHandleExt::join_async()`].
	///
	/// # Example
	///
	/// ```
	/// # #[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
	/// # wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
	/// # #[cfg_attr(all(target_feature = "atomics", not(unsupported_spawn)), wasm_bindgen_test::wasm_bindgen_test)]
	/// # async fn test() {
	/// use web_thread::web::{self, ScopedJoinHandleExt};
	///
	/// web::scope_async(|scope| async {
	/// 	scope.spawn(|| ()).join_async().await.unwrap();
	/// }).await;
	/// # }
	/// # #[cfg(not(all(target_feature = "atomics", not(unsupported_spawn))))]
	/// # let _ = test();
	/// ```
	fn join_async<'handle>(&'handle mut self) -> ScopedJoinHandleFuture<'handle, 'scope, T>;
}

impl<'scope, T> ScopedJoinHandleExt<'scope, T> for ScopedJoinHandle<'scope, T> {
	fn join_async<'handle>(&'handle mut self) -> ScopedJoinHandleFuture<'handle, 'scope, T> {
		ScopedJoinHandleFuture(self)
	}
}

/// Waits for the associated thread to finish. See
/// [`ScopedJoinHandleExt::join_async()`].
#[must_use = "does nothing if not polled"]
pub struct ScopedJoinHandleFuture<'handle, 'scope, T>(&'handle mut ScopedJoinHandle<'scope, T>);

impl<T> Debug for ScopedJoinHandleFuture<'_, '_, T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter
			.debug_tuple("JoinHandleFuture")
			.field(&self.0)
			.finish()
	}
}

impl<T> Future for ScopedJoinHandleFuture<'_, '_, T> {
	type Output = crate::Result<T>;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		ScopedJoinHandle::poll(self.0, cx)
	}
}

/// Poll to completion to get a [`ScopeJoinFuture`]. See
/// [`ScopeFuture::into_wait()`].
///
/// # Notes
///
/// Keep in mind that if dropped it will block, or spinloop if blocking is not
/// supported on this thread (see [`has_block_support()`]), until all threads
/// are joined but does not continue polling the [`Future`] passed into
/// [`scope_async()`].
#[must_use = "will block until all spawned threads are finished if not polled to completion"]
#[cfg_attr(all(target_family = "wasm", target_os = "unknown"), pin_project)]
pub struct ScopeIntoJoinFuture<'scope, 'env, F, T>(
	#[cfg_attr(all(target_family = "wasm", target_os = "unknown"), pin)]
	ScopeFuture<'scope, 'env, F, T>,
);

impl<F, T> Debug for ScopeIntoJoinFuture<'_, '_, F, T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter
			.debug_tuple("ScopeIntoJoinFuture")
			.field(&self.0)
			.finish()
	}
}

impl<'scope, 'env, F, T> Future for ScopeIntoJoinFuture<'scope, 'env, F, T>
where
	F: Future<Output = T>,
{
	type Output = ScopeJoinFuture<'scope, 'env, T>;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		self.project()
			.0
			.project()
			.0
			.poll_into_wait(cx)
			.map(ScopeFuture)
			.map(ScopeJoinFuture)
	}
}

impl<'scope, 'env, F, T> ScopeIntoJoinFuture<'scope, 'env, F, T> {
	/// Reverts back to [`ScopeFuture`]. See [`ScopeFuture::into_wait()`].
	pub fn revert(self) -> ScopeFuture<'scope, 'env, F, T> {
		self.0
	}
}

/// Waits for the associated scope to finish. See [`ScopeFuture::into_wait()`].
///
/// # Notes
///
/// Keep in mind that if dropped it will block, or spinloop if blocking is not
/// supported on this thread (see [`has_block_support()`]), until all threads
/// are joined.
#[must_use = "will block until all spawned threads are finished if not polled to completion"]
pub struct ScopeJoinFuture<'scope, 'env, T>(ScopeFuture<'scope, 'env, Ready<T>, T>);

impl<T> Debug for ScopeJoinFuture<'_, '_, T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter
			.debug_tuple("ScopeJoinFuture")
			.field(&self.0)
			.finish()
	}
}

impl<T> Future for ScopeJoinFuture<'_, '_, T> {
	type Output = T;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		Pin::new(&mut self.0).poll(cx)
	}
}

impl<T> ScopeJoinFuture<'_, '_, T> {
	/// Returns [`true`] if all threads have finished.
	///
	/// # Notes
	///
	/// When this returns [`true`] it guarantees [`ScopeJoinFuture::join_all()`]
	/// not to block.
	#[must_use]
	pub fn is_finished(&self) -> bool {
		self.0 .0.is_finished()
	}

	/// This will block until all associated threads are finished.
	///
	/// # Panics
	///
	/// - If the calling thread doesn't support blocking, see
	///   [`web::has_block_support()`](has_block_support). Though it is
	///   guaranteed to not block if [`ScopeJoinFuture::is_finished()`] returns
	///   [`true`]. Alternatively consider just polling this [`Future`] to
	///   completion.
	/// - If called after being polled to completion.
	///
	/// # Example
	///
	/// ```
	/// # #[cfg(all(target_feature = "atomics", not(unsupported_spawn), not(unsupported_spawn_then_block)))]
	/// # wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_dedicated_worker);
	/// # #[cfg_attr(all(target_feature = "atomics", not(unsupported_spawn), not(unsupported_spawn_then_block)), wasm_bindgen_test::wasm_bindgen_test)]
	/// # async fn test() {
	/// # use std::sync::atomic::{AtomicUsize, Ordering};
	/// #
	/// let value = AtomicUsize::new(0);
	///
	/// let future = web_thread::web::scope_async(|scope| async {
	/// 	(0..3).for_each(|_| {
	/// 		scope.spawn(|| value.fetch_add(1, Ordering::Relaxed));
	/// 	});
	///
	/// 	value.fetch_add(1, Ordering::Relaxed);
	/// }).into_wait().await;
	///
	/// // This will block until all threads are done.
	/// future.join_all();
	///
	/// assert_eq!(value.load(Ordering::Relaxed), 4);
	/// # }
	/// # #[cfg(not(all(target_feature = "atomics", not(unsupported_spawn), not(unsupported_spawn_then_block))))]
	/// # let _ = test();
	/// ```
	pub fn join_all(self) -> T {
		self.0 .0.join_all()
	}
}

/// Web-specific extension for [`web_thread::Builder`](crate::Builder).
pub trait BuilderExt {
	/// Async version of [`Builder::spawn()`].
	///
	/// For a more complete documentation see [`spawn_async()`].
	///
	/// # Errors
	///
	/// If the main thread does not support spawning threads, see
	/// [`has_spawn_support()`].
	fn spawn_async<F1, F2, T>(self, f: F1) -> io::Result<JoinHandle<T>>
	where
		F1: 'static + FnOnce() -> F2 + Send,
		F2: 'static + Future<Output = T>,
		T: 'static + Send;

	/// [`spawn_async()`] with [message](MessageSend).
	///
	/// For a more complete documentation see [`spawn_with_message()`].
	///
	/// # Errors
	///
	/// - If the main thread does not support spawning threads, see
	///   [`has_spawn_support()`].
	/// - If `message` was unable to be cloned.
	#[cfg(any(feature = "message", docsrs))]
	fn spawn_with_message<F1, F2, T, M>(self, f: F1, message: M) -> io::Result<JoinHandle<T>>
	where
		F1: 'static + FnOnce(M) -> F2 + Send,
		F2: 'static + Future<Output = T>,
		T: 'static + Send,
		M: 'static + MessageSend;

	/// Async version of [`Builder::spawn_scoped()`].
	///
	/// For a more complete documentation see [`Scope::spawn_async()`].
	///
	/// # Errors
	///
	/// If the main thread does not support spawning threads, see
	/// [`has_spawn_support()`].
	fn spawn_scoped_async<'scope, #[allow(single_use_lifetimes)] 'env, F1, F2, T>(
		self,
		scope: &'scope Scope<'scope, 'env>,
		f: F1,
	) -> io::Result<ScopedJoinHandle<'scope, T>>
	where
		F1: 'scope + FnOnce() -> F2 + Send,
		F2: 'scope + Future<Output = T>,
		T: 'scope + Send;

	/// [`BuilderExt::spawn_scoped_async()`] with [message](MessageSend).
	///
	/// For a more complete documentation see [`Scope::spawn_with_message()`].
	///
	/// # Errors
	///
	/// - If the main thread does not support spawning threads, see
	///   [`has_spawn_support()`].
	/// - If `message` was unable to be cloned.
	#[cfg(any(feature = "message", docsrs))]
	fn spawn_scoped_with_message<'scope, #[allow(single_use_lifetimes)] 'env, F1, F2, T, M>(
		self,
		scope: &'scope Scope<'scope, 'env>,
		f: F1,
		message: M,
	) -> io::Result<ScopedJoinHandle<'scope, T>>
	where
		F1: 'scope + FnOnce(M) -> F2 + Send,
		F2: 'scope + Future<Output = T>,
		T: 'scope + Send,
		M: 'scope + MessageSend;
}

impl BuilderExt for Builder {
	fn spawn_async<F1, F2, T>(
		self,
		#[allow(clippy::min_ident_chars)] f: F1,
	) -> io::Result<JoinHandle<T>>
	where
		F1: 'static + FnOnce() -> F2 + Send,
		F2: 'static + Future<Output = T>,
		T: 'static + Send,
	{
		self.spawn_async_internal(f)
	}

	#[cfg(any(feature = "message", docsrs))]
	fn spawn_with_message<F1, F2, T, M>(
		self,
		#[allow(clippy::min_ident_chars)] f: F1,
		message: M,
	) -> io::Result<JoinHandle<T>>
	where
		F1: 'static + FnOnce(M) -> F2 + Send,
		F2: 'static + Future<Output = T>,
		T: 'static + Send,
		M: 'static + MessageSend,
	{
		self.spawn_with_message_internal(f, message)
	}

	fn spawn_scoped_async<'scope, #[allow(single_use_lifetimes)] 'env, F1, F2, T>(
		self,
		scope: &'scope Scope<'scope, 'env>,
		#[allow(clippy::min_ident_chars)] f: F1,
	) -> io::Result<ScopedJoinHandle<'scope, T>>
	where
		F1: 'scope + FnOnce() -> F2 + Send,
		F2: 'scope + Future<Output = T>,
		T: 'scope + Send,
	{
		self.spawn_scoped_async_internal(scope, f)
	}

	#[cfg(any(feature = "message", docsrs))]
	fn spawn_scoped_with_message<'scope, #[allow(single_use_lifetimes)] 'env, F1, F2, T, M>(
		self,
		scope: &'scope Scope<'scope, 'env>,
		#[allow(clippy::min_ident_chars)] f: F1,
		message: M,
	) -> io::Result<ScopedJoinHandle<'scope, T>>
	where
		F1: 'scope + FnOnce(M) -> F2 + Send,
		F2: 'scope + Future<Output = T>,
		T: 'scope + Send,
		M: 'scope + MessageSend,
	{
		self.spawn_scoped_with_message_internal(scope, f, message)
	}
}

/// Web-specific extension for [`web_thread::Scope`](crate::Scope).
pub trait ScopeExt<'scope> {
	/// Async version of [`Scope::spawn()`].
	///
	/// # Notes
	///
	/// Commonly a long-running thread is used by sending messages or tasks to
	/// it and blocking it when there is no work. Unfortunately this is often
	/// undesirable on the Web platform as it prevents yielding to the event
	/// loop.
	///
	/// Therefor being able to `await` the next task instead of blocking the
	/// thread is essential to build long-running threads on the Web platform.
	///
	/// # Panics
	///
	/// If the main thread does not support spawning threads, see
	/// [`has_spawn_support()`].
	///
	/// # Example
	///
	/// ```
	/// # #[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
	/// # wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
	/// # #[cfg_attr(all(target_feature = "atomics", not(unsupported_spawn)), wasm_bindgen_test::wasm_bindgen_test)]
	/// # async fn test() {
	/// use web_thread::web::{self, ScopeExt};
	///
	/// let (sender, receiver) = async_channel::unbounded::<usize>();
	///
	/// # let handle =
	/// web::scope_async(move |scope| async move {
	/// 	scope.spawn_async(move || async move {
	/// 		while let Ok(message) = receiver.recv().await {
	/// 			web_sys::console::log_1(&message.into());
	/// 		}
	/// 	});
	/// });
	///
	/// for message in 0..10 {
	/// 	sender.try_send(message).unwrap();
	/// }
	///
	/// # drop(sender);
	/// # handle.await;
	/// # }
	/// # #[cfg(not(all(target_feature = "atomics", not(unsupported_spawn))))]
	/// # let _ = test();
	/// ```
	fn spawn_async<F1, F2, T>(&'scope self, f: F1) -> ScopedJoinHandle<'scope, T>
	where
		F1: 'scope + FnOnce() -> F2 + Send,
		F2: 'scope + Future<Output = T>,
		T: 'scope + Send;

	/// [`ScopeExt::spawn_async()`] with [message](MessageSend).
	///
	/// For a more complete documentation see [`ScopeExt::spawn_async()`] and
	/// [`spawn_with_message()`].
	///
	/// # Panics
	///
	/// - If the main thread does not support spawning threads, see
	///   [`has_spawn_support()`].
	/// - If `message` was unable to be cloned.
	#[cfg(any(feature = "message", docsrs))]
	fn spawn_with_message<F1, F2, T, M>(
		&'scope self,
		f: F1,
		message: M,
	) -> ScopedJoinHandle<'scope, T>
	where
		F1: 'scope + FnOnce(M) -> F2 + Send,
		F2: 'scope + Future<Output = T>,
		T: 'scope + Send,
		M: 'scope + MessageSend;
}

impl<'scope> ScopeExt<'scope> for Scope<'scope, '_> {
	fn spawn_async<F1, F2, T>(
		&'scope self,
		#[allow(clippy::min_ident_chars)] f: F1,
	) -> ScopedJoinHandle<'scope, T>
	where
		F1: 'scope + FnOnce() -> F2 + Send,
		F2: 'scope + Future<Output = T>,
		T: 'scope + Send,
	{
		self.spawn_async_internal(f)
	}

	#[cfg(any(feature = "message", docsrs))]
	fn spawn_with_message<F1, F2, T, M>(
		&'scope self,
		#[allow(clippy::min_ident_chars)] f: F1,
		message: M,
	) -> ScopedJoinHandle<'scope, T>
	where
		F1: 'scope + FnOnce(M) -> F2 + Send,
		F2: 'scope + Future<Output = T>,
		T: 'scope + Send,
		M: 'scope + MessageSend,
	{
		self.spawn_with_message_internal(f, message)
	}
}

/// Async version of [`spawn()`](std::thread::spawn).
///
/// # Notes
///
/// Commonly a long-running thread is used by sending messages or tasks to
/// it and blocking it when there is no work. Unfortunately this is often
/// undesirable on the Web platform as it prevents yielding to the event
/// loop.
///
/// Therefor being able to `await` the next task instead of blocking the
/// thread is essential to build long-running threads on the Web platform.
///
/// # Panics
///
/// If the main thread does not support spawning threads, see
/// [`has_spawn_support()`].
///
/// # Example
///
/// ```
/// # #[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
/// # wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
/// # #[cfg_attr(all(target_feature = "atomics", not(unsupported_spawn)), wasm_bindgen_test::wasm_bindgen_test)]
/// # async fn test() {
/// # use web_thread::web::JoinHandleExt;
/// #
/// let (sender, receiver) = async_channel::unbounded::<usize>();
///
/// # let mut handle =
/// web_thread::web::spawn_async(move || async move {
/// 	while let Ok(message) = receiver.recv().await {
/// 		web_sys::console::log_1(&message.into());
/// 	}
/// });
///
/// for message in 0..10 {
/// 	sender.try_send(message).unwrap();
/// }
///
/// # drop(sender);
/// # handle.join_async().await.unwrap();
/// # }
/// # #[cfg(not(all(target_feature = "atomics", not(unsupported_spawn))))]
/// # let _ = test();
/// ```
pub fn spawn_async<F1, F2, T>(#[allow(clippy::min_ident_chars)] f: F1) -> JoinHandle<T>
where
	F1: 'static + FnOnce() -> F2 + Send,
	F2: 'static + Future<Output = T>,
	T: 'static + Send,
{
	Builder::new()
		.spawn_async(f)
		.expect("failed to spawn thread")
}

/// [`spawn_async()`] with [message](MessageSend).
///
/// For a more complete documentation see [`spawn_async()`].
///
/// # Panics
///
/// - If the main thread does not support spawning threads, see
///   [`has_spawn_support()`].
/// - If `message` was unable to be cloned.
///
/// # Example
///
/// ```
/// # #[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
/// # wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
/// # #[cfg_attr(all(target_feature = "atomics", not(unsupported_spawn)), wasm_bindgen_test::wasm_bindgen_test)]
/// # async fn test() {
/// # use wasm_bindgen::JsCast;
/// use web_sys::{HtmlCanvasElement, OffscreenCanvas};
/// use web_thread::web::{self, JoinHandleExt};
/// use web_thread::web::message::TransferableWrapper;
///
/// # let canvas = web_sys::window().unwrap().document().unwrap().create_element("canvas").unwrap().unchecked_into();
/// let canvas: HtmlCanvasElement = canvas;
/// let message = TransferableWrapper(canvas.transfer_control_to_offscreen().unwrap());
/// web::spawn_with_message(
/// 	|message| async move {
/// 		let canvas: OffscreenCanvas = message.0;
/// 		// Do work.
/// #       let _ = canvas;
/// 	},
/// 	message,
/// )
/// .join_async()
/// .await
/// .unwrap();
/// # }
/// # #[cfg(not(all(target_feature = "atomics", not(unsupported_spawn))))]
/// # let _ = test();
/// ```
#[cfg(any(feature = "message", docsrs))]
pub fn spawn_with_message<F1, F2, T, M>(
	#[allow(clippy::min_ident_chars)] f: F1,
	message: M,
) -> JoinHandle<T>
where
	F1: 'static + FnOnce(M) -> F2 + Send,
	F2: 'static + Future<Output = T>,
	T: 'static + Send,
	M: 'static + MessageSend,
{
	Builder::new()
		.spawn_with_message(f, message)
		.expect("failed to spawn thread")
}

/// Async version of [`yield_now()`](std::thread::yield_now). This yields
/// execution to the [event loop].
///
/// # Notes
///
/// This is no-op in worklets.
///
/// # Example
///
/// ```
/// # wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
/// # #[wasm_bindgen_test::wasm_bindgen_test]
/// # async fn test() {
/// use web_thread::web::{self, YieldTime};
///
/// # fn long_running_task() -> bool { false }
/// while long_running_task() {
/// 	web::yield_now_async(YieldTime::default()).await
/// }
/// # }
/// ```
///
/// [event loop]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Event_loop
pub fn yield_now_async(time: YieldTime) -> YieldNowFuture {
	YieldNowFuture(thread::YieldNowFuture::new(time))
}

/// How long [`yield_now_async()`] should yield execution to the event loop. See
/// [`yield_now_async()`] for more information.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum YieldTime {
	/// Shortest execution yield to the event loop. Translates to
	/// [`TaskPriority."user-blocking"`].
	///
	/// # Notes
	///
	/// Will fall back to [`MessagePort.postMessage()`] when [`Scheduler`] is
	/// not supported, which is at least as short as
	/// [`UserVisible`](Self::UserVisible).
	///
	/// [`MessagePort.postMessage()`]: https://developer.mozilla.org/en-US/docs/Web/API/MessagePort/postMessage
	/// [`Scheduler`]: https://developer.mozilla.org/en-US/docs/Web/API/Scheduler
	/// [`TaskPriority."user-blocking"`]: https://developer.mozilla.org/en-US/docs/Web/API/Prioritized_Task_Scheduling_API#user-blocking
	UserBlocking,
	/// Default. Translates to [`TaskPriority."user-visible"`].
	///
	/// # Notes
	///
	/// Will fall back to [`MessagePort.postMessage()`] when [`Scheduler`] is
	/// not supported, which is at least as short as
	/// [`UserVisible`](Self::UserVisible).
	///
	/// [`MessagePort.postMessage()`]: https://developer.mozilla.org/en-US/docs/Web/API/MessagePort/postMessage
	/// [`Scheduler`]: https://developer.mozilla.org/en-US/docs/Web/API/Scheduler
	/// [`TaskPriority."user-visible"`]: https://developer.mozilla.org/en-US/docs/Web/API/Prioritized_Task_Scheduling_API#user-visible
	#[default]
	UserVisible,
	/// Translates to [`TaskPriority."background"`].
	///
	/// # Notes
	///
	/// Will fall back to [`MessagePort.postMessage()`] when [`Scheduler`] is
	/// not supported, which is at least as short as
	/// [`UserVisible`](Self::UserVisible).
	///
	/// [`MessagePort.postMessage()`]: https://developer.mozilla.org/en-US/docs/Web/API/MessagePort/postMessage
	/// [`Scheduler`]: https://developer.mozilla.org/en-US/docs/Web/API/Scheduler
	/// [`TaskPriority."background"`]: https://developer.mozilla.org/en-US/docs/Web/API/Prioritized_Task_Scheduling_API#background
	Background,
	/// Longest execution yield to the event loop. Uses
	/// [`Window.requestIdleCallback()`].
	///
	/// # Notes
	///
	/// Will fall back to [`MessagePort.postMessage()`] when
	/// [`Window.requestIdleCallback()`] is not supported, which is at least as
	/// short as [`UserVisible`](Self::UserVisible).
	///
	/// [`MessagePort.postMessage()`]: https://developer.mozilla.org/en-US/docs/Web/API/MessagePort/postMessage
	/// [`Window.requestIdleCallback()`]: https://developer.mozilla.org/en-US/docs/Web/API/Window/requestIdleCallback
	Idle,
}

/// Waits for yielding to the event loop to happen. See [`yield_now_async()`].
#[derive(Debug)]
#[must_use = "does nothing if not polled"]
pub struct YieldNowFuture(thread::YieldNowFuture);

impl Future for YieldNowFuture {
	type Output = ();

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		Pin::new(&mut self.0).poll(cx)
	}
}

impl RefUnwindSafe for YieldNowFuture {}
