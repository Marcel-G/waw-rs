//! Implementation of [`Builder`].

use std::future::Future;
use std::io::{self, Error, ErrorKind};

use super::{r#impl, JoinHandle, Scope, ScopedJoinHandle};
#[cfg(feature = "message")]
use crate::web::message::MessageSend;

/// See [`std::thread::Builder`].
#[derive(Debug)]
#[must_use = "must eventually spawn the thread"]
pub struct Builder(r#impl::Builder);

impl Builder {
	/// See [`std::thread::Builder::new()`].
	#[allow(clippy::new_without_default)]
	pub fn new() -> Self {
		Self(r#impl::Builder::new())
	}

	/// See [`std::thread::Builder::name()`].
	pub fn name(self, name: String) -> Self {
		Self(self.0.name(name))
	}

	/// See [`std::thread::Builder::spawn()`].
	///
	/// # Errors
	///
	/// If the main thread does not support spawning threads, see
	/// [`web::has_spawn_support()`](crate::web::has_spawn_support).
	#[allow(clippy::type_repetition_in_bounds)]
	pub fn spawn<F, T>(self, #[allow(clippy::min_ident_chars)] f: F) -> io::Result<JoinHandle<T>>
	where
		F: FnOnce() -> T,
		F: Send + 'static,
		T: Send + 'static,
	{
		if super::has_spawn_support() {
			self.0.spawn(f).map(JoinHandle::new)
		} else {
			Err(Error::new(
				ErrorKind::Unsupported,
				"operation not supported on this platform without the atomics target feature and \
				 cross-origin isolation",
			))
		}
	}

	/// Implementation for
	/// [`BuilderExt::spawn_async()`](crate::web::BuilderExt::spawn_async).
	pub(crate) fn spawn_async_internal<F1, F2, T>(self, task: F1) -> io::Result<JoinHandle<T>>
	where
		F1: 'static + FnOnce() -> F2 + Send,
		F2: 'static + Future<Output = T>,
		T: 'static + Send,
	{
		if super::has_spawn_support() {
			self.0.spawn_async_internal(task).map(JoinHandle::new)
		} else {
			Err(Error::new(
				ErrorKind::Unsupported,
				"operation not supported on this platform without the atomics target feature and \
				 cross-origin isolation",
			))
		}
	}

	/// Implementation for
	/// [`BuilderExt::spawn_with_message()`](crate::web::BuilderExt::spawn_with_message).
	#[cfg(feature = "message")]
	pub(crate) fn spawn_with_message_internal<F1, F2, T, M>(
		self,
		task: F1,
		message: M,
	) -> io::Result<JoinHandle<T>>
	where
		F1: 'static + FnOnce(M) -> F2 + Send,
		F2: 'static + Future<Output = T>,
		T: 'static + Send,
		M: 'static + MessageSend,
	{
		if super::has_spawn_support() {
			self.0
				.spawn_with_message_internal(task, message)
				.map(JoinHandle::new)
		} else {
			Err(Error::new(
				ErrorKind::Unsupported,
				"operation not supported on this platform without the atomics target feature and \
				 cross-origin isolation",
			))
		}
	}

	/// See [`std::thread::Builder::spawn_scoped()`].
	///
	/// # Errors
	///
	/// If the main thread does not support spawning threads, see
	/// [`web::has_spawn_support()`](crate::web::has_spawn_support).
	pub fn spawn_scoped<'scope, #[allow(single_use_lifetimes)] 'env, F, T>(
		self,
		scope: &'scope Scope<'scope, 'env>,
		#[allow(clippy::min_ident_chars)] f: F,
	) -> io::Result<ScopedJoinHandle<'scope, T>>
	where
		F: FnOnce() -> T + Send + 'scope,
		T: Send + 'scope,
	{
		if super::has_spawn_support() {
			self.0.spawn_scoped(&scope.this, f)
		} else {
			Err(Error::new(
				ErrorKind::Unsupported,
				"operation not supported on this platform without the atomics target feature and \
				 cross-origin isolation",
			))
		}
	}

	/// Implementation for
	/// [`BuilderExt::spawn_scoped_async()`](crate::web::BuilderExt::spawn_scoped_async).
	pub(crate) fn spawn_scoped_async_internal<'scope, F1, F2, T>(
		self,
		scope: &'scope Scope<'scope, '_>,
		task: F1,
	) -> io::Result<ScopedJoinHandle<'scope, T>>
	where
		F1: 'scope + FnOnce() -> F2 + Send,
		F2: 'scope + Future<Output = T>,
		T: 'scope + Send,
	{
		if super::has_spawn_support() {
			self.0.spawn_scoped_async_internal(&scope.this, task)
		} else {
			Err(Error::new(
				ErrorKind::Unsupported,
				"operation not supported on this platform without the atomics target feature and \
				 cross-origin isolation",
			))
		}
	}

	/// Implementation for
	/// [`BuilderExt::spawn_scoped_with_message()`](crate::web::BuilderExt::spawn_scoped_with_message).
	#[cfg(feature = "message")]
	pub(crate) fn spawn_scoped_with_message_internal<'scope, F1, F2, T, M>(
		self,
		scope: &'scope Scope<'scope, '_>,
		task: F1,
		message: M,
	) -> io::Result<ScopedJoinHandle<'scope, T>>
	where
		F1: 'scope + FnOnce(M) -> F2 + Send,
		F2: 'scope + Future<Output = T>,
		T: 'scope + Send,
		M: 'scope + MessageSend,
	{
		if super::has_spawn_support() {
			self.0
				.spawn_scoped_with_message_internal(&scope.this, task, message)
		} else {
			Err(Error::new(
				ErrorKind::Unsupported,
				"operation not supported on this platform without the atomics target feature and \
				 cross-origin isolation",
			))
		}
	}

	/// See [`std::thread::Builder::stack_size()`].
	///
	/// # Notes
	///
	/// Stack size will be round up to the nearest multiple of the WebAssembly
	/// page size, which is 64 Ki.
	pub fn stack_size(self, size: usize) -> Self {
		Self(self.0.stack_size(size))
	}
}
