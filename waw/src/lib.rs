#![feature(associated_type_defaults)]
#![feature(stmt_expr_attributes)]
#![doc = include_str!("../../README.md")]
#![warn(missing_docs)]

/// Audio buffer utilities for input/output and parameter conversion.
pub mod buffer;

/// Macros for processor registration and code generation.
pub mod macros;

/// Node wrapper for proper cleanup and lifecycle management.
pub mod node;

/// Parameter types and JS conversion utilities for audio processing.
pub mod parameter;

/// Core audio processor trait and parameter types.
pub mod processor;

/// Processor registration and node creation utilities.
pub mod registry;

/// Wrapper for integrating processors with the Web Audio API.
pub mod wrapper;

pub use buffer::ParameterValuesRef;
pub use node::AudioWorkletNodeWrapper;
pub use parameter::*;
pub use processor::*;
pub use registry::{create_node, register_all};
pub use wrapper::{ProcessorWrapper, ProcessorWrapperData};

// Re-export dependencies for macros and processor implementations
pub use inventory;
pub use js_sys;
pub use wasm_bindgen;
pub use waw_thread;
pub use web_sys;
