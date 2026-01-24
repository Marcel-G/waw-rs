//! Main thread initialization for audio worklets.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::OnceLock;

use wasm_bindgen::closure::Closure;
use web_sys::{MessageEvent, MessagePort};

use super::super::super::ThreadId;
use super::super::channel::{self, Sender};

/// [`ThreadId`] to destroy [`Sender`] to the main thread.
#[allow(
	clippy::disallowed_methods,
	reason = "this is guaranteed to be initialized from the main thread before any other thread \
	          will try to access it"
)]
pub(super) static DESTROY_SENDER: OnceLock<Sender<ThreadId>> = OnceLock::new();

thread_local! {
	/// Containing all spawned audio worklets.
	pub(super) static WORKLETS: RefCell<HashMap<ThreadId, State>> = RefCell::new(HashMap::new());
}

/// State for each audio worklet.
pub(super) struct State {
	/// [`MessagePort`]
	pub(super) port: MessagePort,
	/// Callback handling messages.
	pub(super) _message_handler: Closure<dyn Fn(MessageEvent)>,
}

/// Initializes the main thread worklet handler. Make sure to call this at
/// least once on the main thread before spawning any audio worklet.
///
/// # Panics
///
/// This will panic if called outside the main thread.
pub(in super::super) fn init_main_thread() {
	debug_assert!(
		super::super::is_main_thread(),
		"initizalizing main thread without being on the main thread"
	);

	DESTROY_SENDER.get_or_init(|| {
		let (sender, receiver) = channel::channel();

		wasm_bindgen_futures::spawn_local(async move {
			while let Ok(id) = receiver.next().await {
				let state = WORKLETS.with(|worklets| {
					worklets
						.borrow_mut()
						.remove(&id)
						.expect("audio worklet to be terminated not found")
				});
				state.port.set_onmessage(None);
			}
		});

		sender
	});
}
