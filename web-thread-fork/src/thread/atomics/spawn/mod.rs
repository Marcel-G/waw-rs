//! Thread spawning implementation.

#[cfg(feature = "message")]
pub(super) mod message;

use std::future::Future;
use std::pin::Pin;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::{io, mem};

use js_sys::Array;
use js_sys::WebAssembly::{Memory, Module};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use web_sys::{Worker, WorkerOptions, WorkerType};
#[cfg(feature = "message")]
use {self::message::SPAWN_SENDER, super::channel};

#[cfg(feature = "audio-worklet")]
use super::audio_worklet::register::THREAD_LOCK_INDEXES;
use super::js::{Meta, META};
use super::main::{self, Command};
use super::memory::ThreadMemory;
use super::url::ScriptUrl;
use super::{oneshot, JoinHandle, ScopeData, Thread, ThreadId, MEMORY, MODULE};
use crate::thread::atomics::main::{State, WORKERS};

/// Type of the task being sent to the worker.
type Task<'scope> =
	Box<dyn 'scope + FnOnce(JsValue) -> Pin<Box<dyn 'scope + Future<Output = u32>>> + Send>;

/// Data to spawn new thread.
pub(super) struct SpawnData {
	/// [`ThreadId`] of the thread to be spawned.
	pub(super) id: ThreadId,
	/// Name of the thread.
	pub(super) name: Option<String>,
	/// Stack size of the thread.
	pub(super) stack_size: Option<usize>,
	/// [`Task`]s with messages to spawn.
	#[cfg(feature = "message")]
	pub(super) spawn_receiver: channel::Receiver<SpawnData>,
	/// Task.
	pub(super) task: Task<'static>,
}

/// Internal spawn function.
///
/// # Safety
///
/// `task` has to outlive the thread.
#[allow(clippy::unnecessary_wraps)]
pub(super) unsafe fn spawn<F1, F2, T>(
	task: F1,
	name: Option<String>,
	stack_size: Option<usize>,
	scope: Option<Arc<ScopeData>>,
) -> io::Result<JoinHandle<T>>
where
	F1: FnOnce() -> F2 + Send,
	F2: Future<Output = T>,
	T: Send,
{
	let thread = thread_init(name, scope.as_deref());
	let (result_sender, result_receiver) = oneshot::channel();
	#[cfg(feature = "message")]
	let (spawn_sender, spawn_receiver) = channel::channel();

	let task: Task<'_> = Box::new({
		let thread = thread.clone();
		move |_| {
			thread_runner(
				thread,
				stack_size,
				result_sender,
				#[cfg(feature = "message")]
				spawn_sender,
				scope,
				task,
			)
		}
	});

	Ok(spawn_without_message(
		thread,
		stack_size,
		result_receiver,
		#[cfg(feature = "message")]
		spawn_receiver,
		task,
	))
}

/// Spawn if no message requires transferring through JS.
fn spawn_without_message<T>(
	thread: Thread,
	stack_size: Option<usize>,
	result_receiver: oneshot::Receiver<T>,
	#[cfg(feature = "message")] spawn_receiver: channel::Receiver<SpawnData>,
	task: Task<'_>,
) -> JoinHandle<T> {
	if super::is_main_thread() {
		main::init_main_thread();

		spawn_internal(
			thread.id(),
			thread.name(),
			stack_size,
			#[cfg(feature = "message")]
			spawn_receiver,
			Box::new(task),
		);
	} else {
		// SAFETY: `task` has to be `'static` or `scope` has to be `Some`, which
		// prevents this thread from outliving its lifetime.
		let task = unsafe { mem::transmute::<Task<'_>, Task<'static>>(task) };

		Command::Spawn(SpawnData {
			id: thread.id(),
			name: thread.0.name.clone(),
			stack_size,
			#[cfg(feature = "message")]
			spawn_receiver,
			task,
		})
		.send();
	}

	JoinHandle {
		receiver: Some(result_receiver),
		thread,
	}
}

/// Common functionality between thread spawning initialization, regardless if a
/// message is passed or not.
fn thread_init(name: Option<String>, scope: Option<&ScopeData>) -> Thread {
	let thread = Thread::new_with_name(name);

	if let Some(scope) = &scope {
		// This can't overflow because creating a `ThreadId` would fail beforehand.
		scope.threads.fetch_add(1, Ordering::Relaxed);
	}

	thread
}

/// Common functionality between threads, regardless if a message is passed.
fn thread_runner<'scope, T: 'scope + Send, F1: 'scope + FnOnce() -> F2, F2: Future<Output = T>>(
	thread: Thread,
	stack_size: Option<usize>,
	result_sender: oneshot::Sender<T>,
	#[cfg(feature = "message")] spawn_sender: channel::Sender<SpawnData>,
	scope: Option<Arc<ScopeData>>,
	task: F1,
) -> Pin<Box<dyn 'scope + Future<Output = u32>>> {
	Box::pin(async move {
		Thread::register(thread);

		#[cfg(feature = "message")]
		{
			let old = SPAWN_SENDER.with(|cell| cell.borrow_mut().replace(spawn_sender));
			debug_assert!(old.is_none(), "found existing `Sender` in new thread");
		}

		result_sender.send(task().await);

		if let Some(scope) = scope {
			if scope.threads.fetch_sub(1, Ordering::Release) == 1 {
				scope.thread.unpark();
				scope.waker.wake();
			}
		}

		#[cfg(feature = "message")]
		SPAWN_SENDER
			.with(|cell| cell.borrow_mut().take())
			.expect("found no `Sender` in existing thread");

		let value = Box::pin(AtomicI32::new(0));
		let index = super::i32_to_buffer_index(value.as_ptr());

		Command::Terminate {
			id: super::current_id(),
			value,
			memory: ThreadMemory::new(stack_size),
		}
		.send();

		index
	})
}

/// Spawning thread regardless of being nested.
pub(super) fn spawn_internal(
	id: ThreadId,
	name: Option<&str>,
	stack_size: Option<usize>,
	#[cfg(feature = "message")] spawn_receiver: channel::Receiver<SpawnData>,
	task: Task<'_>,
) {
	spawn_common(
		id,
		name,
		#[cfg(feature = "message")]
		spawn_receiver,
		task,
		|worker, module, memory, task| {
			#[cfg(not(feature = "audio-worklet"))]
			let message = Array::of4(module, memory, &stack_size.into(), &task);
			#[cfg(feature = "audio-worklet")]
			let message = {
				THREAD_LOCK_INDEXES
					.with(|indexes| Array::of5(module, memory, &stack_size.into(), indexes, &task))
			};
			worker.post_message(&message)
		},
	)
	.expect("`Worker.postMessage()` is not expected to fail without a `transfer` object");
}

/// [`spawn_internal`] regardless if a message is passed or not.
fn spawn_common(
	id: ThreadId,
	name: Option<&str>,
	#[cfg(feature = "message")] spawn_receiver: channel::Receiver<SpawnData>,
	task: Task<'_>,
	post: impl FnOnce(&Worker, &Module, &Memory, JsValue) -> Result<(), JsValue>,
) -> Result<(), JsValue> {
	thread_local! {
		/// Object URL to the worker script.
		static URL: ScriptUrl = {
			#[cfg(not(feature = "audio-worklet"))]
			let template = include_str!("../script/worker.min.js");
			#[cfg(feature = "audio-worklet")]
			let template = include_str!("../script/worker_with_audio_worklet.min.js");

			ScriptUrl::new(&template.replacen("@shim.js", &META.with(Meta::url), 1))
		};
	}

	let options = WorkerOptions::new();
	options.set_type(WorkerType::Module);

	if let Some(name) = name {
		options.set_name(name);
	}

	let worker = URL
		.with(|url| Worker::new_with_options(url.as_raw(), &options))
		.expect("`new Worker()` is not expected to fail with a local script");

	#[cfg(feature = "message")]
	let message_handler = message::setup_message_handler(&worker, spawn_receiver);

	let task: NonNull<Task<'_>> = NonNull::from(Box::leak(Box::new(task)));

	if let Err(err) =
		MODULE.with(|module| MEMORY.with(|memory| post(&worker, module, memory, task.into())))
	{
		// SAFETY: We just made this pointer above and `post` has to guarantee that on
		// error transmission has failed to avoid double-free.
		let task: Task<'_> = *unsafe { Box::from_raw(task.as_ptr()) };
		drop(task);
		worker.terminate();
		return Err(err);
	};

	let previous = WORKERS.with(|workers| {
		workers.borrow_mut().insert(
			id,
			State {
				this: worker,
				#[cfg(feature = "message")]
				_message_handler: message_handler,
			},
		)
	});
	debug_assert!(
		previous.is_none(),
		"found previous worker with the same `ThreadId`"
	);

	Ok(())
}

/// TODO: Remove when `wasm-bindgen` supports `'static` in functions.
type TaskStatic = Task<'static>;

/// Entry function for the worker.
///
/// # Safety
///
/// `task` has to be a valid pointer to [`Task`].
#[wasm_bindgen(skip_typescript)]
#[allow(unreachable_pub)]
pub async unsafe fn __web_thread_worker_entry(task: NonNull<TaskStatic>, message: JsValue) -> u32 {
	// SAFETY: Has to be a valid pointer to a `Task`. We only call
	// `__web_thread_worker_entry` from `worker.js`. The data sent to it comes only
	// from `spawn_internal()`.
	let task: Task<'_> = *unsafe { Box::from_raw(task.as_ptr()) };
	task(message).await
}
