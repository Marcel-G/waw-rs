//! Handling message related functionality.

use std::cell::OnceCell;
use std::ptr::NonNull;

use js_sys::{Array, Function};
use wasm_bindgen::prelude::{wasm_bindgen, JsValue};
use web_sys::{BaseAudioContext, MessagePort};

use super::super::super::memory::ThreadMemory;
use super::super::super::oneshot::Sender;
use super::super::super::spawn::message::HasMessagePortInterface;
use super::super::super::Thread;
use super::RegisterThreadFuture;
use crate::web::message::{ArrayBuilder, MessageSend};

thread_local! {
	pub(in super::super::super) static MESSAGE_PORT: OnceCell<MessagePort> = const { OnceCell::new() };
}

/// Message to be sent.
#[derive(Debug)]
pub(super) struct MessageState {
	/// Values to be [serialized](https://developer.mozilla.org/en-US/docs/Glossary/Serializable_object).
	pub(super) serialize: JsValue,
	/// Values to be [transferred](https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API/Transferable_objects).
	pub(super) transfer: Option<Array>,
}

/// Implementation for
/// [`crate::web::audio_worklet::BaseAudioContextExt::register_thread_with_message()`].
pub(in super::super::super::super) fn register_thread_with_message<F, M>(
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
	let mut transfer_builder = ArrayBuilder::new();
	let raw_message = message.send(&mut transfer_builder);
	let transfer = transfer_builder.finish();
	let message = raw_message.serialize.map(|serialize| MessageState {
		serialize,
		transfer,
	});

	super::register_thread_internal(
		context,
		stack_size,
		shim_url,
		move |message| {
			let message = (!message.is_undefined()).then_some(message);
			let message = M::receive(message, raw_message.send);
			task(message);
		},
		message,
	)
}

impl HasMessagePortInterface for MessagePort {
	fn set_onmessage(&self, value: Option<&Function>) {
		self.set_onmessage(value);
	}

	fn post_message(&self, message: &JsValue) -> Result<(), JsValue> {
		self.post_message(message)
	}

	fn post_message_with_transfer(
		&self,
		message: &JsValue,
		transfer: &JsValue,
	) -> Result<(), JsValue> {
		self.post_message_with_transferable(message, transfer)
	}
}

/// Data sent to initialize the audio worklet.
#[derive(Debug)]
#[cfg(feature = "message")]
pub(super) struct Data {
	/// [`Thread`].
	pub(super) thread: Thread,
	/// Stack size of the thread.
	pub(super) stack_size: Option<usize>,
	/// [`Sender`] to send back the associated [`ThreadMemory`].
	pub(super) memory_sender: Sender<ThreadMemory>,
}

/// Register function for the worklet.
///
/// # Safety
///
/// `data` has to be a valid pointer to [`Data`].
#[wasm_bindgen(skip_typescript)]
#[allow(private_interfaces, unreachable_pub)]
pub unsafe fn __web_thread_worklet_register(data: NonNull<Data>) {
	// SAFETY: Has to be a valid pointer to a `Data`. We only call
	// `__web_thread_worklet_register` from `worklet_with_message.js`. The data sent
	// to it comes only from `RegisterThreadFuture::poll()`.
	let data: Data = *unsafe { Box::from_raw(data.as_ptr()) };

	Thread::register(data.thread);
	data.memory_sender.send(ThreadMemory::new(data.stack_size));
}
