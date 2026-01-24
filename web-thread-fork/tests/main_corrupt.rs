#![cfg(test)]
#![cfg(all(
	target_family = "wasm",
	target_feature = "atomics",
	not(unsupported_spawn)
))]

mod supported_spawn_corrupt;
#[cfg(feature = "audio-worklet")]
mod test_processor;
mod util;

#[cfg(feature = "audio-worklet")]
use web_sys::BaseAudioContext;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[cfg(feature = "audio-worklet")]
async fn test_stack_size(context: BaseAudioContext) {
	use std::hint;

	use futures_util::future;
	use futures_util::future::Either;
	use web_thread::web::audio_worklet::BaseAudioContextExt;

	use self::util::{Flag, SIGNAL_DURATION};

	#[inline(never)]
	#[allow(clippy::large_stack_frames, clippy::missing_const_for_fn)]
	fn allocate_on_stack() {
		#[allow(clippy::large_stack_arrays)]
		hint::black_box([0_u8; 1024 * 1024 * 2]);
	}

	let flag = Flag::new();

	context.register_thread(Some(1024 * 64), {
		let flag = flag.clone();
		move || {
			allocate_on_stack();
			flag.signal();
		}
	});

	assert!(matches!(
		future::select(flag, util::sleep(SIGNAL_DURATION)).await,
		Either::Right(_)
	));
}

#[cfg(feature = "audio-worklet")]
test_audio!(stack_size);
