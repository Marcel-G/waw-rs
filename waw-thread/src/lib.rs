//! Minimal audio worklet thread support for waw-rs.
//!
//! This is a simplified fork of web-thread focused solely on audio worklet
//! functionality with support for custom shim URLs for bundler compatibility.

mod audio_worklet;
mod js;
mod script_url;

pub use audio_worklet::{
    AudioWorkletGlobalScopeExt, AudioWorkletHandle, BaseAudioContextExt,
    ExtendAudioWorkletProcessor, RegisterThreadFuture,
};

// Re-export for processor implementations
pub use js_sys;
pub use wasm_bindgen;
pub use web_sys;
