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
///
/// Uses `MessageChannel.postMessage()` which properly yields to the event loop
/// without the minimum delay of `setTimeout`. Falls back to immediate resolution
/// if `MessageChannel` is not available (e.g., in worklet contexts).
///
/// Based on the approach from wasm-worker/web-thread.
async fn yield_to_event_loop() {
    use std::cell::RefCell;
    use std::future::Future;
    use std::pin::Pin;
    use std::rc::Rc;
    use std::task::{Context, Poll, Waker};
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;
    use web_sys::MessageChannel;

    /// Shared state for the yield future.
    struct YieldState {
        completed: bool,
        waker: Option<Waker>,
    }

    /// Future that yields to the event loop via MessageChannel.
    struct YieldFuture {
        state: Rc<RefCell<YieldState>>,
        _callback: Option<Closure<dyn FnMut()>>,
        _channel: Option<MessageChannel>,
    }

    impl Future for YieldFuture {
        type Output = ();

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let mut state = self.state.borrow_mut();
            if state.completed {
                Poll::Ready(())
            } else {
                state.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }

    // Try to create a MessageChannel for yielding
    let channel = MessageChannel::new().ok();

    if let Some(channel) = channel {
        let state = Rc::new(RefCell::new(YieldState {
            completed: false,
            waker: None,
        }));

        let callback = {
            let state = Rc::clone(&state);
            Closure::once(move || {
                let mut s = state.borrow_mut();
                s.completed = true;
                if let Some(waker) = s.waker.take() {
                    waker.wake();
                }
            })
        };

        channel
            .port1()
            .set_onmessage(Some(callback.as_ref().unchecked_ref()));
        let _ = channel.port2().post_message(&JsValue::UNDEFINED);

        let future = YieldFuture {
            state,
            _callback: Some(callback),
            _channel: Some(channel),
        };

        future.await;
    }
    // If MessageChannel is not available (e.g., in worklet), resolve immediately
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
