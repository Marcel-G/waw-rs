//! Registering an audio worklet thread on a [`BaseAudioContext`].

#[cfg(feature = "message")]
pub(in super::super) mod message;

use std::arch::wasm32;
use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::io::{Error, ErrorKind};
use std::pin::Pin;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicI32, Ordering};
use std::task::{Context, Poll};
use std::{any, io};

use js_sys::Array;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
	AudioContextState, AudioWorkletNode, AudioWorkletNodeOptions, BaseAudioContext, MessagePort,
};
#[cfg(feature = "message")]
use {
	self::message::{Data, MessageState},
	super::super::channel,
	super::super::spawn::message::SPAWN_SENDER,
	super::super::spawn::SpawnData,
	super::main::WORKLETS,
	web_sys::MessageChannel,
};

use super::super::js::{Meta, META};
use super::super::memory::ThreadMemory;
use super::super::url::ScriptUrl;
use super::super::wait_async::WaitAsync;
use super::super::{main, oneshot, Thread, MEMORY, MODULE};
use super::js::BaseAudioContextExt;
use crate::thread::atomics::is_main_thread;

/// Type of the task being sent to the worklet.
type Task = Box<dyn 'static + FnOnce(JsValue) + Send>;

/// Locks instantiating workers until worklets have finished instantiating.
static WORKLET_LOCK: AtomicI32 = AtomicI32::new(0);
/// Counts how many workers are currently instantiating.
static WORKER_LOCK: AtomicI32 = AtomicI32::new(0);

thread_local! {
	/// Cached [`JsValue`] holding index to worklet lock.
	pub(in super::super) static WORKLET_LOCK_INDEX: JsValue =
		super::super::i32_to_buffer_index(WORKLET_LOCK.as_ptr()).into();
	/// Cached [`Array`] holding indexes to worker and worklet locks.
	pub(in super::super) static THREAD_LOCK_INDEXES: Array =
		WORKLET_LOCK_INDEX.with(|worklet_index| {
			Array::of2(
				worklet_index,
				&super::super::i32_to_buffer_index(WORKER_LOCK.as_ptr()).into(),
			)
		});
}

/// Implementation for
/// [`crate::web::audio_worklet::BaseAudioContextExt::register_thread()`].
pub(in super::super::super) fn register_thread<F>(
	context: BaseAudioContext,
	stack_size: Option<usize>,
	shim_url: Option<&str>,
	task: F,
) -> RegisterThreadFuture
where
	F: 'static + FnOnce() + Send,
{
	register_thread_internal(
		context,
		stack_size,
		shim_url,
		|_| task(),
		#[cfg(feature = "message")]
		None,
	)
}

/// Register thread regardless of message.
fn register_thread_internal(
	context: BaseAudioContext,
	stack_size: Option<usize>,
	shim_url: Option<&str>,
	task: impl 'static + FnOnce(JsValue) + Send,
	#[cfg(feature = "message")] message: Option<MessageState>,
) -> RegisterThreadFuture {
	// Build the script URL using the provided shim URL or fall back to import.meta.url.
	let shim_url_string = shim_url
		.map(String::from)
		.unwrap_or_else(|| META.with(Meta::url));

	let script_url = {
		#[cfg(not(feature = "message"))]
		let template = include_str!("../../script/worklet.min.js");
		#[cfg(feature = "message")]
		let template = include_str!("../../script/worklet_with_message.min.js");

		ScriptUrl::new(&template.replacen("@shim.js", &shim_url_string, 1))
	};

	if let AudioContextState::Closed = context.state() {
		return RegisterThreadFuture(Some(State::Error(Error::other(
			"`BaseAudioContext` is closed",
		))));
	}

	if let Some(true) = context.unchecked_ref::<BaseAudioContextExt>().registered() {
		return RegisterThreadFuture(Some(State::Error(Error::new(
			ErrorKind::AlreadyExists,
			"`BaseAudioContext` already registered a thread",
		))));
	}

	let worklet = context
		.audio_worklet()
		.expect("`BaseAudioContext.audioWorklet` expected to be valid");

	RegisterThreadFuture(Some(
		match worklet.add_module(script_url.as_raw()) {
			Ok(promise) => {
				context
					.unchecked_ref::<BaseAudioContextExt>()
					.set_registered(true);
				let promise = JsFuture::from(promise);
				let (memory_sender, memory_receiver) = oneshot::channel();
				#[cfg(feature = "message")]
				let (spawn_sender, spawn_receiver) = channel::channel();
				let thread = Thread::new_with_name(None);

				let task = Box::new({
					#[cfg(not(feature = "message"))]
					let thread = thread.clone();
					move |message| {
						#[cfg(not(feature = "message"))]
						{
							Thread::register(thread);
							memory_sender.send(ThreadMemory::new(stack_size));
						}
						#[cfg(feature = "message")]
						{
							let old =
								SPAWN_SENDER.with(|cell| cell.borrow_mut().replace(spawn_sender));
							debug_assert!(old.is_none(), "found existing `Sender` in new thread");
						}
						task(message);
					}
				});

				State::Module {
					context,
					promise,
					thread,
					task,
					stack_size,
					#[cfg(feature = "message")]
					memory_sender,
					memory_receiver,
					#[cfg(feature = "message")]
					spawn_receiver,
					#[cfg(feature = "message")]
					message,
				}
			}
			Err(error) => State::Error(super::super::error_from_exception(error)),
		},
	))
}

/// Implementation for [`crate::web::audio_worklet::RegisterThreadFuture`].
#[derive(Debug)]
pub(in super::super::super) struct RegisterThreadFuture(Option<State>);

/// State of [`RegisterThreadFuture`].
enum State {
	/// Early error.
	Error(Error),
	/// Waiting for `Worklet.addModule()`.
	Module {
		/// Corresponding [`BaseAudioContext`].
		context: BaseAudioContext,
		/// `Promise` returned by `Worklet.addModule()`.
		promise: JsFuture,
		/// [`Thread`].
		thread: Thread,
		/// Stack size of the thread.
		stack_size: Option<usize>,
		/// Caller-supplied task.
		task: Task,
		/// [`Receiver`](oneshot::Sender) for [`ThreadMemory`].
		#[cfg(feature = "message")]
		memory_sender: oneshot::Sender<ThreadMemory>,
		/// [`Receiver`](oneshot::Receiver) for [`ThreadMemory`].
		memory_receiver: oneshot::Receiver<ThreadMemory>,
		/// [`Receiver`](channel::Receiver) for [`SpawnData`].
		#[cfg(feature = "message")]
		spawn_receiver: channel::Receiver<SpawnData>,
		/// Message to be sent.
		#[cfg(feature = "message")]
		message: Option<MessageState>,
	},
	/// Waiting for the worklet lock to be available.
	WorkletLock {
		/// [`Future`] waiting for the worklet lock to be available.
		future: Option<WaitAsync>,
		/// Corresponding [`BaseAudioContext`].
		context: BaseAudioContext,
		/// [`Thread`].
		thread: Thread,
		/// Stack size of the thread.
		stack_size: Option<usize>,
		/// Caller-supplied task.
		task: Task,
		/// [`Receiver`](oneshot::Sender) for [`ThreadMemory`].
		#[cfg(feature = "message")]
		memory_sender: oneshot::Sender<ThreadMemory>,
		/// [`Receiver`](oneshot::Receiver) for [`ThreadMemory`].
		memory_receiver: oneshot::Receiver<ThreadMemory>,
		/// [`Receiver`](channel::Receiver) for [`SpawnData`].
		#[cfg(feature = "message")]
		spawn_receiver: channel::Receiver<SpawnData>,
		/// Message to be sent.
		#[cfg(feature = "message")]
		message: Option<MessageState>,
	},
	/// Waiting for the worker lock to be available.
	WorkerLock {
		/// [`Future`] waiting for the worker lock to be available.
		future: Option<WaitAsync>,
		/// Corresponding [`BaseAudioContext`].
		context: BaseAudioContext,
		/// [`Thread`].
		thread: Thread,
		/// Stack size of the thread.
		stack_size: Option<usize>,
		/// Caller-supplied task.
		task: Task,
		/// [`Receiver`](oneshot::Sender) for [`ThreadMemory`].
		#[cfg(feature = "message")]
		memory_sender: oneshot::Sender<ThreadMemory>,
		/// [`Receiver`](oneshot::Receiver) for [`ThreadMemory`].
		memory_receiver: oneshot::Receiver<ThreadMemory>,
		/// [`Receiver`](channel::Receiver) for [`SpawnData`].
		#[cfg(feature = "message")]
		spawn_receiver: channel::Receiver<SpawnData>,
		/// Message to be sent.
		#[cfg(feature = "message")]
		message: Option<MessageState>,
	},
	/// Waiting for [`ThreadMemory`].
	Memory {
		/// Corresponding [`BaseAudioContext`].
		context: BaseAudioContext,
		/// [`Thread`].
		thread: Thread,
		/// [`Receiver`](oneshot::Receiver) for [`ThreadMemory`].
		memory_receiver: oneshot::Receiver<ThreadMemory>,
		/// Caller-supplied task.
		#[cfg(feature = "message")]
		task: Task,
		/// [`AudioWorkletNode`] used to initialize the Wasm module.
		#[cfg(feature = "message")]
		node: AudioWorkletNode,
		/// [`Receiver`](channel::Receiver) for [`SpawnData`].
		#[cfg(feature = "message")]
		spawn_receiver: channel::Receiver<SpawnData>,
		/// Message to be sent.
		#[cfg(feature = "message")]
		message: Option<MessageState>,
	},
}

impl Debug for State {
	#[allow(clippy::too_many_lines)]
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::Error(error) => formatter.debug_tuple("Error").field(error).finish(),
			Self::Module {
				context,
				promise,
				thread,
				stack_size,
				task,
				#[cfg(feature = "message")]
				memory_sender,
				memory_receiver,
				#[cfg(feature = "message")]
				spawn_receiver,
				#[cfg(feature = "message")]
				message,
			} => {
				let mut debug_struct = formatter.debug_struct("Module");
				debug_struct
					.field("context", context)
					.field("promise", promise)
					.field("thread", thread)
					.field("stack_size", stack_size)
					.field("task", &any::type_name_of_val(task));
				#[cfg(feature = "message")]
				debug_struct.field("memory_sender", memory_sender);
				debug_struct.field("memory_receiver", memory_receiver);
				#[cfg(feature = "message")]
				debug_struct
					.field("spawn_receiver", spawn_receiver)
					.field("message", message);
				debug_struct.finish()
			}
			Self::WorkletLock {
				future,
				context,
				thread,
				stack_size,
				task,
				#[cfg(feature = "message")]
				memory_sender,
				memory_receiver,
				#[cfg(feature = "message")]
				spawn_receiver,
				#[cfg(feature = "message")]
				message,
			}
			| Self::WorkerLock {
				future,
				context,
				thread,
				stack_size,
				task,
				#[cfg(feature = "message")]
				memory_sender,
				memory_receiver,
				#[cfg(feature = "message")]
				spawn_receiver,
				#[cfg(feature = "message")]
				message,
			} => {
				let mut debug_struct = formatter.debug_struct(match self {
					Self::WorkletLock { .. } => "WorkletLock",
					Self::WorkerLock { .. } => "WorkerLock",
					_ => unreachable!(),
				});
				debug_struct
					.field("future", future)
					.field("context", context)
					.field("thread", thread)
					.field("stack_size", stack_size)
					.field("task", &any::type_name_of_val(task));
				#[cfg(feature = "message")]
				debug_struct.field("memory_sender", memory_sender);
				debug_struct.field("memory_receiver", memory_receiver);
				#[cfg(feature = "message")]
				debug_struct
					.field("spawn_receiver", spawn_receiver)
					.field("message", message);
				debug_struct.finish()
			}
			Self::Memory {
				context,
				thread,
				memory_receiver,
				#[cfg(feature = "message")]
				task,
				#[cfg(feature = "message")]
				node,
				#[cfg(feature = "message")]
				spawn_receiver,
				#[cfg(feature = "message")]
				message,
			} => {
				let mut debug_struct = formatter.debug_struct("Module");
				debug_struct
					.field("context", context)
					.field("thread", thread)
					.field("memory_receiver", memory_receiver);
				#[cfg(feature = "message")]
				debug_struct
					.field("task", &any::type_name_of_val(task))
					.field("node", node)
					.field("spawn_receiver", spawn_receiver)
					.field("message", message);
				debug_struct.finish()
			}
		}
	}
}

impl Drop for RegisterThreadFuture {
	fn drop(&mut self) {
		let Some(state) = self.0.take() else { return };

		if !matches!(state, State::Error(_)) {
			wasm_bindgen_futures::spawn_local(async move {
				let _ = Self(Some(state)).await;
			});
		}
	}
}

impl Future for RegisterThreadFuture {
	type Output = io::Result<AudioWorkletHandle>;

	#[allow(clippy::too_many_lines)]
	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		loop {
			let mut state = self.0.take().expect("polled after completion");

			match state {
				State::Error(error) => return Poll::Ready(Err(error)),
				State::Module {
					ref mut promise, ..
				} => match Pin::new(promise).poll(cx) {
					Poll::Ready(Ok(_)) => {
						// This is checked earlier.
						debug_assert!(
							is_main_thread(),
							"started registering thread without being on the main thread"
						);
						// Before spawning a new thread make sure we initialize the main thread.
						main::init_main_thread();

						let State::Module {
							context,
							thread,
							stack_size,
							task,
							#[cfg(feature = "message")]
							memory_sender,
							memory_receiver,
							#[cfg(feature = "message")]
							spawn_receiver,
							#[cfg(feature = "message")]
							message,
							..
						} = state
						else {
							unreachable!("found wrong state")
						};

						self.0 = Some(State::WorkletLock {
							future: None,
							context,
							thread,
							stack_size,
							task,
							#[cfg(feature = "message")]
							memory_sender,
							memory_receiver,
							#[cfg(feature = "message")]
							spawn_receiver,
							#[cfg(feature = "message")]
							message,
						});
					}
					Poll::Ready(Err(error)) => {
						return Poll::Ready(Err(super::super::error_from_exception(error)))
					}
					Poll::Pending => {
						self.0 = Some(state);
						return Poll::Pending;
					}
				},
				State::WorkletLock { ref mut future, .. } => {
					if let Some(future) = future {
						if Pin::new(future).poll(cx).is_pending() {
							self.0 = Some(state);
							return Poll::Pending;
						}
					}

					if WORKLET_LOCK
						.compare_exchange_weak(0, 1, Ordering::Relaxed, Ordering::Relaxed)
						.is_err()
					{
						*future = Some(WaitAsync::wait(&WORKLET_LOCK, 1));
						self.0 = Some(state);
						continue;
					}

					let State::WorkletLock {
						context,
						thread,
						stack_size,
						task,
						#[cfg(feature = "message")]
						memory_sender,
						memory_receiver,
						#[cfg(feature = "message")]
						spawn_receiver,
						#[cfg(feature = "message")]
						message,
						..
					} = state
					else {
						unreachable!("found wrong state")
					};

					self.0 = Some(State::WorkerLock {
						future: None,
						context,
						thread,
						stack_size,
						task,
						#[cfg(feature = "message")]
						memory_sender,
						memory_receiver,
						#[cfg(feature = "message")]
						spawn_receiver,
						#[cfg(feature = "message")]
						message,
					});
				}
				State::WorkerLock { ref mut future, .. } => {
					if let Some(future) = future {
						if Pin::new(future).poll(cx).is_pending() {
							self.0 = Some(state);
							return Poll::Pending;
						}
					}

					let worker_lock = WORKER_LOCK.load(Ordering::Relaxed);

					if worker_lock != 0 {
						*future = Some(WaitAsync::wait(&WORKER_LOCK, worker_lock));
						self.0 = Some(state);
						continue;
					}

					let State::WorkerLock {
						context,
						thread,
						stack_size,
						task,
						#[cfg(feature = "message")]
						memory_sender,
						memory_receiver,
						#[cfg(feature = "message")]
						spawn_receiver,
						#[cfg(feature = "message")]
						message,
						..
					} = state
					else {
						unreachable!("found wrong state")
					};

					#[cfg(not(feature = "message"))]
					let data: NonNull<Task> = NonNull::from(Box::leak(Box::new(task)));
					#[cfg(feature = "message")]
					let data: NonNull<Data> = NonNull::from(Box::leak(Box::new(Data {
						thread: thread.clone(),
						stack_size,
						memory_sender,
					})));

					let options = AudioWorkletNodeOptions::new();
					options.set_processor_options(Some(&MODULE.with(|module| {
						MEMORY.with(|memory| {
							WORKLET_LOCK_INDEX.with(|index| {
								Array::of5(module, memory, &stack_size.into(), index, &data.into())
							})
						})
					})));

					match AudioWorkletNode::new_with_options(
						&context,
						"__web_thread_worklet",
						&options,
					) {
						Ok(node) => {
							#[cfg(not(feature = "message"))]
							drop(node);
							self.0 = Some(State::Memory {
								context,
								thread,
								memory_receiver,
								#[cfg(feature = "message")]
								task,
								#[cfg(feature = "message")]
								node,
								#[cfg(feature = "message")]
								spawn_receiver,
								#[cfg(feature = "message")]
								message,
							});
						}
						Err(error) => {
							// SAFETY: We just made this pointer above and `new
							// AudioWorkletNode` has to guarantee that on error transmission
							// failed to avoid double-free.
							let data = unsafe { *Box::from_raw(data.as_ptr()) };
							#[cfg(not(feature = "message"))]
							let data: Task = data;
							#[cfg(feature = "message")]
							let data: Data = data;
							drop(data);

							WORKLET_LOCK.store(0, Ordering::Relaxed);
							// SAFETY: This is safe because `AtomicI32::as_ptr()` returns a valid
							// pointer.
							unsafe {
								wasm32::memory_atomic_notify(WORKLET_LOCK.as_ptr(), u32::MAX)
							};

							return Poll::Ready(Err(super::super::error_from_exception(error)));
						}
					}
				}
				State::Memory {
					ref mut memory_receiver,
					..
				} => match Pin::new(memory_receiver).poll(cx) {
					Poll::Ready(Some(memory)) => {
						let State::Memory {
							thread,
							#[cfg(feature = "message")]
							task,
							#[cfg(feature = "message")]
							node,
							#[cfg(feature = "message")]
							spawn_receiver,
							#[cfg(feature = "message")]
							message,
							..
						} = state
						else {
							unreachable!("found wrong state")
						};

						#[cfg(feature = "message")]
						{
							let node_port = node
								.port()
								.expect("`AudioWorkletNode.port` is not expected to fail");

							let channel = MessageChannel::new()
								.expect("`new MessageChannel` is not expected to fail");
							let port = channel.port1();
							let message_handler =
								super::super::spawn::message::setup_message_handler(
									&port,
									spawn_receiver,
								);
							let task: NonNull<Task> = NonNull::from(Box::leak(Box::new(task)));
							let (serialize, transfer) = match message {
								Some(MessageState {
									serialize,
									transfer: Some(transfer),
								}) => {
									transfer.splice(0, 0, &channel.port2());
									(Array::of2(&task.into(), &serialize), transfer)
								}
								Some(MessageState {
									serialize,
									transfer: None,
								}) => (
									Array::of2(&task.into(), &serialize),
									Array::of1(&channel.port2()),
								),
								None => (Array::of1(&task.into()), Array::of1(&channel.port2())),
							};
							let result =
								node_port.post_message_with_transferable(&serialize, &transfer);

							match result {
								Ok(()) => {
									let previous = WORKLETS.with(|worklets| {
										worklets.borrow_mut().insert(
											thread.id(),
											super::main::State {
												port,
												#[cfg(feature = "message")]
												_message_handler: message_handler,
											},
										)
									});
									debug_assert!(
										previous.is_none(),
										"found previous worker with the same `ThreadId`"
									);
								}
								Err(error) => {
									port.post_message(&JsValue::UNDEFINED).expect(
										"`MessagePort.postMessage()` is not expected to fail \
										 without a `transfer` object",
									);
									// SAFETY: We just made this pointer above and
									// `MessagePort.postMessage()` has to guarantee that on error
									// transmission failed to avoid double-free.
									let task: Task = *unsafe { Box::from_raw(task.as_ptr()) };
									drop(task);
									// SAFETY: We just spawned this audio worklet and
									// `MessagePort.postMessage()` has to guarantee that on error
									// transmission failed so that this worklet will never run
									// anything from this Wasm module.
									unsafe { memory.release() }.expect(
										"found `RegisterThreadFuture` not on the main thread",
									);
									return Poll::Ready(Err(super::super::error_from_exception(
										error,
									)));
								}
							}
						}

						return Poll::Ready(Ok(AudioWorkletHandle { thread, memory }));
					}
					Poll::Pending => {
						self.0 = Some(state);
						return Poll::Pending;
					}
					Poll::Ready(None) => unreachable!("`Sender` dropped somehow"),
				},
			}
		}
	}
}

impl RegisterThreadFuture {
	/// Create a [`RegisterThreadFuture`] that returns `error`.
	pub(in super::super::super) const fn error(error: Error) -> Self {
		Self(Some(State::Error(error)))
	}
}

/// Implementation for [`crate::web::audio_worklet::AudioWorkletHandle`].
#[derive(Debug)]
pub(in super::super::super) struct AudioWorkletHandle {
	/// Corresponding [`Thread`].
	thread: Thread,
	/// Memory handle of the corresponding audio worklet thread.
	memory: ThreadMemory,
}

impl AudioWorkletHandle {
	/// Implementation for
	/// [`crate::web::audio_worklet::AudioWorkletHandle::thread()`].
	pub(crate) const fn thread(&self) -> &Thread {
		&self.thread
	}

	/// Implementation for
	/// [`crate::web::audio_worklet::AudioWorkletHandle::release()`].
	///
	/// # Safety
	///
	/// See [`ThreadMemory::release()`].
	pub(crate) unsafe fn release(self) -> Result<(), Self> {
		// SAFETY: See `ThreadMemory::release()`. Other safety guarantees have to be
		// uphold by the caller.
		let result = unsafe { self.memory.release() };

		match result {
			Ok(()) => {
				#[cfg(feature = "message")]
				super::main::DESTROY_SENDER
					.get()
					.expect("sending `ThreadId` before `DESTROY_SENDER` is initialized")
					.send(self.thread.id())
					.expect("`Receiver` was somehow dropped from the main thread");

				Ok(())
			}
			Err(memory) => Err(Self {
				thread: self.thread,
				memory,
			}),
		}
	}
}

/// Entry function for the worklet.
///
/// # Safety
///
/// `task` has to be a valid pointer to [`Task`].
#[wasm_bindgen(skip_typescript)]
#[allow(unreachable_pub)]
#[cfg_attr(not(feature = "message"), allow(clippy::needless_pass_by_value))]
pub unsafe fn __web_thread_worklet_entry(
	task: NonNull<Task>,
	message: JsValue,
	#[cfg_attr(not(feature = "message"), allow(unused))] port: MessagePort,
) {
	#[cfg(feature = "message")]
	message::MESSAGE_PORT
		.with(|cell| cell.set(port))
		.expect("found existing `MessagePort` in new thread");

	// SAFETY: Has to be a valid pointer to a `Task`. We only call
	// `__web_thread_worklet_entry` from `worklet.js`. The data sent to it comes
	// only from `RegisterThreadFuture::poll()`.
	let task: Task = *unsafe { Box::from_raw(task.as_ptr()) };
	task(message);
}
