#![feature(associated_type_defaults)]
#![feature(stmt_expr_attributes)]
#![doc = include_str!("../../README.md")]
#![warn(missing_docs)]

/// Audio buffer utilities for input/output and parameter conversion.
pub mod buffer;

/// Macros for processor registration and code generation.
pub mod macros;

/// Parameter types and JS conversion utilities for audio processing.
pub mod parameter;

/// Core audio processor trait and parameter types.
pub mod processor;

/// Processor registration and node creation utilities.
pub mod registry;

/// Wrapper for integrating processors with the Web Audio API.
pub mod wrapper;

pub use parameter::*;
pub use processor::*;
pub use registry::{create_node, register_all};
pub use wrapper::ProcessorWrapper;

// Re-export wasm-bindgen for macros
pub use inventory;
pub use js_sys;
pub use wasm_bindgen;
pub use web_sys;
pub use web_thread;
