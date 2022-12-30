/// Adds necessary implementations for event enums
///
/// Use the macro to create an event enum:
///
/// ```
/// waw::derive_event! {
///    pub enum MyEvent {
///        One(u32),
///        Two,
///    }
/// }
/// ```
#[macro_export]
macro_rules! derive_event {
    ($i:item) => {
        #[automatically_derived]
        mod derived_event {
            use waw::tsify as tsify;

            #[derive(
                waw::serde::Serialize,
                waw::serde::Deserialize,
                waw::tsify::Tsify,
                waw::waw_macros::RawDescribe,
                Clone,
            )]
            #[tsify(into_wasm_abi, from_wasm_abi)]
            #[serde(crate = "waw::serde")]
            $i
        }
        use derived_event::*;
    };
}

/// Adds necessary implementations for command enums
///
/// Use the macro to create a command enum:
/// ```
/// waw::derive_command! {
///     pub enum MyCommand {
///         Count(u32),
///     }
/// }
/// ```
#[macro_export]
macro_rules! derive_command {
    ($i:item) => {
        #[automatically_derived]
        mod derived_command {
            use waw::tsify as tsify;

            #[derive(waw::serde::Serialize, waw::serde::Deserialize, waw::tsify::Tsify, Clone)]
            #[tsify(into_wasm_abi, from_wasm_abi)]
            #[serde(crate = "waw::serde")]
            $i
        }
        use derived_command::*;
    };
}

/// Adds necessary implementations for parameter enums
///
/// Use the macro to create a param enum:
///
/// ```
/// waw::derive_param! {
///    pub enum MyParams {
///        Frequency,
///    }
/// }
/// ```
/// > Note: Due to an issue <https://gitlab.com/KonradBorowski/enum-map/-/issues/22> with enum-map re-exports you'll need to add add it as a dependency directly.
/// > ```toml
/// > [dependencies]
/// > enum-map = "2.4.1"
/// > ```
///
/// Parameter options can be applied using the `#[param()]` annotation
///
/// ```
/// waw::derive_param! {
///    pub enum MyParams {
///        #[param(
///            automation_rate = "a-rate",
///            min_value = 20.0,
///            max_value = 20_000.0,
///            default_value = 440.
///        )]
///        Frequency,
///    }
/// }
/// ```
#[macro_export]
macro_rules! derive_param {
    ($i:item) => {
        #[automatically_derived]
        mod derived_param {
            use waw::tsify as tsify;
            use waw::enum_map as enum_map;

            #[derive(
                waw::serde::Serialize,
                waw::serde::Deserialize,
                waw::tsify::Tsify,
                waw::enum_map::Enum,
                waw::waw_macros::Param,
                Clone,
                Debug,
            )]
            #[tsify(into_wasm_abi, from_wasm_abi)]
            #[serde(crate = "waw::serde")]
            $i
        }
        use derived_param::*;
    };
}
