# Test

## Native

cargo test

## Single-Threaded

CHROMEDRIVER=chromedriver RUSTFLAGS=--cfg=web_sys_unstable_apis cargo test --all-features --target wasm32-unknown-unknown
GECKODRIVER=geckodriver RUSTFLAGS="--cfg=web_sys_unstable_apis --cfg=unsupported_service --cfg=unsupported_shared_block" cargo test --all-features --target wasm32-unknown-unknown

## Single-Threaded Doc Tests

CHROMEDRIVER=chromedriver RUSTFLAGS=--cfg=web_sys_unstable_apis RUSTDOCFLAGS=--cfg=web_sys_unstable_apis cargo +nightly test --doc --all-features --target wasm32-unknown-unknown -Zdoctest-xcompile
GECKODRIVER=geckodriver RUSTFLAGS="--cfg=web_sys_unstable_apis --cfg=unsupported_service --cfg=unsupported_shared_block" RUSTDOCFLAGS="--cfg=web_sys_unstable_apis --cfg=unsupported_service --cfg=unsupported_shared_block" cargo +nightly test --doc --all-features --target wasm32-unknown-unknown -Zdoctest-xcompile

## Single-Threaded without Cross-Origin Isolation

WASM_BINDGEN_TEST_NO_ORIGIN_ISOLATION=1 CHROMEDRIVER=chromedriver RUSTFLAGS=--cfg=web_sys_unstable_apis cargo test --all-features --target wasm32-unknown-unknown
WASM_BINDGEN_TEST_NO_ORIGIN_ISOLATION=1 GECKODRIVER=geckodriver RUSTFLAGS="--cfg=web_sys_unstable_apis --cfg=unsupported_service --cfg=unsupported_shared_block" cargo test --all-features --target wasm32-unknown-unknown

## Single-Threaded Doc Tests without Cross-Origin Isolation

WASM_BINDGEN_TEST_NO_ORIGIN_ISOLATION=1 CHROMEDRIVER=chromedriver RUSTFLAGS=--cfg=web_sys_unstable_apis RUSTDOCFLAGS=--cfg=web_sys_unstable_apis cargo +nightly test --doc --all-features --target wasm32-unknown-unknown -Zdoctest-xcompile
WASM_BINDGEN_TEST_NO_ORIGIN_ISOLATION=1 GECKODRIVER=geckodriver RUSTFLAGS="--cfg=web_sys_unstable_apis --cfg=unsupported_service --cfg=unsupported_shared_block" RUSTDOCFLAGS="--cfg=web_sys_unstable_apis --cfg=unsupported_service --cfg=unsupported_shared_block" cargo +nightly test --doc --all-features --target wasm32-unknown-unknown -Zdoctest-xcompile

## Single-Threaded Compile Tests

UI_TEST_TARGET=wasm32-unknown-unknown UI_TEST_ARGS="--features message" cargo test --test compile_test

## Multi-Threaded

CHROMEDRIVER=chromedriver RUSTFLAGS="--cfg=web_sys_unstable_apis --cfg=unsupported_spawn_then_block -Ctarget-feature=+atomics,+bulk-memory" RUSTDOCFLAGS="--cfg=web_sys_unstable_apis --cfg=unsupported_spawn_then_block -Ctarget-feature=+atomics,+bulk-memory" cargo +nightly test --all-features --target wasm32-unknown-unknown -Zdoctest-xcompile -Zbuild-std=panic_abort,std
GECKODRIVER=geckodriver RUSTFLAGS="--cfg=web_sys_unstable_apis --cfg=unsupported_service --cfg=unsupported_shared_block --cfg=unsupported_wait_async -Ctarget-feature=+atomics,+bulk-memory" RUSTDOCFLAGS="--cfg=web_sys_unstable_apis --cfg=unsupported_service --cfg=unsupported_shared_block --cfg=unsupported_wait_async -Ctarget-feature=+atomics,+bulk-memory" cargo +nightly test --all-features --target wasm32-unknown-unknown -Zdoctest-xcompile -Zbuild-std=panic_abort,std

## Multi-Threaded without Cross-Origin Isolation

WASM_BINDGEN_TEST_NO_ORIGIN_ISOLATION=1 CHROMEDRIVER=chromedriver RUSTFLAGS="--cfg=web_sys_unstable_apis --cfg=unsupported_spawn -Ctarget-feature=+atomics,+bulk-memory" RUSTDOCFLAGS="--cfg=web_sys_unstable_apis --cfg=unsupported_spawn -Ctarget-feature=+atomics,+bulk-memory" cargo +nightly test --all-features --target wasm32-unknown-unknown -Zdoctest-xcompile -Zbuild-std=panic_abort,std
WASM_BINDGEN_TEST_NO_ORIGIN_ISOLATION=1 GECKODRIVER=geckodriver RUSTFLAGS="--cfg=web_sys_unstable_apis --cfg=unsupported_spawn --cfg=unsupported_service --cfg=unsupported_shared_block --cfg=unsupported_wait_async -Ctarget-feature=+atomics,+bulk-memory" RUSTDOCFLAGS="--cfg=web_sys_unstable_apis --cfg=unsupported_spawn --cfg=unsupported_service --cfg=unsupported_shared_block --cfg=unsupported_wait_async -Ctarget-feature=+atomics,+bulk-memory" cargo +nightly test --all-features --target wasm32-unknown-unknown -Zdoctest-xcompile -Zbuild-std=panic_abort,std

## Multi-Threaded Compile Tests

UI_TEST_TARGET=wasm32-unknown-unknown UI_TEST_RUSTFLAGS=-Ctarget-feature=+atomics,+bulk-memory UI_TEST_ARGS="--features message" UI_TEST_BUILD_STD=1 cargo +nightly test --test compile_test

# Lint

cargo clippy --all-targets
RUSTFLAGS=--cfg=web_sys_unstable_apis cargo clippy --all-targets --all-features --target wasm32-unknown-unknown
RUSTFLAGS="--cfg=web_sys_unstable_apis -Ctarget-feature=+atomics,+bulk-memory" cargo +nightly clippy --all-targets --all-features --target wasm32-unknown-unknown -Zbuild-std=panic_abort,std

# Doc Lint

cargo doc --no-deps --document-private-items --lib --examples
RUSTDOCFLAGS=--cfg=web_sys_unstable_apis RUSTFLAGS=--cfg=web_sys_unstable_apis cargo doc --no-deps --document-private-items --lib --examples --all-features --target wasm32-unknown-unknown
RUSTDOCFLAGS="--cfg=web_sys_unstable_apis -Ctarget-feature=+atomics,+bulk-memory" RUSTFLAGS="--cfg=web_sys_unstable_apis -Ctarget-feature=+atomics,+bulk-memory" cargo +nightly doc --no-deps --document-private-items --lib --examples --all-features --target wasm32-unknown-unknown -Zbuild-std=panic_abort,std

# docs.rs Lint

RUSTDOCFLAGS=--cfg=docsrs cargo +nightly doc --no-deps --document-private-items --lib --examples
RUSTDOCFLAGS=--cfg=docsrs cargo +nightly doc --no-deps --document-private-items --lib --examples --target wasm32-unknown-unknown
RUSTDOCFLAGS="--cfg=docsrs -Ctarget-feature=+atomics,+bulk-memory" RUSTFLAGS=-Ctarget-feature=+atomics,+bulk-memory cargo +nightly doc --no-deps --document-private-items --lib --examples --target wasm32-unknown-unknown -Zbuild-std=panic_abort,std

# docs.rs Lint with all features

RUSTDOCFLAGS=--cfg=docsrs cargo +nightly doc --no-deps --document-private-items --lib --examples --all-features
RUSTDOCFLAGS="--cfg=web_sys_unstable_apis --cfg=docsrs" RUSTFLAGS=--cfg=web_sys_unstable_apis cargo +nightly doc --no-deps --document-private-items --lib --examples --all-features --target wasm32-unknown-unknown
RUSTDOCFLAGS="--cfg=web_sys_unstable_apis --cfg=docsrs -Ctarget-feature=+atomics,+bulk-memory" RUSTFLAGS="--cfg=web_sys_unstable_apis -Ctarget-feature=+atomics,+bulk-memory" cargo +nightly doc --no-deps --document-private-items --lib --examples --all-features --target wasm32-unknown-unknown -Zbuild-std=panic_abort,std
