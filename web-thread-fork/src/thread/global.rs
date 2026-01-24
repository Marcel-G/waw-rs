//! Global context of each worker type.

use wasm_bindgen::JsCast;
use web_sys::{DedicatedWorkerGlobalScope, SharedWorkerGlobalScope, Window, WorkerGlobalScope};

use super::js::{GlobalExt, WindowOrWorkerExt};

thread_local! {
	static GLOBAL: Global = {
		let global: GlobalExt = js_sys::global().unchecked_into();

		if !global.window().is_undefined() {
			Global::Window(global.unchecked_into())
		} else if !global.dedicated_worker_global_scope().is_undefined() {
			Global::Dedicated(global.unchecked_into())
		} else if !global.shared_worker_global_scope().is_undefined() {
			Global::Shared(global.unchecked_into())
		} else if !global.service_worker_global_scope().is_undefined() {
			Global::Service(global.unchecked_into())
		} else if !global.worklet_global_scope().is_undefined() {
			Global::Worklet
		} else if !global.worker_global_scope().is_undefined() {
			Global::Worker(global.unchecked_into())
		} else {
			Global::Unknown
		}
	};
}

/// Global context.
pub(super) enum Global {
	/// [`Window`].
	Window(Window),
	/// [`DedicatedWorkerGlobalScope`].
	Dedicated(DedicatedWorkerGlobalScope),
	/// [`SharedWorkerGlobalScope`].
	Shared(SharedWorkerGlobalScope),
	/// Service worker.
	Service(WorkerGlobalScope),
	/// Unknown worker type.
	Worker(WorkerGlobalScope),
	/// Worklet.
	Worklet,
	/// Unknown.
	Unknown,
}

impl Global {
	/// Executes the given `task` with [`Global`].
	pub(super) fn with<R>(task: impl FnOnce(&Self) -> R) -> R {
		GLOBAL.with(task)
	}

	/// Converts the global type to [`WindowOrWorkerExt`] when appropriate and
	/// executes the given `task` with it.
	pub(super) fn with_window_or_worker<R>(
		task: impl FnOnce(&WindowOrWorkerExt) -> R,
	) -> Option<R> {
		GLOBAL.with(|global| {
			let global: &WindowOrWorkerExt = match global {
				Self::Window(window) => window.unchecked_ref(),
				Self::Dedicated(worker) => worker.unchecked_ref(),
				Self::Service(worker) | Self::Worker(worker) => worker.unchecked_ref(),
				Self::Shared(worker) => worker.unchecked_ref(),
				Self::Worklet | Self::Unknown => return None,
			};

			Some(task(global))
		})
	}
}
