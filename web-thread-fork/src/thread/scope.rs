//! Implementation of [`scope()`] and types related to it.

use std::fmt::{self, Debug, Formatter};
use std::future::{Future, Ready};
use std::marker::PhantomData;
use std::ops::Deref;
use std::pin::Pin;
use std::task::{ready, Context, Poll};
use std::{any, mem, thread};

use pin_project::{pin_project, pinned_drop};

use super::r#impl::JoinHandle;
use super::{r#impl, Builder, Thread};
#[cfg(feature = "message")]
use crate::web::message::MessageSend;

/// See [`std::thread::scope()`].
///
/// # Notes
///
/// Keep in mind that this will enter a spinloop until all threads are joined if
/// blocking is not supported on this thread, see
/// [`web::has_block_support()`](crate::web::has_block_support).
///
/// Alternatively consider using
/// [`web::scope_async()`](crate::web::scope_async).
#[track_caller]
pub fn scope<'env, F, T>(#[allow(clippy::min_ident_chars)] f: F) -> T
where
	F: for<'scope> FnOnce(&'scope Scope<'scope, 'env>) -> T,
{
	let scope = Scope {
		this: r#impl::Scope::new(),
		_scope: PhantomData,
		_env: PhantomData,
	};
	let result = f(&scope);

	scope.this.finish();

	result
}

/// Implementation for [`crate::web::scope_async()`].
pub(crate) fn scope_async<'scope, 'env: 'scope, F1, F2, T>(
	task: F1,
) -> ScopeFuture<'scope, 'env, F2, T>
where
	F1: FnOnce(&'scope Scope<'scope, 'env>) -> F2,
	F2: Future<Output = T>,
{
	let scope = Box::pin(Scope {
		this: r#impl::Scope::new(),
		_scope: PhantomData,
		_env: PhantomData,
	});
	// SAFETY: `scope` and `task` reference each other, but self-referential objects
	// are not support. We have to make sure that `task` is dropped and all threads
	// have finished before `scope` is dropped.
	let task = task(unsafe { mem::transmute::<&Scope<'_, '_>, &Scope<'_, '_>>(scope.deref()) });

	ScopeFuture::new(task, scope)
}

/// See [`std::thread::Scope`].
#[derive(Debug)]
pub struct Scope<'scope, 'env: 'scope> {
	/// Implementation of [`Scope`].
	pub(super) this: r#impl::Scope,
	/// Invariance over the lifetime `'scope`.
	#[allow(clippy::struct_field_names)]
	pub(super) _scope: PhantomData<&'scope mut &'scope ()>,
	/// Invariance over the lifetime `'env`.
	pub(super) _env: PhantomData<&'env mut &'env ()>,
}

impl<'scope, #[allow(single_use_lifetimes)] 'env> Scope<'scope, 'env> {
	/// See [`std::thread::Scope::spawn()`].
	///
	/// # Panics
	///
	/// See [`spawn()`](super::spawn()).
	pub fn spawn<F, T>(
		&'scope self,
		#[allow(clippy::min_ident_chars)] f: F,
	) -> ScopedJoinHandle<'scope, T>
	where
		F: FnOnce() -> T + Send + 'scope,
		T: Send + 'scope,
	{
		Builder::new()
			.spawn_scoped(self, f)
			.expect("failed to spawn thread")
	}

	/// Implementation for
	/// [`ScopeExt::spawn_async()`](crate::web::ScopeExt::spawn_async).
	pub(crate) fn spawn_async_internal<F1, F2, T>(
		&'scope self,
		task: F1,
	) -> ScopedJoinHandle<'scope, T>
	where
		F1: 'scope + FnOnce() -> F2 + Send,
		F2: 'scope + Future<Output = T>,
		T: 'scope + Send,
	{
		Builder::new()
			.spawn_scoped_async_internal(self, task)
			.expect("failed to spawn thread")
	}

	/// Implementation for
	/// [`ScopeExt::spawn_async()`](crate::web::ScopeExt::spawn_async).
	#[cfg(feature = "message")]
	pub(crate) fn spawn_with_message_internal<F1, F2, T, M>(
		&'scope self,
		task: F1,
		message: M,
	) -> ScopedJoinHandle<'scope, T>
	where
		F1: 'scope + FnOnce(M) -> F2 + Send,
		F2: 'scope + Future<Output = T>,
		T: 'scope + Send,
		M: 'scope + MessageSend,
	{
		Builder::new()
			.spawn_scoped_with_message_internal(self, task, message)
			.expect("failed to spawn thread")
	}
}

/// See [`std::thread::ScopedJoinHandle`].
pub struct ScopedJoinHandle<'scope, T> {
	/// The underlying [`JoinHandle`].
	handle: JoinHandle<T>,
	/// Hold the `'scope` lifetime.
	_scope: PhantomData<&'scope ()>,
}

impl<T> Debug for ScopedJoinHandle<'_, T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("ScopedJoinHandle")
			.field("handle", &self.handle)
			.field("_scope", &self._scope)
			.finish()
	}
}

impl<#[allow(single_use_lifetimes)] 'scope, T> ScopedJoinHandle<'scope, T> {
	/// Creates a new [`ScopedJoinHandle`].
	#[cfg(target_feature = "atomics")]
	pub(super) const fn new(handle: JoinHandle<T>) -> Self {
		Self {
			handle,
			_scope: PhantomData,
		}
	}

	/// See [`std::thread::ScopedJoinHandle::thread()`].
	#[must_use]
	pub fn thread(&self) -> &Thread {
		self.handle.thread()
	}

	/// See [`std::thread::ScopedJoinHandle::join()`].
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
	///   is guaranteed to not block if [`ScopedJoinHandle::is_finished()`]
	///   returns [`true`]. Alternatively consider using
	///   [`web::ScopedJoinHandleExt::join_async()`].
	/// - If called on the thread to join.
	/// - If it was already polled to completion through
	///   [`web::ScopedJoinHandleExt::join_async()`].
	///
	/// [`panic = "abort"`]: https://doc.rust-lang.org/1.75.0/cargo/reference/profiles.html#panic
	/// [`web::ScopedJoinHandleExt::join_async()`]: crate::web::ScopedJoinHandleExt::join_async
	#[allow(clippy::missing_errors_doc)]
	pub fn join(self) -> thread::Result<T> {
		self.handle.join()
	}

	/// See [`std::thread::ScopedJoinHandle::is_finished()`].
	///
	/// # Notes
	///
	/// When this returns [`true`] it guarantees [`ScopedJoinHandle::join()`]
	/// not to block.
	#[allow(clippy::must_use_candidate)]
	pub fn is_finished(&self) -> bool {
		self.handle.is_finished()
	}

	/// Implementation for
	/// [`ScopedJoinHandleFuture::poll()`](crate::web::ScopedJoinHandleFuture).
	pub(crate) fn poll(&mut self, cx: &mut Context<'_>) -> Poll<thread::Result<T>> {
		Pin::new(&mut self.handle).poll(cx)
	}
}

/// Waits for the associated scope to finish.
#[pin_project(PinnedDrop)]
pub(crate) struct ScopeFuture<'scope, 'env, F, T>(#[pin] State<'scope, 'env, F, T>);

/// State for [`ScopeFuture`].
#[pin_project(project = ScopeFutureProj, project_replace = ScopeFutureReplace)]
enum State<'scope, 'env, F, T> {
	/// Executing the task given to [`scope_async()`].
	Task {
		/// [`Future`] given by the caller.
		#[pin]
		task: F,
		/// Corresponding [`Scope`].
		scope: Pin<Box<Scope<'scope, 'env>>>,
	},
	/// Wait for all threads to finish.
	Wait {
		/// Result of the [`Future`] given by the caller.
		result: T,
		/// Corresponding [`Scope`].
		scope: Pin<Box<Scope<'scope, 'env>>>,
	},
	/// [`Future`] was polled to conclusion.
	None,
}

impl<F, T> Debug for ScopeFuture<'_, '_, F, T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter.debug_tuple("ScopeFuture").field(&self.0).finish()
	}
}

impl<F, T> Debug for State<'_, '_, F, T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::Task { scope, .. } => formatter
				.debug_struct("Task")
				.field("task", &any::type_name::<F>())
				.field("scope", &scope)
				.finish(),
			Self::Wait { scope, .. } => formatter
				.debug_struct("Wait")
				.field("result", &any::type_name::<T>())
				.field("scope", &scope)
				.finish(),
			Self::None => formatter.write_str("None"),
		}
	}
}

#[pinned_drop]
impl<F, T> PinnedDrop for ScopeFuture<'_, '_, F, T> {
	fn drop(self: Pin<&mut Self>) {
		let this = self.project();

		// SAFETY: We have to make sure that `task` is dropped and all threads have
		// finished before `scope` is dropped.
		if let ScopeFutureReplace::Task { scope, .. } | ScopeFutureReplace::Wait { scope, .. } =
			this.0.project_replace(State::None)
		{
			scope.this.finish();
		}
	}
}

impl<F, T> Future for ScopeFuture<'_, '_, F, T>
where
	F: Future<Output = T>,
{
	type Output = T;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let mut this = self.project();

		loop {
			match this.0.as_mut().project() {
				ScopeFutureProj::Task { task, .. } => {
					let result = ready!(task.poll(cx));
					let ScopeFutureReplace::Task { scope, .. } =
						this.0.as_mut().project_replace(State::None)
					else {
						unreachable!("found wrong state")
					};
					this.0
						.as_mut()
						.project_replace(State::Wait { result, scope });
				}
				ScopeFutureProj::Wait { scope, .. } => {
					ready!(scope.this.finish_async(cx));
					// SAFETY: We have to make sure that `task` is dropped and all threads have
					// finished before `scope` is dropped.
					let ScopeFutureReplace::Wait { result, .. } =
						this.0.project_replace(State::None)
					else {
						unreachable!("found wrong state")
					};
					return Poll::Ready(result);
				}
				ScopeFutureProj::None => panic!("`ScopeFuture` polled after completion"),
			}
		}
	}
}

impl<'scope, 'env, F, T> ScopeFuture<'scope, 'env, F, T>
where
	F: Future<Output = T>,
{
	/// Creates a new [`ScopeFuture`].
	pub(super) const fn new(task: F, scope: Pin<Box<Scope<'scope, 'env>>>) -> Self {
		Self(State::Task { task, scope })
	}

	/// Implementation for
	/// [`ScopeFuture::into_wait()`](crate::web::ScopeFuture::into_wait).
	pub(crate) fn poll_into_wait(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<ScopeFuture<'scope, 'env, Ready<T>, T>> {
		let mut this = self.project();

		match this.0.as_mut().project() {
			ScopeFutureProj::Task { task, .. } => {
				let result = ready!(task.poll(cx));
				let ScopeFutureReplace::Task { scope, .. } = this.0.project_replace(State::None)
				else {
					unreachable!("found wrong state")
				};
				Poll::Ready(ScopeFuture(State::Wait { result, scope }))
			}
			ScopeFutureProj::Wait { .. } => {
				let ScopeFutureReplace::Wait { result, scope } =
					this.0.project_replace(State::None)
				else {
					unreachable!("found wrong state")
				};
				return Poll::Ready(ScopeFuture(State::Wait { result, scope }));
			}
			ScopeFutureProj::None => panic!("`ScopeFuture` polled after completion"),
		}
	}

	/// Implementation for
	/// [`ScopeJoinFuture::is_finished()`](crate::web::ScopeJoinFuture::is_finished).
	pub(crate) fn is_finished(&self) -> bool {
		match &self.0 {
			State::Task { .. } => false,
			State::Wait { scope, .. } => scope.this.thread_count() == 0,
			State::None => true,
		}
	}

	/// Implementation for
	/// [`ScopeJoinFuture::join_all()`](crate::web::ScopeJoinFuture::join_all).
	pub(crate) fn join_all(mut self) -> T {
		match mem::replace(&mut self.0, State::None) {
			State::Wait { result, scope } => {
				assert!(
					super::has_block_support(),
					"current thread type cannot be blocked"
				);

				scope.this.finish();
				result
			}
			State::None => {
				panic!("called after `ScopeJoinFuture` was polled to completion")
			}
			State::Task { .. } => {
				unreachable!("should only be called from `ScopeJoinFuture`")
			}
		}
	}
}
