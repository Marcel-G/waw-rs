use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::processor::Processor;
use crate::wrapper::ProcessorWrapper;
use wasm_bindgen::prelude::*;
use web_sys::{AudioContext, AudioWorkletNode};

/// Registration entry for inventory
pub struct ProcessorRegistration {
    /// The name of the processor to register
    pub name: &'static str,
    /// The function used to register the processor
    pub register_fn: fn() -> Result<(), JsValue>,
}

impl ProcessorRegistration {
    /// Creates a new `ProcessorRegistration` with the given name and registration function.
    pub const fn new(name: &'static str, register_fn: fn() -> Result<(), JsValue>) -> Self {
        Self { name, register_fn }
    }
}

// Collect all registrations using inventory
inventory::collect!(ProcessorRegistration);

/// Register all processors in the given audio context
pub async fn register_all(ctx: &AudioContext) -> Result<(), JsValue> {
    use web_thread::web::audio_worklet::BaseAudioContextExt;

    let completed = Arc::new(AtomicBool::new(false));
    let completed_clone = completed.clone();

    // Clone registrations for move into closure
    let registrations: Vec<_> = inventory::iter::<ProcessorRegistration>()
        .map(|reg| ProcessorRegistration::new(reg.name, reg.register_fn))
        .collect();

    ctx.clone()
        .register_thread(None, move || {
            for reg in &registrations {
                if let Err(e) = (reg.register_fn)() {
                    web_sys::console::error_1(
                        &format!("Failed to register {}: {:?}", reg.name, e).into(),
                    );
                }
            }

            completed_clone.store(true, Ordering::Release);
        })
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to register thread: {:?}", e)))?;

    while !completed.load(Ordering::Acquire) {
        web_thread::web::yield_now_async(web_thread::web::YieldTime::UserBlocking).await;
    }
    web_thread::web::yield_now_async(web_thread::web::YieldTime::UserBlocking).await;

    Ok(())
}

/// Create an audio worklet node
pub fn create_node<P: Processor>(
    ctx: &AudioContext,
    name: &str,
    data: P::Data,
) -> Result<AudioWorkletNode, JsValue> {
    use web_thread::web::audio_worklet::BaseAudioContextExt;

    ctx.audio_worklet_node::<ProcessorWrapper<P>>(name, data, None)
        .map_err(|e| JsValue::from_str(&format!("Failed to create node: {:?}", e)))
}
