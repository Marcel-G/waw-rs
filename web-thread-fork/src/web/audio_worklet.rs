//! Platform-specific extensions for [`web-thread`](crate) on the Web platform
//! to spawn and use audio worklets. See
//! [`BaseAudioContextExt::audio_worklet_node()`] for a usage example.

use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::future::Future;
use std::panic::RefUnwindSafe;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{any, fmt, io};

use js_sys::{Array, Iterator, Object};
#[cfg(all(
	target_family = "wasm",
	target_os = "unknown",
	feature = "audio-worklet"
))]
use web_sys::{AudioWorkletGlobalScope, BaseAudioContext};
use web_sys::{AudioWorkletNode, AudioWorkletNodeOptions, AudioWorkletProcessor};

#[cfg(any(feature = "message", docsrs))]
use super::message::MessageSend;
#[cfg(all(
	target_family = "wasm",
	target_os = "unknown",
	feature = "audio-worklet"
))]
use crate::thread::audio_worklet;
use crate::Thread;

#[cfg(not(all(
	target_family = "wasm",
	target_os = "unknown",
	feature = "audio-worklet"
)))]
mod audio_worklet {
	pub(super) struct AudioWorkletHandle;
	pub(super) struct RegisterThreadFuture;
}
#[cfg(not(all(
	target_family = "wasm",
	target_os = "unknown",
	feature = "audio-worklet"
)))]
mod web_sys {
	pub(super) struct AudioWorkletNodeOptions;
	pub(super) struct AudioWorkletProcessor;
	pub(super) struct AudioWorkletNode;
}
#[cfg(not(all(
	target_family = "wasm",
	target_os = "unknown",
	feature = "audio-worklet"
)))]
mod js_sys {
	pub(super) struct Array;
	pub(super) struct Iterator;
	pub(super) struct Object;
}

/// Extension for [`BaseAudioContext`].
#[cfg_attr(
	not(all(
		target_family = "wasm",
		target_os = "unknown",
		feature = "audio-worklet"
	)),
	doc = "",
	doc = "[`BaseAudioContext`]: https://docs.rs/web-sys/0.3.68/web_sys/struct.BaseAudioContext.html"
)]
pub trait BaseAudioContextExt {
	/// Registers a thread at this [`BaseAudioContext`].
	///
	/// # Notes
	///
	/// Unfortunately there is currently no way to determine when the thread has
	/// fully shutdown. So this will leak memory unless
	/// [`AudioWorkletHandle::release()`] is called.
	///
	/// # Errors
	///
	/// - If a thread was already registered at this [`BaseAudioContext`].
	/// - If the [`BaseAudioContext`] is [`closed`].
	/// - If the main thread does not support spawning threads, see
	///   [`has_spawn_support()`](super::has_spawn_support).
	///
	/// # Example
	///
	/// ```
	/// # #[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
	/// # wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
	/// # #[cfg_attr(all(target_feature = "atomics", not(unsupported_spawn)), wasm_bindgen_test::wasm_bindgen_test)]
	/// # async fn test() {
	/// use web_sys::AudioContext;
	/// use web_thread::web::audio_worklet::BaseAudioContextExt;
	///
	/// let context = AudioContext::new().unwrap();
	/// context.clone().register_thread(
	/// 	None,
	/// 	|| {
	/// 		// Do work.
	/// 	},
	/// ).await.unwrap();
	/// # }
	/// # #[cfg(not(all(target_feature = "atomics", not(unsupported_spawn))))]
	/// # let _ = test();
	/// ```
	///
	/// [`closed`]: https://developer.mozilla.org/en-US/docs/Web/API/BaseAudioContext/state#closed
	#[cfg_attr(
		not(all(
			target_family = "wasm",
			target_os = "unknown",
			feature = "audio-worklet"
		)),
		doc = "[`BaseAudioContext`]: https://docs.rs/web-sys/0.3.68/web_sys/struct.BaseAudioContext.html"
	)]
	fn register_thread<F>(
		self,
		stack_size: Option<usize>,
		shim_url: Option<&str>,
		f: F,
	) -> RegisterThreadFuture
	where
		F: 'static + FnOnce() + Send;

	/// Registers a thread at this [`BaseAudioContext`].
	///
	/// # Notes
	///
	/// Unfortunately there is currently no way to determine when the thread has
	/// fully shutdown. So this will leak memory unless
	/// [`AudioWorkletHandle::release()`] is called.
	///
	/// # Errors
	///
	/// - If a thread was already registered at this [`BaseAudioContext`].
	/// - If the [`BaseAudioContext`] is [`closed`].
	/// - If the main thread does not support spawning threads, see
	///   [`has_spawn_support()`](super::has_spawn_support).
	///
	/// # Example
	///
	/// ```
	/// # #[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
	/// # wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
	/// # #[cfg_attr(all(target_feature = "atomics", not(unsupported_spawn)), wasm_bindgen_test::wasm_bindgen_test)]
	/// # async fn test() {
	/// use js_sys::ArrayBuffer;
	/// use web_sys::AudioContext;
	/// use web_thread::web::audio_worklet::BaseAudioContextExt;
	/// use web_thread::web::message::TransferableWrapper;
	///
	/// let context = AudioContext::new().unwrap();
	/// let buffer = TransferableWrapper(ArrayBuffer::new(1024));
	/// context
	/// 	.clone()
	/// 	.register_thread_with_message(
	/// 		None,
	/// 		|message| {
	/// 			let buffer: ArrayBuffer = message.0;
	/// 			// Do work.
	/// #           let _ = buffer;
	/// 		},
	/// 		buffer,
	/// 	)
	/// 	.await
	/// 	.unwrap();
	/// # }
	/// # #[cfg(not(all(target_feature = "atomics", not(unsupported_spawn))))]
	/// # let _ = test();
	/// ```
	///
	/// [`closed`]: https://developer.mozilla.org/en-US/docs/Web/API/BaseAudioContext/state#closed
	#[cfg_attr(
		not(all(
			target_family = "wasm",
			target_os = "unknown",
			feature = "audio-worklet"
		)),
		doc = "[`BaseAudioContext`]: https://docs.rs/web-sys/0.3.68/web_sys/struct.BaseAudioContext.html"
	)]
	#[cfg(any(feature = "message", docsrs))]
	fn register_thread_with_message<F, M>(
		self,
		stack_size: Option<usize>,
		shim_url: Option<&str>,
		f: F,
		message: M,
	) -> RegisterThreadFuture
	where
		F: 'static + FnOnce(M) + Send,
		M: 'static + MessageSend;

	/// Instantiates a [`AudioWorkletProcessor`]. No `data` will be delivered if
	/// `name` corresponds to a different type registered with
	/// [`AudioWorkletGlobalScopeExt::register_processor_ext()`]. If `name`
	/// corresponds to a [`AudioWorkletProcessor`] not registered through
	/// [`AudioWorkletGlobalScopeExt::register_processor_ext()`], it will leak
	/// `data`.
	///
	/// # Errors
	///
	/// - If [`Self::register_thread()`] was not called on this context yet.
	/// - If [`new AudioWorkletNode`] throws an exception.
	///
	/// # Example
	///
	/// ```
	/// # #[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
	/// # wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
	/// # #[cfg_attr(all(target_feature = "atomics", not(unsupported_spawn)), wasm_bindgen_test::wasm_bindgen_test)]
	/// # async fn test() {
	/// # use wasm_bindgen::JsCast;
	/// use web_sys::{AudioContext, AudioWorkletGlobalScope, AudioWorkletNodeOptions, AudioWorkletProcessor};
	/// use web_thread::web::{self, YieldTime};
	/// use web_thread::web::audio_worklet::{AudioWorkletGlobalScopeExt, BaseAudioContextExt, ExtendAudioWorkletProcessor};
	///
	/// /// Example [`AudioWorkletProcessor`].
	/// struct TestProcessor;
	///
	/// impl ExtendAudioWorkletProcessor for TestProcessor {
	/// 	type Data = String;
	///
	/// 	fn new(
	/// 		_: AudioWorkletProcessor,
	/// 		data: Option<Self::Data>,
	/// 		_: AudioWorkletNodeOptions,
	/// 	) -> Self {
	/// 		assert_eq!(data.as_deref(), Some("test"));
	/// 		Self
	/// 	}
	/// }
	///
	/// let context = AudioContext::new().unwrap();
	/// let (sender, receiver) = async_channel::bounded(1);
	/// context.clone().register_thread(
	/// 	None,
	/// 	move || {
	/// 		let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
	/// 		global
	/// 			.register_processor_ext::<TestProcessor>("test")
	/// 			.unwrap();
	/// 		sender.try_send(()).unwrap();
	/// 	},
	/// ).await.unwrap();
	///
	/// // Wait until processor is registered.
	/// receiver.recv().await.unwrap();
	/// web::yield_now_async(YieldTime::UserBlocking).await;
	///
	/// let node = context.audio_worklet_node::<TestProcessor>("test", String::from("test"), None).unwrap();
	/// # let _ = node;
	/// # }
	/// # #[cfg(not(all(target_feature = "atomics", not(unsupported_spawn))))]
	/// # let _ = test();
	/// ```
	///
	/// [`new AudioWorkletNode`]: https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletNode/AudioWorkletNode
	/// [`AudioWorkletProcessor`]: https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletProcessor
	fn audio_worklet_node<P>(
		&self,
		name: &str,
		data: P::Data,
		options: Option<&AudioWorkletNodeOptions>,
	) -> Result<AudioWorkletNode, AudioWorkletNodeError<P>>
	where
		P: 'static + ExtendAudioWorkletProcessor;
}

#[cfg(all(
	target_family = "wasm",
	target_os = "unknown",
	feature = "audio-worklet"
))]
impl<T> BaseAudioContextExt for T
where
	BaseAudioContext: From<T>,
	T: AsRef<BaseAudioContext>,
{
	fn register_thread<F>(
		self,
		stack_size: Option<usize>,
		shim_url: Option<&str>,
		#[allow(clippy::min_ident_chars)] f: F,
	) -> RegisterThreadFuture
	where
		F: 'static + FnOnce() + Send,
	{
		RegisterThreadFuture(audio_worklet::register_thread(
			self.into(),
			stack_size,
			shim_url,
			f,
		))
	}

	#[cfg(any(feature = "message", docsrs))]
	fn register_thread_with_message<F, M>(
		self,
		stack_size: Option<usize>,
		shim_url: Option<&str>,
		#[allow(clippy::min_ident_chars)] f: F,
		message: M,
	) -> RegisterThreadFuture
	where
		F: 'static + FnOnce(M) + Send,
		M: 'static + MessageSend,
	{
		RegisterThreadFuture(audio_worklet::register_thread_with_message(
			self.into(),
			stack_size,
			shim_url,
			f,
			message,
		))
	}

	fn audio_worklet_node<P>(
		&self,
		name: &str,
		data: P::Data,
		options: Option<&AudioWorkletNodeOptions>,
	) -> Result<AudioWorkletNode, AudioWorkletNodeError<P>>
	where
		P: 'static + ExtendAudioWorkletProcessor,
	{
		audio_worklet::audio_worklet_node(self.as_ref(), name, data, options)
	}
}

/// Error returned by [`BaseAudioContextExt::audio_worklet_node()`].
pub struct AudioWorkletNodeError<P>
where
	P: ExtendAudioWorkletProcessor,
{
	/// The passed [`ExtendAudioWorkletProcessor::Data`].
	pub data: P::Data,
	/// The error thrown by [`new AudioWorkletNode`].
	///
	/// [`new AudioWorkletNode`]: https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletNode/AudioWorkletNode
	pub error: io::Error,
}

impl<P> Debug for AudioWorkletNodeError<P>
where
	P: ExtendAudioWorkletProcessor,
{
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("AudioWorkletNodeError")
			.field("data", &any::type_name::<P::Data>())
			.field("error", &self.error)
			.finish()
	}
}

impl<P> Display for AudioWorkletNodeError<P>
where
	P: ExtendAudioWorkletProcessor,
{
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		Display::fmt(&self.error, formatter)
	}
}

impl<P> Error for AudioWorkletNodeError<P>
where
	P: ExtendAudioWorkletProcessor,
{
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		Some(&self.error)
	}
}

/// Waits for the associated thread to register. See
/// [`BaseAudioContextExt::register_thread()`].
#[derive(Debug)]
pub struct RegisterThreadFuture(audio_worklet::RegisterThreadFuture);

impl Future for RegisterThreadFuture {
	type Output = io::Result<AudioWorkletHandle>;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		Pin::new(&mut self.0).poll(cx).map_ok(AudioWorkletHandle)
	}
}

impl RefUnwindSafe for RegisterThreadFuture {}

/// Handle to the audio worklet. See [`BaseAudioContextExt::register_thread()`].
#[derive(Debug)]
pub struct AudioWorkletHandle(audio_worklet::AudioWorkletHandle);

impl AudioWorkletHandle {
	/// Extracts a handle to the underlying thread.
	///
	/// # Example
	///
	/// ```
	/// # #[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
	/// # wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
	/// # #[cfg_attr(all(target_feature = "atomics", not(unsupported_spawn)), wasm_bindgen_test::wasm_bindgen_test)]
	/// # async fn test() {
	/// use web_sys::{AudioContext, console};
	/// use web_thread::web::audio_worklet::BaseAudioContextExt;
	///
	/// let context = AudioContext::new().unwrap();
	/// let handle = context.clone().register_thread(
	/// 	None,
	/// 	|| {
	/// 		// Do work.
	/// 	},
	/// ).await.unwrap();
	///
	/// console::log_1(&format!("thread id: {:?}", handle.thread().id()).into());
	/// # }
	/// # #[cfg(not(all(target_feature = "atomics", not(unsupported_spawn))))]
	/// # let _ = test();
	/// ```
	#[must_use]
	pub const fn thread(&self) -> &Thread {
		self.0.thread()
	}

	/// This releases memory allocated for the corresponding audio worklet
	/// thread.
	///
	/// # Safety
	///
	/// The corresponding thread must not currently or in the future access this
	/// Wasm module.
	///
	/// # Errors
	///
	/// If called from its corresponding audio worklet thread.
	///
	/// # Example
	///
	/// ```
	/// # #[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
	/// # wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
	/// # #[cfg_attr(all(target_feature = "atomics", not(unsupported_spawn)), wasm_bindgen_test::wasm_bindgen_test)]
	/// # async fn test() {
	/// use wasm_bindgen_futures::JsFuture;
	/// use web_sys::AudioContext;
	/// use web_thread::web::audio_worklet::BaseAudioContextExt;
	///
	/// let context = AudioContext::new().unwrap();
	/// let (sender, receiver) = async_channel::bounded(1);
	/// let handle = context.clone().register_thread(
	/// 	None,
	/// 	move || {
	/// 		// Do work.
	/// 		sender.try_send(()).unwrap();
	/// 	},
	/// ).await.unwrap();
	///
	/// // Wait until audio worklet is finished.
	/// receiver.recv().await.unwrap();
	/// JsFuture::from(context.close().unwrap()).await.unwrap();
	/// // SAFETY: We are sure we are done with the audio worklet and didn't register any
	/// // events or promises that could call into the Wasm module later.
	/// unsafe { handle.release() }.unwrap();
	/// # }
	/// # #[cfg(not(all(target_feature = "atomics", not(unsupported_spawn))))]
	/// # let _ = test();
	/// ```
	pub unsafe fn release(self) -> Result<(), ReleaseError> {
		// SAFETY: See `ThreadMemory::release()`. Other safety guarantees have to be
		// uphold by the caller.
		unsafe { self.0.release() }
			.map_err(Self)
			.map_err(ReleaseError)
	}
}

/// Returned on error in [`AudioWorkletHandle::release()`].
#[derive(Debug)]
pub struct ReleaseError(pub AudioWorkletHandle);

impl Display for ReleaseError {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter.write_str(
			"called `AudioWorkletHandle::release()` from its corresponding audio worklet thread",
		)
	}
}

impl Error for ReleaseError {}

/// Extension for [`AudioWorkletGlobalScope`].
#[cfg_attr(
	not(all(
		target_family = "wasm",
		target_os = "unknown",
		feature = "audio-worklet"
	)),
	doc = "",
	doc = "[`AudioWorkletGlobalScope`]: https://docs.rs/web-sys/0.3.68/web_sys/struct.AudioWorkletGlobalScope.html"
)]
pub trait AudioWorkletGlobalScopeExt {
	/// Creates a class that extends [`AudioWorkletProcessor`] and calls
	/// [`AudioWorkletGlobalScope.registerProcessor()`]. This is a workaround
	/// for [`wasm-bindgen`] currently unable to extend classes, see
	/// [this `wasm-bindgen` issue](https://github.com/rustwasm/wasm-bindgen/issues/210).
	///
	/// # Notes
	///
	/// [`AudioWorkletGlobalScope.registerProcessor()`] does not sync with it's
	/// corresponding [`AudioWorkletNode`] immediately and requires at least one
	/// yield to the event loop cycle in the [`AudioWorkletNode`]s thread for
	/// [`AudioWorkletNode::new()`] to successfully find the requested
	/// [`AudioWorkletProcessor`] by its name. See [`yield_now_async()`].
	///
	/// # Errors
	///
	/// - If the `name` is empty.
	/// - If a processor with this `name` is already registered.
	/// - If this thread was not spawned by [`web-thread`](crate).
	///
	/// # Example
	///
	/// ```
	/// # #[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
	/// # wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
	/// # #[cfg_attr(all(target_feature = "atomics", not(unsupported_spawn)), wasm_bindgen_test::wasm_bindgen_test)]
	/// # async fn test() {
	/// # use wasm_bindgen::JsCast;
	/// use web_sys::{AudioContext, AudioWorkletGlobalScope, AudioWorkletNode};
	/// # use web_sys::{AudioWorkletNodeOptions, AudioWorkletProcessor};
	/// use web_thread::web::{self, YieldTime};
	/// use web_thread::web::audio_worklet::{AudioWorkletGlobalScopeExt, BaseAudioContextExt};
	/// # use web_thread::web::audio_worklet::ExtendAudioWorkletProcessor;
	///
	/// # struct TestProcessor;
	/// # impl ExtendAudioWorkletProcessor for TestProcessor {
	/// # 	type Data = ();
	/// # 	fn new(
	/// # 		_: AudioWorkletProcessor,
	/// # 		_: Option<Self::Data>,
	/// # 		_: AudioWorkletNodeOptions,
	/// # 	) -> Self {
	/// # 		Self
	/// # 	}
	/// # }
	/// #
	/// let context = AudioContext::new().unwrap();
	/// let (sender, receiver) = async_channel::bounded(1);
	/// context.clone().register_thread(
	/// 	None,
	/// 	move || {
	/// 		let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
	/// 		global
	/// 			.register_processor_ext::<TestProcessor>("test")
	/// 			.unwrap();
	/// 		sender.try_send(()).unwrap();
	/// 	},
	/// ).await.unwrap();
	///
	/// // Wait until processor is registered.
	/// receiver.recv().await.unwrap();
	/// web::yield_now_async(YieldTime::UserBlocking).await;
	///
	/// let node = AudioWorkletNode::new(&context, "test").unwrap();
	/// # let _ = node;
	/// # }
	/// # #[cfg(not(all(target_feature = "atomics", not(unsupported_spawn))))]
	/// # let _ = test();
	/// ```
	///
	/// [`AudioWorkletGlobalScope.registerProcessor()`]: https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletGlobalScope/registerProcessor
	/// [`AudioWorkletProcessor`]: https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletProcessor
	/// [`yield_now_async()`]: super::yield_now_async
	#[cfg_attr(
		all(
			target_family = "wasm",
			target_os = "unknown",
			feature = "audio-worklet"
		),
		doc = "[`AudioWorkletNode`]: web_sys::AudioWorkletNode",
		doc = "[`AudioWorkletNode::new()`]: web_sys::AudioWorkletNode::new"
	)]
	#[cfg_attr(
		not(all(
			target_family = "wasm",
			target_os = "unknown",
			feature = "audio-worklet"
		)),
		doc = "[`AudioWorkletNode`]: https://docs.rs/web-sys/0.3.68/web_sys/struct.AudioWorkletNode.html",
		doc = "[`AudioWorkletNode::new()`]: https://docs.rs/web-sys/0.3.68/web_sys/struct.AudioWorkletNode.html#method.new"
	)]
	#[cfg_attr(
		all(target_family = "wasm", target_os = "unknown"),
		doc = "[`wasm-bindgen`]: wasm_bindgen"
	)]
	#[cfg_attr(
		not(all(target_family = "wasm", target_os = "unknown")),
		doc = "[`wasm-bindgen`]: https://docs.rs/wasm-bindgen/0.2.91"
	)]
	fn register_processor_ext<P>(&self, name: &str) -> Result<(), io::Error>
	where
		P: 'static + ExtendAudioWorkletProcessor;
}

#[cfg(all(
	target_family = "wasm",
	target_os = "unknown",
	feature = "audio-worklet"
))]
impl AudioWorkletGlobalScopeExt for AudioWorkletGlobalScope {
	fn register_processor_ext<P>(&self, name: &str) -> Result<(), io::Error>
	where
		P: 'static + ExtendAudioWorkletProcessor,
	{
		audio_worklet::register_processor::<P>(name)
	}
}

/// Extends type with [`AudioWorkletProcessor`].
///
/// [`AudioWorkletProcessor`]: https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletProcessor
pub trait ExtendAudioWorkletProcessor {
	/// Data passed into [`Self::new()`] when using
	/// [`BaseAudioContextExt::audio_worklet_node()`].
	type Data: 'static + Send;

	/// Equivalent to [`AudioWorkletProcessor()`].
	///
	/// [`AudioWorkletProcessor()`]: https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletProcessor/AudioWorkletProcessor
	fn new(
		this: AudioWorkletProcessor,
		data: Option<Self::Data>,
		options: AudioWorkletNodeOptions,
	) -> Self;

	/// Equivalent to [`AudioWorkletProcessor.process()`].
	///
	/// [`AudioWorkletProcessor.process()`]: https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletProcessor/process
	#[allow(unused_variables)]
	fn process(&mut self, inputs: Array, outputs: Array, parameters: Object) -> bool {
		false
	}

	/// Equivalent to [`AudioWorkletProcessor.parameterDescriptors`].
	///
	/// [`AudioWorkletProcessor.parameterDescriptors`]: https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletProcessor/parameterDescriptors
	#[allow(clippy::must_use_candidate)]
	fn parameter_descriptors() -> Iterator {
		Array::new().values()
	}
}
