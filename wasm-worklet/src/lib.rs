#![feature(associated_type_defaults)]

pub mod buffer;
pub mod node;
pub mod types;
pub mod worklet;

// Re-export dependencies that are used in macros.
pub use enum_map;
pub use js_sys;
pub use serde;
pub use serde_json;
pub use tsify;
pub use web_sys;

pub use wasm_worklet_macros;
pub use wasm_worklet_macros::module;

pub mod utils;
