use futures::{channel::oneshot, lock::Mutex};
use js_sys::{ArrayBuffer, Reflect};
use lazy_static::lazy_static;
use std::{collections::HashSet, marker::PhantomData, mem::forget};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    AddEventListenerOptions, AudioContext, AudioParam, AudioWorkletNode, AudioWorkletNodeOptions,
    MessageEvent, MessagePort, Response,
};

use crate::{
    types::{AudioModuleDescriptor, InternalMessage},
    utils::{environment::assert_main, import_meta},
    worklet::{register_processor, register_wasm, AudioModule},
};

use wasm_bindgen::{prelude::*, JsCast};

// Keep track of the registered AudioWorklets. This is to avoid building Blobs & registering worklets multiple times.
// Also, the WASM module only needs to be compiled and loaded once in the AudioWorkletGlobalScope. This must happen for the first worklet only.
lazy_static! {
    static ref REGISTERED_WORKLETS: Mutex<HashSet<String>> = Mutex::new(Default::default());
}

/// Wrapper struct for the audio worklet node.
///
/// This struct is automatically generated when using the [`main!`]. It should not be used directly.
pub struct Node<Module: AudioModule + AudioModuleDescriptor> {
    /// The inner AudioWorkletNode
    pub inner: AudioWorkletNode,
    _phantom: PhantomData<Module>,
}

impl<Module: AudioModule + AudioModuleDescriptor> Node<Module> {
    /// Initialises a new audio worklet.
    pub async fn install(ctx: AudioContext) -> Result<Node<Module>, JsValue> {
        assert_main();
        // Register the worklet processor
        // Note: it's possible to add serveral modules with `add_module` by calling `registerProcessor` several times.
        //       However, it's not trivial in rust to collect all the audio modules defined by the consumer in order to generate that js at once.
        let mut registered_worklets = REGISTERED_WORKLETS.lock().await;
        let is_first = registered_worklets.is_empty();

        if is_first {
            // Before registering any worklets, load the wasm JS module into the AudioWorkletGlobalScope
            // This is necessary so that the Firefox addModule polyfill only needs to transpile once
            // rather than for each worklet.
            let wasm_loader_blob = register_wasm()?;
            JsFuture::from(ctx.audio_worklet()?.add_module(&wasm_loader_blob)?).await?;
        }

        if !registered_worklets.contains(Module::processor_name()) {
            let worklet_blob_url = register_processor::<Module>()?;
            JsFuture::from(ctx.audio_worklet()?.add_module(&worklet_blob_url)?).await?;
            registered_worklets.insert(Module::processor_name().to_string());
        };

        let mut options = AudioWorkletNodeOptions::new();

        options
            .number_of_inputs(Module::INPUTS)
            .number_of_outputs(Module::OUTPUTS);

        if is_first {
            // Get the URL to this .wasm file
            let url = import_meta::url_wasm();

            // Fetching the wasm module has to take place on the main thread (rather than from within the worklet).
            // This is because the AudioWorkletGlobalScope does not support fetch API.
            let window = web_sys::window().unwrap();
            let resp_value = JsFuture::from(window.fetch_with_str(&url)).await?;
            assert!(resp_value.is_instance_of::<Response>());
            let resp: Response = resp_value.dyn_into().unwrap();
            let wasm_source = JsFuture::from(resp.array_buffer()?)
                .await?
                .unchecked_into::<ArrayBuffer>();

            // Compiling the wasm module should ideally be done here on the main thread. (as is done here https://github.com/rustwasm/wasm-bindgen/tree/main/examples/wasm-audio-worklet)
            // However, Safari does not support transferring `WebAssembly.Module` or `WebAssembly.Memory` over to the worklet. https://bugs.webkit.org/show_bug.cgi?id=220038
            // Next best is to send the wasm source code to the worklet and compile it within the worklet.
            // All modules are in a single `.wasm` file since it's not currently possible yet to split it up into smaller files.
            options.processor_options(Some(&js_sys::Array::of1(&wasm_source)));
        }

        // Initialise the worklet node
        let node = AudioWorkletNode::new_with_options(&ctx, Module::processor_name(), &options)?;

        // Wait for the worklet processor to finish compiling.
        wait_for_any_response(node.port()?).await?;

        Ok(Node {
            inner: node,
            _phantom: PhantomData,
        })
    }

    /// Returns a given AudioParam.
    pub fn get_param(&self, param: Module::Param) -> AudioParam {
        Reflect::get(
            &js_sys::Object::from_entries(&self.inner.parameters().unwrap()).unwrap(),
            &format!("{param:?}").into(),
        )
        .unwrap()
        .unchecked_into()
    }

    /// Sends a command to the audio worklet processor.
    pub fn command(&self, message: Module::Command) {
        self.inner
            .port()
            .unwrap()
            .post_message(&message.into())
            .unwrap();
    }

    /// Sets up a subscription to events emitted from the audio worklet processor.
    pub fn subscribe(&mut self, callback: js_sys::Function) {
        let outer_callback = Closure::wrap(Box::new(move |event: MessageEvent| {
            callback.call1(&JsValue::NULL, &event.data()).unwrap();
        }) as Box<dyn Fn(MessageEvent)>);

        self.inner
            .port()
            .unwrap()
            .add_event_listener_with_callback("message", outer_callback.as_ref().unchecked_ref())
            .unwrap();

        forget(outer_callback); // @todo -- remove_event_listener on drop
    }

    /// Destroys the audio worklet processor.
    pub fn destroy(&mut self) {
        // Send message into worklet to destroy it
        self.inner
            .port()
            .unwrap()
            .post_message(&serde_wasm_bindgen::to_value(&InternalMessage::Destroy).unwrap())
            .unwrap();
    }
}

async fn wait_for_any_response(port: MessagePort) -> Result<(), JsValue> {
    let (sender, receiver) = oneshot::channel();

    let callback = Closure::once(Box::new(move |_| {
        sender.send(()).unwrap();
    }) as Box<dyn FnOnce(MessageEvent)>);

    port.add_event_listener_with_callback_and_add_event_listener_options(
        "message",
        callback.as_ref().unchecked_ref(),
        AddEventListenerOptions::new().once(true),
    )?;
    port.start();

    receiver.await.map_err(|_| JsValue::from_str("Cancelled"))?;

    Ok(())
}
