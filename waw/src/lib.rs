#![feature(associated_type_defaults)]
#![doc = include_str!("../../README.md")]
#![warn(missing_docs)]

/// Audio buffer structs
pub mod buffer;
/// Audio node bindings (main thread)
pub mod node;
/// Audio worklet bindings (audio thread)
pub mod worklet;
#[doc(hidden)]
pub mod types;
#[doc(hidden)]
pub mod utils;

// Re-export dependencies that are used in macros.
#[doc(hidden)]
pub use enum_map;
#[doc(hidden)]
pub use js_sys;
#[doc(hidden)]
pub use serde;
#[doc(hidden)]
pub use serde_json;
#[doc(hidden)]
pub use tsify;
#[doc(hidden)]
pub use web_sys;

#[doc(hidden)]
pub use waw_macros;

/// Generates the JS bindings
/// 
/// Given a struct implementing `AudioModule`, the [`module!`] will enable wasm-bindgen to
/// generate the neccesary JS to connect the Rust worklet to the WebAudio API.
///
/// ```
/// use waw::{
///   worklet::AudioModule,
///   buffer::{ AudioBuffer, ParamBuffer }
/// };
/// 
/// struct MyWorklet;
/// 
/// impl AudioModule for MyWorklet {
///   fn create() -> Self { MyWorklet }
///   fn process(&mut self, audio: &mut AudioBuffer, params: &ParamBuffer<Self::Param>) {
///     // Implement process
///   }
/// }
/// waw::module!(MyWorklet);
/// ```
pub use waw_macros::module;


