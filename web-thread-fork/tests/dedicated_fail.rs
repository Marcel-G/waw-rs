#![cfg(target_family = "wasm")]

mod basic_fail;
#[cfg(any(
	not(target_feature = "atomics"),
	not(unsupported_spawn),
	not(unsupported_wait_async)
))]
mod basic_fail_async;
#[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
mod supported_spawn_fail;
#[cfg(any(not(target_feature = "atomics"), unsupported_spawn))]
mod unsupported_spawn;
mod util;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_dedicated_worker);
