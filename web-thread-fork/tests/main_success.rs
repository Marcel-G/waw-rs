#[cfg(all(
	target_family = "wasm",
	target_feature = "atomics",
	feature = "audio-worklet",
	feature = "message",
	not(unsupported_spawn)
))]
mod audio_worklet_message_success;
#[cfg(all(
	target_family = "wasm",
	target_feature = "atomics",
	feature = "audio-worklet",
	not(unsupported_spawn)
))]
mod audio_worklet_success;
mod basic_success;
#[cfg(all(
	target_family = "wasm",
	any(not(unsupported_spawn), not(unsupported_wait_async))
))]
mod basic_success_async;
#[cfg(all(
	target_family = "wasm",
	target_feature = "atomics",
	feature = "message",
	not(unsupported_spawn)
))]
mod message_success;
#[cfg(any(
	not(target_family = "wasm"),
	all(
		target_family = "wasm",
		target_feature = "atomics",
		not(unsupported_spawn)
	)
))]
mod supported_spawn_success;
mod test_processor;
mod util;

#[cfg(target_family = "wasm")]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
