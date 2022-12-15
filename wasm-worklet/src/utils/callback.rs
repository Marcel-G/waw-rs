use std::marker::PhantomData;
use wasm_bindgen::{convert::FromWasmAbi, describe::WasmDescribe, prelude::wasm_bindgen, JsValue};

pub trait RawHackDescribe: WasmDescribe {
    fn len() -> u32;

    // Like WasmDescribe::describe() but without NAMED_EXTERNREF & length chars.
    fn raw_describe();
}

pub struct Callback<T>(pub js_sys::Function, PhantomData<T>);

// @todo move this macro
#[macro_export]
macro_rules! inform_char {
    ($($str:literal,)*) => {
        $(
            inform(($str as char) as u32);
        )*
    };
}

// This is hack to wrap the inner type with `Callback<>` when generating the TS definitions.
// wasm-bindgen does not support his currently
impl<T: RawHackDescribe> WasmDescribe for Callback<T> {
    fn describe() {
        use wasm_bindgen::describe::*;
        inform(NAMED_EXTERNREF);
        inform(T::len() + 10);
        inform_char!('C', 'a', 'l', 'l', 'b', 'a', 'c', 'k', '<',); // Lol
        T::raw_describe();
        inform('>' as u32);
    }
}

#[wasm_bindgen(typescript_custom_section)]
const TS_APPEND_CONTENT: &'static str = r#"
export type Callback<T> = (event: T) => void; 
"#;

impl<T: RawHackDescribe> FromWasmAbi for Callback<T> {
    type Abi = <JsValue as FromWasmAbi>::Abi;

    unsafe fn from_abi(js: Self::Abi) -> Self {
        Callback(js_sys::Function::from_abi(js), PhantomData)
    }
}
