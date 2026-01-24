use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomPinned;
use std::panic::{RefUnwindSafe, UnwindSafe};

use static_assertions::{assert_impl_all, assert_not_impl_any};
#[cfg(target_family = "wasm")]
use wasm_bindgen_test::wasm_bindgen_test;
use web_thread::{Builder, JoinHandle, Scope, ScopedJoinHandle, Thread, ThreadId};

#[cfg(target_family = "wasm")]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[cfg_attr(not(target_family = "wasm"), test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
const fn basic() {
	assert_impl_all!(Builder: Debug, Send, Sync, Unpin, RefUnwindSafe, UnwindSafe);
	assert_not_impl_any!(Builder: Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd);

	assert_impl_all!(JoinHandle<PhantomPinned>: Debug, Send, Sync, Unpin);
	assert_not_impl_any!(JoinHandle<PhantomPinned>: Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd, RefUnwindSafe, UnwindSafe);

	assert_impl_all!(Scope<'_, '_>: Debug, Send, Sync, Unpin, RefUnwindSafe);
	assert_not_impl_any!(Scope<'_, '_>: Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd, UnwindSafe);

	assert_impl_all!(ScopedJoinHandle<'_, PhantomPinned>: Debug, Send, Sync, Unpin);
	assert_not_impl_any!(ScopedJoinHandle<'_, PhantomPinned>: Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd, RefUnwindSafe, UnwindSafe);

	assert_impl_all!(Thread: Clone, Debug, Send, Sync, Unpin, RefUnwindSafe, UnwindSafe);
	assert_not_impl_any!(Thread: Copy, Hash, Eq, PartialEq, Ord, PartialOrd);

	assert_impl_all!(ThreadId: Clone, Copy, Debug, Hash, Eq, PartialEq, Send, Sync, Unpin, RefUnwindSafe, UnwindSafe);
	assert_not_impl_any!(ThreadId: Ord, PartialOrd);
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen_test]
const fn web() {
	use static_assertions::assert_obj_safe;
	use web_thread::web::{
		JoinHandleExt, JoinHandleFuture, ScopeFuture, ScopeIntoJoinFuture, ScopeJoinFuture,
		ScopedJoinHandleExt, ScopedJoinHandleFuture, YieldNowFuture, YieldTime,
	};

	assert_impl_all!(JoinHandleFuture<'_, PhantomPinned>: Debug, Send, Sync, Unpin);
	assert_not_impl_any!(JoinHandleFuture<'_, PhantomPinned>: Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd, RefUnwindSafe, UnwindSafe);

	assert_impl_all!(ScopedJoinHandleFuture<'_, '_, PhantomPinned>: Debug, Send, Sync, Unpin);
	assert_not_impl_any!(ScopedJoinHandleFuture<'_, '_, PhantomPinned>: Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd, RefUnwindSafe, UnwindSafe);

	assert_impl_all!(ScopeFuture<'_, '_, PhantomPinned, PhantomPinned>: Debug, Send, Sync, RefUnwindSafe);
	assert_impl_all!(ScopeFuture<'_, '_, (), PhantomPinned>: Unpin);
	assert_not_impl_any!(ScopeFuture<'_, '_, PhantomPinned, PhantomPinned>: Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd, UnwindSafe);

	assert_impl_all!(ScopeIntoJoinFuture<'_, '_, PhantomPinned, PhantomPinned>: Debug, Send, Sync, RefUnwindSafe);
	assert_impl_all!(ScopeIntoJoinFuture<'_, '_, (), PhantomPinned>: Unpin);
	assert_not_impl_any!(ScopeIntoJoinFuture<'_, '_, PhantomPinned, PhantomPinned>: Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd, UnwindSafe);

	assert_impl_all!(ScopeJoinFuture<'_, '_, PhantomPinned>: Debug, Send, Sync, Unpin, RefUnwindSafe);
	assert_not_impl_any!(ScopeJoinFuture<'_, '_, PhantomPinned>: Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd, UnwindSafe);

	assert_impl_all!(YieldNowFuture: Debug, Unpin, RefUnwindSafe);
	assert_not_impl_any!(YieldNowFuture: Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd, Send, Sync, UnwindSafe);

	assert_impl_all!(YieldTime: Clone, Copy, Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Send, Sync, Unpin, RefUnwindSafe, UnwindSafe);

	assert_obj_safe!(JoinHandleExt<()>, ScopedJoinHandleExt<'_, ()>);

	#[cfg(feature = "audio-worklet")]
	{
		use std::error::Error;
		use std::fmt::Display;

		use web_sys::{AudioWorkletNodeOptions, AudioWorkletProcessor};
		use web_thread::web::audio_worklet::{
			AudioWorkletHandle, AudioWorkletNodeError, ExtendAudioWorkletProcessor,
			RegisterThreadFuture, ReleaseError,
		};

		#[allow(dead_code)]
		struct TestProcessor;

		impl ExtendAudioWorkletProcessor for TestProcessor {
			type Data = ();

			fn new(
				_: AudioWorkletProcessor,
				_: Option<Self::Data>,
				_: AudioWorkletNodeOptions,
			) -> Self {
				Self
			}
		}

		assert_impl_all!(RegisterThreadFuture: Debug, Unpin, RefUnwindSafe);
		assert_not_impl_any!(RegisterThreadFuture: Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd, Send, Sync, UnwindSafe);

		assert_impl_all!(AudioWorkletHandle: Debug, Send, Sync, Unpin, RefUnwindSafe, UnwindSafe);
		assert_not_impl_any!(AudioWorkletHandle: Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd);

		assert_impl_all!(AudioWorkletNodeError<TestProcessor>: Debug, Display, Error, Send, Sync, Unpin);
		assert_not_impl_any!(AudioWorkletNodeError<TestProcessor>: Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd, RefUnwindSafe, UnwindSafe);

		assert_impl_all!(ReleaseError: Debug, Display, Error, Send, Sync, Unpin, RefUnwindSafe, UnwindSafe);
		assert_not_impl_any!(ReleaseError: Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd);
	}
}
