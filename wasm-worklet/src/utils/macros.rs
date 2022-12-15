// @todo -- Find where these should belong, rename this module?

#[macro_export]
macro_rules! derive_event {
    ($i:item) => {
        #[derive(
            wasm_worklet::serde::Serialize,
            wasm_worklet::serde::Deserialize,
            wasm_worklet::tsify::Tsify,
            wasm_worklet::wasm_worklet_macros::RawDescribe,
            Clone,
        )]
        #[tsify(into_wasm_abi, from_wasm_abi)]
        #[serde(crate = "wasm_worklet::serde")]
        $i
    };
}

#[macro_export]
macro_rules! derive_command {
    ($i:item) => {
        #[derive(
            wasm_worklet::serde::Serialize,
            wasm_worklet::serde::Deserialize,
            wasm_worklet::tsify::Tsify,
            Clone,
        )]
        #[tsify(into_wasm_abi, from_wasm_abi)]
        #[serde(crate = "wasm_worklet::serde")]
        $i
    };
}

#[macro_export]
macro_rules! derive_param {
    ($i:item) => {
        #[derive(
            wasm_worklet::serde::Serialize,
            wasm_worklet::serde::Deserialize,
            wasm_worklet::tsify::Tsify,
            wasm_worklet::enum_map::Enum,
            wasm_worklet::wasm_worklet_macros::Param,
            Clone,
            Debug,
        )]
        #[tsify(into_wasm_abi, from_wasm_abi)]
        #[serde(crate = "wasm_worklet::serde")]
        $i
    };
}
