//! Handling message related functionality.

use std::cell::RefCell;
use std::future::Future;
use std::sync::Arc;
use std::{io, mem};

use js_sys::{Array, Function};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent, Worker};

use super::super::super::global::Global;
#[cfg(feature = "audio-worklet")]
use super::super::audio_worklet::register::THREAD_LOCK_INDEXES;
#[cfg(feature = "audio-worklet")]
use super::super::js::ArrayExt;
use super::super::{channel, main, oneshot, JoinHandle, ScopeData, ThreadId};
use super::{SpawnData, Task};
use crate::thread::atomics::channel::Receiver;
use crate::web::message::{ArrayBuilder, MessageSend};

thread_local! {
	pub(in super::super) static SPAWN_SENDER: RefCell<Option<channel::Sender<SpawnData>>> = const { RefCell::new(None) };
}

/// Internal spawn function.
///
/// # Safety
///
/// `task` and `message` have to outlive the thread.
pub(in super::super) unsafe fn spawn<F1, F2, T, M>(
	task: F1,
	name: Option<String>,
	stack_size: Option<usize>,
	scope: Option<Arc<ScopeData>>,
	message: M,
) -> io::Result<JoinHandle<T>>
where
	F1: FnOnce(M) -> F2 + Send,
	F2: Future<Output = T>,
	T: Send,
	M: MessageSend,
{
	let thread = super::thread_init(name, scope.as_deref());
	let (result_sender, result_receiver) = oneshot::channel();
	let (spawn_sender, spawn_receiver) = channel::channel();

	let mut transfer_builder = ArrayBuilder::new();
	let raw_message = message.send(&mut transfer_builder);
	let transfer = transfer_builder.finish();

	let task: Task<'_> = Box::new({
		let thread = thread.clone();
		move |message| {
			super::thread_runner(
				thread,
				stack_size,
				result_sender,
				spawn_sender,
				scope,
				move || {
					let message = (!message.is_undefined()).then_some(message);
					let message = M::receive(message, raw_message.send);
					task(message)
				},
			)
		}
	});

	if let Some(serialize) = raw_message.serialize {
		if super::super::is_main_thread() {
			main::init_main_thread();

			spawn_internal(
				thread.id(),
				thread.name(),
				stack_size,
				spawn_receiver,
				&serialize,
				transfer,
				Box::new(task),
			)?;
		} else {
			// SAFETY: `task` has to be `'static` or `scope` has to be `Some`, which
			// prevents this thread from outliving its lifetime.
			let task = unsafe { mem::transmute::<Task<'_>, Task<'static>>(task) };

			let data = SpawnData {
				id: thread.id(),
				name: thread.0.name.clone(),
				stack_size,
				spawn_receiver,
				task,
			};

			SPAWN_SENDER
				.with(|cell| {
					cell.borrow()
						.as_ref()
						.expect("found no `Sender` in existing thread")
						.send(data)
				})
				.expect("`Receiver` in main thread dropped");

			Global::with(|global| match global {
				Global::Dedicated(global) => send_message(global, &serialize, transfer),
				#[cfg(feature = "audio-worklet")]
				Global::Worklet => super::super::audio_worklet::register::message::MESSAGE_PORT.with(|port| {
					let port = port
						.get()
						.expect("found audio worklet with uninitialized port");
					send_message(port, &serialize, transfer)
				}),
				_ => unreachable!("spawning from thread not registered by `web-thread`"),
			})?;
		}

		Ok(JoinHandle {
			receiver: Some(result_receiver),
			thread,
		})
	} else {
		Ok(super::spawn_without_message(
			thread,
			stack_size,
			result_receiver,
			spawn_receiver,
			task,
		))
	}
}

/// Send [`MessageSend`] over any [`HasMessagePortInterface`].
fn send_message(
	port: &impl HasMessagePortInterface,
	serialize: &JsValue,
	transfer: Option<Array>,
) -> io::Result<()> {
	let result = if let Some(transfer) = transfer {
		port.post_message_with_transfer(&Array::of2(serialize, &transfer), &transfer)
	} else {
		port.post_message(&Array::of1(serialize))
	};

	if let Err(error) = result {
		port.post_message(&JsValue::UNDEFINED).expect(
			"`DedicatedWorkerGlobalScope.postMessage()` is not expected to fail without a \
			 `transfer` object",
		);
		Err(super::super::error_from_exception(error))
	} else {
		Ok(())
	}
}

/// Spawning thread regardless of being nested.
fn spawn_internal(
	id: ThreadId,
	name: Option<&str>,
	stack_size: Option<usize>,
	spawn_receiver: Receiver<SpawnData>,
	serialize: &JsValue,
	transfer: Option<Array>,
	task: Task<'_>,
) -> io::Result<()> {
	let result = super::spawn_common(
		id,
		name,
		spawn_receiver,
		task,
		#[cfg(not(feature = "audio-worklet"))]
		|worker: &Worker, module, memory, task| {
			if let Some(transfer) = transfer {
				worker.post_message_with_transfer(
					&Array::of5(module, memory, &stack_size.into(), &task, serialize),
					&transfer,
				)
			} else {
				worker.post_message(&Array::of5(
					module,
					memory,
					&stack_size.into(),
					&task,
					serialize,
				))
			}
		},
		#[cfg(feature = "audio-worklet")]
		|worker: &Worker, module, memory, task| {
			THREAD_LOCK_INDEXES.with(|indexes| {
				if let Some(transfer) = transfer {
					worker.post_message_with_transfer(
						&ArrayExt::of6(
							module,
							memory,
							&stack_size.into(),
							indexes,
							&task,
							serialize,
						),
						&transfer,
					)
				} else {
					worker.post_message(&ArrayExt::of6(
						module,
						memory,
						&stack_size.into(),
						indexes,
						&task,
						serialize,
					))
				}
			})
		},
	);

	if let Err(error) = result {
		Err(super::super::error_from_exception(error))
	} else {
		Ok(())
	}
}

/// Trait over any type having an interface like
/// [`MessagePort`](web_sys::MessagePort).
pub(in super::super) trait HasMessagePortInterface {
	/// Setter for the [`message`](https://developer.mozilla.org/en-US/docs/Web/API/MessagePort/message_event) event handler.
	fn set_onmessage(&self, value: Option<&Function>);

	/// [`MessagePort.postMessage()`](https://developer.mozilla.org/en-US/docs/Web/API/MessagePort/postMessage).
	fn post_message(&self, message: &JsValue) -> Result<(), JsValue>;

	/// [`MessagePort.postMessage()`](https://developer.mozilla.org/en-US/docs/Web/API/MessagePort/postMessage).
	fn post_message_with_transfer(
		&self,
		message: &JsValue,
		transfer: &JsValue,
	) -> Result<(), JsValue>;
}

impl HasMessagePortInterface for Worker {
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
		self.post_message_with_transfer(message, transfer)
	}
}

impl HasMessagePortInterface for DedicatedWorkerGlobalScope {
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
		self.post_message_with_transfer(message, transfer)
	}
}

/// Setup `message` event handler.
pub(in super::super) fn setup_message_handler(
	this: &impl HasMessagePortInterface,
	spawn_receiver: Receiver<SpawnData>,
) -> Closure<dyn Fn(MessageEvent)> {
	let message_handler = Closure::new(move |event: MessageEvent| {
		let data = spawn_receiver
			.try_recv()
			.expect("expected data to have been sent before message");
		let message = event.data();

		if message.is_undefined() {
			return;
		}

		let mut values = message.unchecked_into::<Array>().into_iter();
		let serialize = values.next().expect("no serialized data found");
		let transfer = values.next().map(Array::unchecked_from_js);

		spawn_internal(
			data.id,
			data.name.as_deref(),
			data.stack_size,
			data.spawn_receiver,
			&serialize,
			transfer,
			Box::new(data.task),
		)
		.expect("unexpected serialization error when serialization succeeded when sending this");
	});
	this.set_onmessage(Some(message_handler.as_ref().unchecked_ref()));

	message_handler
}
