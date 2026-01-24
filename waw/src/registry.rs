use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::node::AudioWorkletNodeWrapper;
use crate::processor::Processor;
use crate::wrapper::{ProcessorWrapper, ProcessorWrapperData};
use wasm_bindgen::prelude::*;
use web_sys::AudioContext;

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

/// Register all processors in the given audio context.
///
/// # Arguments
///
/// * `ctx` - The audio context to register processors in
/// * `shim_url` - Optional custom URL for the wasm-bindgen JS shim.
///   Use this when bundlers like Vite change the location of the JS shim file.
///   Pass `None` to use the default `import.meta.url` detection.
///
/// # Example
///
/// ```ignore
/// // Default usage (works in development)
/// waw::register_all(&ctx, None).await?;
///
/// // With custom shim URL (for production builds with bundlers)
/// waw::register_all(&ctx, Some("/assets/my_module.js")).await?;
/// ```
pub async fn register_all(ctx: &AudioContext, shim_url: Option<&str>) -> Result<(), JsValue> {
    use waw_thread::BaseAudioContextExt;

    let completed = Arc::new(AtomicBool::new(false));
    let completed_clone = completed.clone();

    // Clone registrations for move into closure
    let registrations: Vec<_> = inventory::iter::<ProcessorRegistration>()
        .map(|reg| ProcessorRegistration::new(reg.name, reg.register_fn))
        .collect();

    ctx.clone()
        .register_thread(None, shim_url, move || {
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

    // Wait for the registration task to complete
    while !completed.load(Ordering::Acquire) {
        yield_to_event_loop().await;
    }
    // One more yield to ensure everything is synchronized
    yield_to_event_loop().await;

    Ok(())
}

/// Yields to the JavaScript event loop.
async fn yield_to_event_loop() {
    use wasm_bindgen_futures::JsFuture;

    let promise = js_sys::Promise::new(&mut |resolve, _| {
        // Use queueMicrotask for minimal delay
        let global = js_sys::global();
        let queue_microtask = js_sys::Reflect::get(&global, &"queueMicrotask".into())
            .ok()
            .and_then(|f| f.dyn_into::<js_sys::Function>().ok());

        if let Some(queue) = queue_microtask {
            let _ = queue.call1(&JsValue::UNDEFINED, &resolve);
        } else {
            // Fallback: resolve immediately
            let _ = resolve.call0(&JsValue::UNDEFINED);
        }
    });

    let _ = JsFuture::from(promise).await;
}

/// Create an audio worklet node
pub fn create_node<P: Processor>(
    ctx: &AudioContext,
    name: &str,
    data: P::Data,
    options: Option<&web_sys::AudioWorkletNodeOptions>,
) -> Result<AudioWorkletNodeWrapper, JsValue> {
    use waw_thread::BaseAudioContextExt;

    // Create the shared active state flag
    let is_active = Arc::new(AtomicBool::new(true));

    // Wrap the user data with the active state
    let wrapper_data = ProcessorWrapperData {
        user_data: data,
        is_active: is_active.clone(),
    };

    // Create the node
    let node = ctx
        .audio_worklet_node::<ProcessorWrapper<P>>(name, wrapper_data, options)
        .map_err(|e| JsValue::from_str(&format!("Failed to create node: {:?}", e)))?;

    // Return the wrapped node with the shared active state
    Ok(AudioWorkletNodeWrapper::new(node, is_active))
}
