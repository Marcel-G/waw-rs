//! Audio worklet extension redirection.

use std::future::Future;
use std::io::{self, Error, ErrorKind};
use std::pin::Pin;
use std::task::{Context, Poll};

use web_sys::{AudioWorkletNode, AudioWorkletNodeOptions, BaseAudioContext};

#[cfg(target_feature = "atomics")]
use super::atomics::audio_worklet;
#[cfg(not(target_feature = "atomics"))]
use super::unsupported::audio_worklet;
use super::Thread;
use crate::web::audio_worklet::{AudioWorkletNodeError, ExtendAudioWorkletProcessor};
#[cfg(feature = "message")]
use crate::web::message::MessageSend;

/// Implementation for
/// [`crate::web::audio_worklet::BaseAudioContextExt::register_thread()`].
pub(crate) fn register_thread<F>(
	context: BaseAudioContext,
	stack_size: Option<usize>,
	shim_url: Option<&str>,
	task: F,
) -> RegisterThreadFuture
where
	F: 'static + FnOnce() + Send,
{
	RegisterThreadFuture(if super::has_spawn_support() {
		audio_worklet::register_thread(context, stack_size, shim_url, task)
	} else {
		audio_worklet::RegisterThreadFuture::error(Error::new(
			ErrorKind::Unsupported,
			"operation not supported on this platform without the atomics target feature and \
			 cross-origin isolation",
		))
	})
}

/// Implementation for
/// [`crate::web::audio_worklet::BaseAudioContextExt::register_thread_with_message()`].
#[cfg(feature = "message")]
pub(crate) fn register_thread_with_message<F, M>(
	context: BaseAudioContext,
	stack_size: Option<usize>,
	shim_url: Option<&str>,
	task: F,
	message: M,
) -> RegisterThreadFuture
where
	F: 'static + FnOnce(M) + Send,
	M: 'static + MessageSend,
{
	RegisterThreadFuture(if super::has_spawn_support() {
		audio_worklet::register_thread_with_message(context, stack_size, shim_url, task, message)
	} else {
		audio_worklet::RegisterThreadFuture::error(Error::new(
			ErrorKind::Unsupported,
			"operation not supported on this platform without the atomics target feature and \
			 cross-origin isolation",
		))
	})
}

/// Implementation for [`crate::web::audio_worklet::RegisterThreadFuture`].
#[derive(Debug)]
pub(crate) struct RegisterThreadFuture(audio_worklet::RegisterThreadFuture);

impl Future for RegisterThreadFuture {
	type Output = io::Result<AudioWorkletHandle>;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		Pin::new(&mut self.0).poll(cx).map_ok(AudioWorkletHandle)
	}
}

/// Implementation for [`crate::web::audio_worklet::AudioWorkletHandle`].
#[derive(Debug)]
pub(crate) struct AudioWorkletHandle(audio_worklet::AudioWorkletHandle);

impl AudioWorkletHandle {
	/// Implementation for
	/// [`crate::web::audio_worklet::AudioWorkletHandle::thread()`].
	pub(crate) const fn thread(&self) -> &Thread {
		self.0.thread()
	}

	/// Implementation for
	/// [`crate::web::audio_worklet::AudioWorkletHandle::release()`].
	///
	/// # Safety
	///
	/// See [`AudioWorkletHandle::release()`](audio_worklet::AudioWorkletHandle::release).
	pub(crate) unsafe fn release(self) -> Result<(), Self> {
		// SAFETY: See `ThreadMemory::release()`. Other safety guarantees have to be
		// uphold by the caller.
		unsafe { self.0.release() }.map_err(Self)
	}
}

/// Implementation for
/// [`crate::web::audio_worklet::AudioWorkletGlobalScopeExt::register_processor_ext()`].
pub(crate) fn register_processor<P: 'static + ExtendAudioWorkletProcessor>(
	name: &str,
) -> Result<(), Error> {
	if audio_worklet::is_main_thread() {
		Err(Error::new(
			ErrorKind::Unsupported,
			"thread was not spawned by `web-thread`",
		))
	} else {
		audio_worklet::register_processor::<P>(name)
	}
}

/// Implementation for
/// [`crate::web::audio_worklet::BaseAudioContextExt::audio_worklet_node()`].
pub(crate) fn audio_worklet_node<P: 'static + ExtendAudioWorkletProcessor>(
	context: &BaseAudioContext,
	name: &str,
	data: P::Data,
	options: Option<&AudioWorkletNodeOptions>,
) -> Result<AudioWorkletNode, AudioWorkletNodeError<P>> {
	if audio_worklet::is_registered(context) {
		audio_worklet::audio_worklet_node(context, name, data, options)
	} else {
		Err(AudioWorkletNodeError {
			data,
			error: Error::new(
				ErrorKind::Other,
				"`register_thread()` has to be called on this context first",
			),
		})
	}
}
