//! Main thread initialization and command handling.

use std::cell::RefCell;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::atomic::AtomicI32;
use std::sync::OnceLock;

use web_sys::Worker;
#[cfg(feature = "message")]
use {wasm_bindgen::closure::Closure, web_sys::MessageEvent};

use super::super::ThreadId;
use super::channel::{self, Sender};
use super::memory::ThreadMemory;
use super::spawn::{self, SpawnData};
use super::wait_async::WaitAsync;

/// [`Command`] [`Sender`] to the main thread.
#[allow(
	clippy::disallowed_methods,
	reason = "this is guaranteed to be initialized from the main thread before any other thread \
	          will try to access it"
)]
static COMMAND_SENDER: OnceLock<Sender<Command>> = OnceLock::new();

thread_local! {
	/// Containing all spawned workers.
	pub(super) static WORKERS: RefCell<HashMap<ThreadId, State>> = RefCell::new(HashMap::new());
}

/// State for each [`Worker`].
pub(super) struct State {
	/// [`Worker`]
	pub(super) this: Worker,
	/// Callback handling messages.
	#[cfg(feature = "message")]
	pub(super) _message_handler: Closure<dyn Fn(MessageEvent)>,
}

/// Command sent to the main thread.
pub(super) enum Command {
	/// Spawn a new thread.
	Spawn(SpawnData),
	/// Terminate thread.
	Terminate {
		/// [`ThreadId`] of the thread to be terminated.
		id: ThreadId,
		/// Value to use `Atomics.waitAsync` on.
		value: Pin<Box<AtomicI32>>,
		/// Handle to release thread memory.
		memory: ThreadMemory,
	},
}

impl Command {
	/// Sends command to be executed on the main thread.
	pub(super) fn send(self) {
		COMMAND_SENDER
			.get()
			.expect("sending `Command` before `COMMAND_SENDER` is initialized")
			.send(self)
			.expect("`Receiver` was somehow dropped from the main thread");
	}
}

/// Initializes the main thread worker handler. Make sure to call this at
/// least once on the main thread before spawning any thread.
///
/// # Panics
///
/// This will panic if called outside the main thread.
pub(super) fn init_main_thread() {
	debug_assert!(
		super::is_main_thread(),
		"initizalizing main thread without being on the main thread"
	);

	COMMAND_SENDER.get_or_init(|| {
		super::has_spawn_support();

		let (sender, receiver) = channel::channel::<Command>();

		wasm_bindgen_futures::spawn_local(async move {
			while let Ok(command) = receiver.next().await {
				match command {
					Command::Spawn(SpawnData {
						id,
						name,
						stack_size,
						#[cfg(feature = "message")]
						spawn_receiver,
						task,
					}) => {
						spawn::spawn_internal(
							id,
							name.as_deref(),
							stack_size,
							#[cfg(feature = "message")]
							spawn_receiver,
							Box::new(task),
						);
					}
					Command::Terminate { id, value, memory } => {
						wasm_bindgen_futures::spawn_local(async move {
							WaitAsync::wait(&value, 0).await;

							// SAFETY: We wait until the execution block has exited and block the
							// thread afterwards.
							unsafe { memory.release() }.expect("attempted to clean up main thread");

							let state = WORKERS.with(|workers| {
								workers
									.borrow_mut()
									.remove(&id)
									.expect("`Worker` to be terminated not found")
							});
							state.this.terminate();
							#[cfg(feature = "message")]
							state.this.set_onmessage(None);
						});
					}
				}
			}
		});

		#[cfg(all(feature = "audio-worklet", feature = "message"))]
		super::audio_worklet::main::init_main_thread();

		sender
	});
}
