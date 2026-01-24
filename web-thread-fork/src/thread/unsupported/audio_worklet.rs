//! Audio worklet extension implementations.

use std::future::Future;
use std::io::{self, Error};
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use web_sys::{AudioWorkletNode, AudioWorkletNodeOptions, BaseAudioContext};

use super::super::Thread;
use crate::web::audio_worklet::{AudioWorkletNodeError, ExtendAudioWorkletProcessor};

/// Implementation for
/// [`crate::web::audio_worklet::BaseAudioContextExt::register_thread()`].
pub(in super::super) fn register_thread<F>(
	_: BaseAudioContext,
	_: Option<usize>,
	_: Option<&str>,
	_: F,
) -> RegisterThreadFuture {
	unreachable!("reached `register_thread()` without atomics target feature")
}

/// Implementation for
/// [`crate::web::audio_worklet::BaseAudioContextExt::register_thread_with_message()`].
#[cfg(feature = "message")]
pub(in super::super) fn register_thread_with_message<F, M>(
	_: BaseAudioContext,
	_: Option<usize>,
	_: Option<&str>,
	_: F,
	_: M,
) -> RegisterThreadFuture {
	unreachable!("reached `register_thread()` without atomics target feature")
}

/// Implementation for [`crate::web::audio_worklet::RegisterThreadFuture`].
#[derive(Debug)]
pub(in super::super) struct RegisterThreadFuture {
	/// Only possible state is an error.
	error: Option<Error>,
	/// Make sure it doesn't implement [`Send`] or [`Sync`].
	_marker: PhantomData<*const ()>,
}

impl Future for RegisterThreadFuture {
	type Output = io::Result<AudioWorkletHandle>;

	fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
		Poll::Ready(Err(self.error.take().expect("polled after completion")))
	}
}

impl RegisterThreadFuture {
	/// Create a [`RegisterThreadFuture`] that returns `error`.
	pub(in super::super) const fn error(error: Error) -> Self {
		Self {
			error: Some(error),
			_marker: PhantomData,
		}
	}
}

/// Implementation for [`crate::web::audio_worklet::AudioWorkletHandle`].
#[derive(Debug)]
pub(in super::super) struct AudioWorkletHandle;

impl AudioWorkletHandle {
	/// Implementation for
	/// [`crate::web::audio_worklet::AudioWorkletHandle::thread()`].
	#[allow(clippy::unused_self)]
	pub(crate) const fn thread(&self) -> &Thread {
		// Reached `register_thread()` without atomics target feature.
		// Text is not inserted in `unreachable!()` because method requires `const`.
		unreachable!()
	}

	/// Implementation for
	/// [`crate::web::audio_worklet::AudioWorkletHandle::release()`].
	///
	/// # Safety
	///
	/// This is only marked `unsafe` for compatibility with the atomics
	/// implementation.
	#[allow(clippy::unused_self)]
	pub(crate) unsafe fn release(self) -> Result<(), Self> {
		unreachable!("reached `register_thread()` without atomics target feature")
	}
}

/// Determined if the current thread is the main thread.
#[allow(clippy::missing_const_for_fn)]
pub(in super::super) fn is_main_thread() -> bool {
	// We can't spawn threads, so this is always `true`.
	true
}

/// Implementation for
/// [`crate::web::audio_worklet::AudioWorkletGlobalScopeExt::register_processor_ext()`].
#[allow(clippy::extra_unused_type_parameters)]
pub(in super::super) fn register_processor<P>(_: &str) -> Result<(), Error> {
	unreachable!("reached `register_processor()` on the main thread")
}

/// Returns [`true`] if this context has a registered thread.
#[allow(clippy::missing_const_for_fn)]
pub(in super::super) fn is_registered(_: &BaseAudioContext) -> bool {
	false
}

/// Implementation for
/// [`crate::web::audio_worklet::BaseAudioContextExt::audio_worklet_node()`].
pub(in super::super) fn audio_worklet_node<P: ExtendAudioWorkletProcessor>(
	_: &BaseAudioContext,
	_: &str,
	_: P::Data,
	_: Option<&AudioWorkletNodeOptions>,
) -> Result<AudioWorkletNode, AudioWorkletNodeError<P>> {
	unreachable!("reached despite not being able to register a thread")
}
