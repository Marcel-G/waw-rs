//! TLS destruction handling.

use js_sys::WebAssembly::Global;
use js_sys::{Number, Object};
use wasm_bindgen::JsCast;

use super::super::ThreadId;
use super::js::{Exports, GlobalDescriptor};

/// Holds pointers to the memory of a thread.
#[derive(Debug)]
pub(super) struct ThreadMemory {
	/// Associated [`ThreadId`].
	thread: ThreadId,
	/// TLS memory.
	tls_base: f64,
	/// Stack memory.
	stack_alloc: f64,
	/// Stack size.
	stack_size: Option<usize>,
}

impl ThreadMemory {
	/// Create new [`ThreadMemory`] for the calling thread.
	pub(super) fn new(stack_size: Option<usize>) -> Self {
		#[cfg(debug_assertions)]
		{
			use std::cell::OnceCell;

			thread_local! {
				static EXISTS: OnceCell<()> = const { OnceCell::new() };
			}

			EXISTS
				.with(|exists| exists.set(()))
				.expect("created `ThreadMemory` twice for this thread");
		}

		let exports: Exports = wasm_bindgen::exports().unchecked_into();
		let tls_base = Number::unchecked_from_js(exports.tls_base().value()).value_of();
		let stack_alloc = Number::unchecked_from_js(exports.stack_alloc().value()).value_of();

		Self {
			thread: super::current_id(),
			tls_base,
			stack_alloc,
			stack_size,
		}
	}

	/// Releases the memory of the referenced thread.
	///
	/// # Safety
	///
	/// The thread is not allowed to be used while or after this function is
	/// executed.
	pub(super) unsafe fn release(self) -> Result<(), Self> {
		thread_local! {
			/// Caches the [`Exports`] object.
			static EXPORTS: Exports = wasm_bindgen::exports().unchecked_into();
			/// Caches the [`GlobalDescriptor`] needed to reconstruct the [`Global`] values.
			static DESCRIPTOR: GlobalDescriptor = {
				let descriptor: GlobalDescriptor = Object::new().unchecked_into();
				descriptor.set_value("i32");
				descriptor
			};
		}

		if self.thread == super::current_id() {
			return Err(self);
		}

		let (tls_base, stack_alloc) = DESCRIPTOR.with(|descriptor| {
			(
				Global::new(descriptor, &self.tls_base.into())
					.expect("unexpected invalid `Global` constructor"),
				Global::new(descriptor, &self.stack_alloc.into())
					.expect("unexpected invalid `Global` constructor"),
			)
		});

		// SAFETY: This is guaranteed to be called only once for the corresponding
		// thread because `Self::new()` prevents two objects to the same thread from
		// being created and `ThreadMemory::release()` consumes itself. Other
		// safety guarantees have to be uphold by the caller.
		EXPORTS.with(|exports| unsafe {
			exports.thread_destroy(&tls_base, &stack_alloc, self.stack_size);
		});

		Ok(())
	}
}
