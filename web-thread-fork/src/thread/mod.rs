//! Re-implementation of [`std::thread`].

#[cfg(target_feature = "atomics")]
mod atomics;
#[cfg(feature = "audio-worklet")]
pub(crate) mod audio_worklet;
mod builder;
mod global;
mod js;
mod scope;
mod spawn;
#[cfg(not(target_feature = "atomics"))]
mod unsupported;
mod yield_now;

use std::cell::OnceCell;
use std::io::{self, Error, ErrorKind};
use std::num::{NonZeroU64, NonZeroUsize};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use js::{GlobalExt, CROSS_ORIGIN_ISOLATED};
use r#impl::Parker;
use wasm_bindgen::JsCast;

#[cfg(target_feature = "atomics")]
use self::atomics as r#impl;
pub use self::builder::Builder;
use self::global::Global;
pub use self::scope::{scope, Scope, ScopedJoinHandle};
pub(crate) use self::scope::{scope_async, ScopeFuture};
pub use self::spawn::{spawn, JoinHandle};
#[cfg(not(target_feature = "atomics"))]
use self::unsupported as r#impl;
pub use self::yield_now::yield_now;
pub(crate) use self::yield_now::YieldNowFuture;

/// See [`std::thread::Thread`].
#[derive(Clone, Debug)]
pub struct Thread(Pin<Arc<ThreadInner>>);

/// Inner shared wrapper for [`Thread`].
#[derive(Debug)]
struct ThreadInner {
	/// [`ThreadId`].
	id: ThreadId,
	/// Name of the thread.
	name: Option<String>,
	/// Parker implementation.
	parker: Parker,
}

thread_local! {
	/// Holds this threads [`Thread`].
	static THREAD: OnceCell<Thread> = const { OnceCell::new() };
}

impl Thread {
	/// Create a new [`Thread`].
	fn new() -> Self {
		let name = Global::with(|global| match global {
			Global::Dedicated(worker) => Some(worker.name()),
			Global::Shared(worker) => Some(worker.name()),
			Global::Window(_)
			| Global::Service(_)
			| Global::Worklet
			| Global::Worker(_)
			| Global::Unknown => None,
		})
		.filter(|name| !name.is_empty());

		Self::new_with_name(name)
	}

	/// Create a new [`Thread`].
	fn new_with_name(name: Option<String>) -> Self {
		let id = ThreadId::new();
		Self(Arc::pin(ThreadInner {
			id,
			name,
			parker: Parker::new(id),
		}))
	}

	/// See [`std::thread::Thread::id()`].
	#[must_use]
	pub fn id(&self) -> ThreadId {
		self.0.id
	}

	/// See [`std::thread::Thread::name()`].
	#[must_use]
	pub fn name(&self) -> Option<&str> {
		self.0.name.as_deref()
	}

	/// See [`std::thread::Thread::unpark()`].
	#[inline]
	pub fn unpark(&self) {
		Pin::new(&self.0.parker).unpark();
	}
}

/// See [`std::thread::ThreadId`].
#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub struct ThreadId(NonZeroU64);

impl ThreadId {
	/// Create a new [`ThreadId`].
	fn new() -> Self {
		// See <https://github.com/rust-lang/rust/blob/1.75.0/library/std/src/thread/mod.rs#L1177-L1218>.

		/// Separate failed [`ThreadId`] to apply `#[cold]` to it.
		#[cold]
		fn exhausted() -> ! {
			panic!("failed to generate unique thread ID: bitspace exhausted")
		}

		/// Global counter for [`ThreadId`].
		static COUNTER: AtomicU64 = AtomicU64::new(0);

		let mut last = COUNTER.load(Ordering::Relaxed);
		loop {
			let Some(id) = last.checked_add(1) else {
				exhausted();
			};

			match COUNTER.compare_exchange_weak(last, id, Ordering::Relaxed, Ordering::Relaxed) {
				Ok(_) => return Self(NonZeroU64::new(id).expect("unexpected `0` `ThreadId`")),
				Err(id) => last = id,
			}
		}
	}
}

/// See [`std::thread::available_parallelism()`].
///
/// # Notes
///
/// Browsers might return lower values, a common case is to prevent
/// fingerprinting.
///
/// # Errors
///
/// This function will return an error if called from a worklet or any other
/// unsupported thread type.
#[allow(clippy::missing_panics_doc)]
pub fn available_parallelism() -> io::Result<NonZeroUsize> {
	let value = Global::with(|global| match global {
		Global::Window(window) => Ok(window.navigator().hardware_concurrency()),
		Global::Dedicated(worker) => Ok(worker.navigator().hardware_concurrency()),
		Global::Shared(worker) => Ok(worker.navigator().hardware_concurrency()),
		Global::Service(worker) | Global::Worker(worker) => {
			Ok(worker.navigator().hardware_concurrency())
		}
		Global::Worklet => Err(Error::new(
			ErrorKind::Unsupported,
			"operation not supported in worklets",
		)),
		Global::Unknown => Err(Error::new(
			ErrorKind::Unsupported,
			"encountered unsupported thread type",
		)),
	})?;

	#[allow(
		clippy::as_conversions,
		clippy::cast_possible_truncation,
		clippy::cast_sign_loss
	)]
	let value = value as usize;
	let value = NonZeroUsize::new(value)
		.expect("`Navigator.hardwareConcurrency` returned an unexpected value of `0`");

	Ok(value)
}

/// See [`std::thread::current()`].
#[must_use]
pub fn current() -> Thread {
	THREAD.with(|cell| cell.get_or_init(Thread::new).clone())
}

/// See [`std::thread::park()`].
///
/// # Notes
///
/// Unlike [`std::thread::park()`], when using the atomics target feature, this
/// will not panic on the main thread, worklet or any other unsupported thread
/// type. However, on supported thread types, this will function correctly even
/// without the atomics target feature.
///
/// Keep in mind that this call will do nothing unless the calling thread
/// supports blocking, see
/// [`web::has_block_support()`](crate::web::has_block_support).
pub fn park() {
	if has_block_support() {
		Pin::new(&current().0.parker).park();
	}
}

/// See [`std::thread::park_timeout()`].
///
/// # Notes
///
/// Unlike [`std::thread::park_timeout()`], when using the atomics target
/// feature, this will not panic on the main thread, worklet or any other
/// unsupported thread type. However, on supported thread types, this will
/// function correctly even without the atomics target feature.
///
/// Keep in mind that this call will do nothing unless the calling thread
/// supports blocking, see
/// [`web::has_block_support()`](crate::web::has_block_support).
pub fn park_timeout(dur: Duration) {
	if has_block_support() {
		Pin::new(&current().0.parker).park_timeout(dur);
	}
}

/// See [`std::thread::park_timeout_ms()`].
///
/// # Notes
///
/// Unlike [`std::thread::park_timeout_ms()`], when using the atomics target
/// feature, this will not panic on the main thread, worklet or any other
/// unsupported thread type. However, on supported thread types, this will
/// function correctly even without the atomics target feature.
///
/// Keep in mind that this call will do nothing unless the calling thread
/// supports blocking, see
/// [`web::has_block_support()`](crate::web::has_block_support).
#[deprecated(note = "replaced by `web_thread::park_timeout`")]
pub fn park_timeout_ms(ms: u32) {
	park_timeout(Duration::from_millis(ms.into()));
}

/// See [`std::thread::sleep()`].
///
/// # Panics
///
/// This call will panic if the calling thread doesn't support blocking, see
/// [`web::has_block_support()`](crate::web::has_block_support).
pub fn sleep(dur: Duration) {
	if has_block_support() {
		r#impl::sleep(dur);
	} else {
		panic!("current thread type cannot be blocked")
	}
}

/// See [`std::thread::sleep_ms()`].
///
/// # Panics
///
/// This call will panic if the calling thread doesn't support blocking, see
/// [`web::has_block_support()`](crate::web::has_block_support).
#[deprecated(note = "replaced by `web_thread::sleep`")]
pub fn sleep_ms(ms: u32) {
	sleep(Duration::from_millis(ms.into()));
}

/// Implementation for [`crate::web::has_block_support()`].
pub(crate) fn has_block_support() -> bool {
	thread_local! {
		static HAS_BLOCK_SUPPORT: bool = Global::with(|global| {
			match global {
				Global::Window(_) | Global::Worklet | Global::Service(_) => false,
				Global::Dedicated(_) => true,
				// Some browsers don't support blocking in shared workers, so for cross-browser
				// support we have to test manually.
				// See <https://bugzilla.mozilla.org/show_bug.cgi?id=1359745>.
				Global::Shared(_) => {
					/// Cache if blocking on shared workers is supported.
					/// REASON: A Wasm module can never be shared between
					/// multiple shared workers, so this can never be
					/// initialized from multiple threads at the same time.
					#[allow(clippy::disallowed_methods)]
					static HAS_SHARED_WORKER_BLOCK_SUPPORT: OnceLock<bool> = OnceLock::new();

					*HAS_SHARED_WORKER_BLOCK_SUPPORT.get_or_init(r#impl::test_block_support)
				}
				// For unknown worker types we test manually.
				Global::Worker(_) | Global::Unknown => r#impl::test_block_support(),
			}
		});
	}

	HAS_BLOCK_SUPPORT.with(bool::clone)
}

/// Implementation for [`crate::web::has_spawn_support()`].
pub(crate) fn has_spawn_support() -> bool {
	r#impl::has_spawn_support()
}

/// Returns if [`SharedArrayBuffer`][js_sys::SharedArrayBuffer] is supported.
fn has_shared_array_buffer_support() -> bool {
	thread_local! {
		static HAS_SHARED_ARRAY_BUFFER_SUPPORT: bool = {
			let global: GlobalExt = js_sys::global().unchecked_into();

			CROSS_ORIGIN_ISOLATED
				.with(Option::clone)
				.unwrap_or_else(|| !global.shared_array_buffer().is_undefined())
		};
	}

	HAS_SHARED_ARRAY_BUFFER_SUPPORT.with(bool::clone)
}
