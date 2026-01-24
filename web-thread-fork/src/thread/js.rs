//! Bindings to the JS API.

use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use web_sys::Window;
#[cfg(web_sys_unstable_apis)]
pub(super) use web_sys::{Scheduler, SchedulerPostTaskOptions, TaskPriority};
#[cfg(not(web_sys_unstable_apis))]
use {
	js_sys::{Function, Promise},
	web_sys::AbortSignal,
};

#[wasm_bindgen]
extern "C" {
	/// Extension for the [global object](https://developer.mozilla.org/en-US/docs/Glossary/Global_object).
	pub(super) type GlobalExt;

	/// Returns the constructor of [`Window`](https://developer.mozilla.org/en-US/docs/Web/API/Window).
	#[wasm_bindgen(method, getter, js_name = Window)]
	pub(super) fn window(this: &GlobalExt) -> JsValue;

	/// Returns the constructor of [`DedicatedWorkerGlobalScope`](https://developer.mozilla.org/en-US/docs/Web/API/DedicatedWorkerGlobalScope).
	#[wasm_bindgen(method, getter, js_name = DedicatedWorkerGlobalScope)]
	pub(super) fn dedicated_worker_global_scope(this: &GlobalExt) -> JsValue;

	/// Returns the constructor of [`SharedWorkerGlobalScope`](https://developer.mozilla.org/en-US/docs/Web/API/SharedWorkerGlobalScope).
	#[wasm_bindgen(method, getter, js_name = SharedWorkerGlobalScope)]
	pub(super) fn shared_worker_global_scope(this: &GlobalExt) -> JsValue;

	/// Returns the constructor of [`ServiceWorkerGlobalScope`](https://developer.mozilla.org/en-US/docs/Web/API/ServiceWorkerGlobalScope).
	#[wasm_bindgen(method, getter, js_name = ServiceWorkerGlobalScope)]
	pub(super) fn service_worker_global_scope(this: &GlobalExt) -> JsValue;

	/// Returns the constructor of [`WorkerGlobalScope`](https://developer.mozilla.org/en-US/docs/Web/API/WorkerGlobalScope).
	#[wasm_bindgen(method, getter, js_name = WorkerGlobalScope)]
	pub(super) fn worker_global_scope(this: &GlobalExt) -> JsValue;

	/// Returns the constructor of [`WorkletGlobalScope`](https://developer.mozilla.org/en-US/docs/Web/API/WorkletGlobalScope).
	#[wasm_bindgen(method, getter, js_name = WorkletGlobalScope)]
	pub(super) fn worklet_global_scope(this: &GlobalExt) -> JsValue;

	/// Returns the constructor of [`SharedArrayBuffer`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/SharedArrayBuffer).
	#[wasm_bindgen(method, getter, js_name = SharedArrayBuffer)]
	pub(super) fn shared_array_buffer(this: &GlobalExt) -> JsValue;

	/// Extension for [`Window`](https://developer.mozilla.org/en-US/docs/Web/API/Window).
	#[wasm_bindgen(extends = Window)]
	pub(super) type WindowExt;

	/// Returns the [`Window.requestIdleCallback`](https://developer.mozilla.org/en-US/docs/Web/API/Window/requestIdleCallback) method.
	#[wasm_bindgen(method, getter, js_name = requestIdleCallback)]
	pub(super) fn has_request_idle_callback(this: &WindowExt) -> JsValue;

	/// Extension for [`Window`] or [`WorkerGlobalScope`].
	///
	/// [`Window`]: https://developer.mozilla.org/en-US/docs/Web/API/Window
	/// [`WorkerGlobalScope`]: https://developer.mozilla.org/en-US/docs/Web/API/WorkerGlobalScope
	#[wasm_bindgen]
	pub(super) type WindowOrWorkerExt;

	/// Returns the [`Scheduler`](https://developer.mozilla.org/en-US/docs/Web/API/Scheduler) object.
	#[wasm_bindgen(method, getter, js_name = scheduler)]
	pub(super) fn has_scheduler(this: &WindowOrWorkerExt) -> JsValue;

	/// Returns the [`Scheduler`](https://developer.mozilla.org/en-US/docs/Web/API/Scheduler) object.
	#[wasm_bindgen(method, getter)]
	pub(super) fn scheduler(this: &WindowOrWorkerExt) -> Scheduler;

	/// Returns [`crossOriginIsolated`](https://developer.mozilla.org/en-US/docs/Web/API/crossOriginIsolated) global property.
	#[wasm_bindgen(thread_local, js_name = crossOriginIsolated)]
	pub(super) static CROSS_ORIGIN_ISOLATED: Option<bool>;
}

#[cfg(not(web_sys_unstable_apis))]
#[wasm_bindgen]
extern "C" {
	/// [`Scheduler`](https://developer.mozilla.org/en-US/docs/Web/API/Scheduler) interface.
	pub(super) type Scheduler;

	/// Binding to [`Scheduler.postTask`](https://developer.mozilla.org/en-US/docs/Web/API/Scheduler/postTask).
	#[wasm_bindgen(method, js_name = postTask)]
	pub(super) fn post_task_with_options(
		this: &Scheduler,
		callback: &Function,
		options: &SchedulerPostTaskOptions,
	) -> Promise;

	/// Dictionary type of [`SchedulerPostTaskOptions`](https://developer.mozilla.org/en-US/docs/Web/API/Scheduler/postTask#options).
	pub(super) type SchedulerPostTaskOptions;

	/// Setter for [`SchedulerPostTaskOptions.signal`](https://developer.mozilla.org/en-US/docs/Web/API/Scheduler/postTask#signal) property.
	#[wasm_bindgen(method, setter, js_name = signal)]
	pub(super) fn set_signal(this: &SchedulerPostTaskOptions, signal: &AbortSignal);

	/// Setter for [`SchedulerPostTaskOptions.priority`](https://developer.mozilla.org/en-US/docs/Web/API/Scheduler/postTask#priority) property.
	#[wasm_bindgen(method, setter, js_name = priority)]
	pub(super) fn set_priority(this: &SchedulerPostTaskOptions, priority: TaskPriority);
}

/// Dictionary type of [`TaskPriority`](https://developer.mozilla.org/en-US/docs/Web/API/Scheduler/postTask#priority).
#[cfg(not(web_sys_unstable_apis))]
#[wasm_bindgen]
pub(super) enum TaskPriority {
	UserBlocking = "user-blocking",
	UserVisible = "user-visible",
	Background = "background",
}
