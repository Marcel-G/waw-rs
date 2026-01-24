//! Audio worklet support for waw-rs.
//!
//! This module provides the core functionality to register audio worklet
//! threads and processors in WebAssembly.

use std::any::{Any, TypeId};
use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};
use std::future::Future;
use std::io;
use std::marker::PhantomData;
use std::pin::Pin;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicI32, Ordering};
use std::task::{Context, Poll};

use js_sys::{Array, Iterator, JsString, Object, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    AudioContextState, AudioWorkletGlobalScope, AudioWorkletNode, AudioWorkletNodeOptions,
    AudioWorkletProcessor, BaseAudioContext, DomException,
};

use crate::js::{BaseAudioContextExt as BaseAudioContextExtJs, Meta, META};
use crate::script_url::ScriptUrl;

// Worklet script template - @shim.js gets replaced with actual URL
const WORKLET_SCRIPT: &str = include_str!("worklet.min.js");

/// Lock for worklet instantiation.
static WORKLET_LOCK: AtomicI32 = AtomicI32::new(0);

thread_local! {
    /// Cached [`JsValue`] holding index to worklet lock.
    static WORKLET_LOCK_INDEX: JsValue = i32_to_buffer_index(WORKLET_LOCK.as_ptr()).into();

    /// Cached script URL - kept alive for the lifetime of the thread.
    /// This prevents the blob URL from being revoked before the worklet loads it.
    static SCRIPT_URL: std::cell::RefCell<Option<ScriptUrl>> = const { std::cell::RefCell::new(None) };
}

#[wasm_bindgen]
extern "C" {
    /// Name of our custom property on [`AudioWorkletNodeOptions`].
    #[wasm_bindgen(thread_local_v2, static_string)]
    static DATA_PROPERTY_NAME: JsString = "__waw_thread_data";

    /// Name of the processorOptions property.
    #[wasm_bindgen(thread_local_v2, static_string)]
    static PROCESSOR_OPTIONS_PROPERTY_NAME: JsString = "processorOptions";

    /// Extension for processor options to store data pointer.
    #[wasm_bindgen(extends = Object)]
    type ProcessorOptions;

    /// Get the data pointer.
    #[wasm_bindgen(method, getter, js_name = __waw_thread_data)]
    fn data(this: &ProcessorOptions) -> Option<NonNull<Data>>;

    /// Set the data pointer.
    #[wasm_bindgen(method, setter, js_name = __waw_thread_data)]
    fn set_data(this: &ProcessorOptions, value: NonNull<Data>);

    /// Entry function for processor registration.
    #[wasm_bindgen(catch)]
    fn __waw_thread_register_processor(
        name: JsString,
        processor: __WawThreadProcessorConstructor,
    ) -> Result<(), DomException>;
}

/// Extension for [`BaseAudioContext`] to register audio worklet threads.
pub trait BaseAudioContextExt {
    /// Registers a thread at this [`BaseAudioContext`].
    ///
    /// # Arguments
    ///
    /// * `stack_size` - Optional stack size for the thread
    /// * `shim_url` - Optional custom URL for the wasm-bindgen JS shim.
    ///   Use this when bundlers change the location of the JS shim file.
    /// * `f` - The function to run in the audio worklet thread
    fn register_thread<F>(
        self,
        stack_size: Option<usize>,
        shim_url: Option<&str>,
        f: F,
    ) -> RegisterThreadFuture
    where
        F: 'static + FnOnce() + Send;

    /// Creates an [`AudioWorkletNode`] for the given processor.
    fn audio_worklet_node<P>(
        &self,
        name: &str,
        data: P::Data,
        options: Option<&AudioWorkletNodeOptions>,
    ) -> Result<AudioWorkletNode, AudioWorkletNodeError<P>>
    where
        P: 'static + ExtendAudioWorkletProcessor;
}

impl<T> BaseAudioContextExt for T
where
    BaseAudioContext: From<T>,
    T: AsRef<BaseAudioContext>,
{
    fn register_thread<F>(
        self,
        stack_size: Option<usize>,
        shim_url: Option<&str>,
        f: F,
    ) -> RegisterThreadFuture
    where
        F: 'static + FnOnce() + Send,
    {
        register_thread_impl(self.into(), stack_size, shim_url, f)
    }

    fn audio_worklet_node<P>(
        &self,
        name: &str,
        data: P::Data,
        options: Option<&AudioWorkletNodeOptions>,
    ) -> Result<AudioWorkletNode, AudioWorkletNodeError<P>>
    where
        P: 'static + ExtendAudioWorkletProcessor,
    {
        audio_worklet_node_impl::<P>(self.as_ref(), name, data, options)
    }
}

/// Extension for [`AudioWorkletGlobalScope`] to register processors.
pub trait AudioWorkletGlobalScopeExt {
    /// Registers a processor class that extends [`AudioWorkletProcessor`].
    fn register_processor_ext<P>(&self, name: &str) -> Result<(), io::Error>
    where
        P: 'static + ExtendAudioWorkletProcessor;
}

impl AudioWorkletGlobalScopeExt for AudioWorkletGlobalScope {
    fn register_processor_ext<P>(&self, name: &str) -> Result<(), io::Error>
    where
        P: 'static + ExtendAudioWorkletProcessor,
    {
        register_processor::<P>(name)
    }
}

/// Trait for types that extend [`AudioWorkletProcessor`].
pub trait ExtendAudioWorkletProcessor {
    /// Data passed into [`Self::new()`] when creating a node.
    type Data: 'static + Send;

    /// Called when the processor is constructed.
    fn new(
        this: AudioWorkletProcessor,
        data: Option<Self::Data>,
        options: AudioWorkletNodeOptions,
    ) -> Self;

    /// Called for each audio block. Return `true` to keep processing.
    #[allow(unused_variables)]
    fn process(&mut self, inputs: Array, outputs: Array, parameters: Object) -> bool {
        false
    }

    /// Returns the parameter descriptors for this processor.
    #[allow(clippy::must_use_candidate)]
    fn parameter_descriptors() -> Iterator {
        Array::new().values()
    }
}

/// Future returned by [`BaseAudioContextExt::register_thread()`].
#[must_use = "futures do nothing unless polled"]
pub struct RegisterThreadFuture {
    state: Option<RegisterState>,
}

enum RegisterState {
    Error(io::Error),
    WaitingForModule {
        context: BaseAudioContext,
        promise: JsFuture,
        stack_size: Option<usize>,
        task: Box<dyn FnOnce() + Send>,
    },
    WaitingForLock {
        context: BaseAudioContext,
        stack_size: Option<usize>,
        task: Box<dyn FnOnce() + Send>,
    },
}

impl Debug for RegisterThreadFuture {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("RegisterThreadFuture").finish_non_exhaustive()
    }
}

impl Future for RegisterThreadFuture {
    type Output = io::Result<AudioWorkletHandle>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let state = self.state.take().expect("polled after completion");

            match state {
                RegisterState::Error(e) => return Poll::Ready(Err(e)),
                RegisterState::WaitingForModule {
                    context,
                    mut promise,
                    stack_size,
                    task,
                } => match Pin::new(&mut promise).poll(cx) {
                    Poll::Ready(Ok(_)) => {
                        self.state = Some(RegisterState::WaitingForLock {
                            context,
                            stack_size,
                            task,
                        });
                        continue;
                    }
                    Poll::Ready(Err(e)) => {
                        return Poll::Ready(Err(error_from_exception(e)));
                    }
                    Poll::Pending => {
                        self.state = Some(RegisterState::WaitingForModule {
                            context,
                            promise,
                            stack_size,
                            task,
                        });
                        return Poll::Pending;
                    }
                },
                RegisterState::WaitingForLock {
                    context,
                    stack_size,
                    task,
                } => {
                    // Try to acquire lock
                    if WORKLET_LOCK
                        .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
                        .is_err()
                    {
                        // Lock is held, we need to wait
                        // For simplicity, we'll just retry on next poll
                        cx.waker().wake_by_ref();
                        self.state = Some(RegisterState::WaitingForLock {
                            context,
                            stack_size,
                            task,
                        });
                        return Poll::Pending;
                    }

                    // Create the initialization node
                    let task_ptr = Box::into_raw(Box::new(task));
                    #[allow(clippy::as_conversions)]
                    let task_ptr_u32 = task_ptr as u32;

                    let options = AudioWorkletNodeOptions::new();
                    let processor_options = Array::of5(
                        &wasm_bindgen::module(),
                        &wasm_bindgen::memory(),
                        &stack_size.into(),
                        &WORKLET_LOCK_INDEX.with(JsValue::clone),
                        &task_ptr_u32.into(),
                    );
                    options.set_processor_options(Some(&processor_options));

                    match AudioWorkletNode::new_with_options(
                        &context,
                        "__waw_thread_worklet",
                        &options,
                    ) {
                        Ok(_node) => {
                            // Wait for worklet to signal it's ready by setting lock to 0
                            #[cfg(target_feature = "atomics")]
                            {
                                use std::arch::wasm32;
                                while WORKLET_LOCK.load(Ordering::Acquire) != 0 {
                                    // SAFETY: WORKLET_LOCK is a valid i32 atomic
                                    unsafe {
                                        wasm32::memory_atomic_wait32(
                                            WORKLET_LOCK.as_ptr(),
                                            1,
                                            -1,
                                        );
                                    }
                                }
                            }

                            return Poll::Ready(Ok(AudioWorkletHandle { _context: context }));
                        }
                        Err(e) => {
                            // SAFETY: We just created this pointer and it wasn't consumed
                            drop(unsafe { Box::from_raw(task_ptr) });
                            WORKLET_LOCK.store(0, Ordering::Release);
                            return Poll::Ready(Err(error_from_exception(e)));
                        }
                    }
                }
            }
        }
    }
}

/// Handle to a registered audio worklet thread.
#[derive(Debug)]
pub struct AudioWorkletHandle {
    _context: BaseAudioContext,
}

/// Error returned by [`BaseAudioContextExt::audio_worklet_node()`].
pub struct AudioWorkletNodeError<P: ExtendAudioWorkletProcessor> {
    /// The data that was passed in.
    pub data: P::Data,
    /// The error that occurred.
    pub error: io::Error,
}

impl<P: ExtendAudioWorkletProcessor> Debug for AudioWorkletNodeError<P> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("AudioWorkletNodeError")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl<P: ExtendAudioWorkletProcessor> Display for AudioWorkletNodeError<P> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.error, f)
    }
}

impl<P: ExtendAudioWorkletProcessor> Error for AudioWorkletNodeError<P> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.error)
    }
}

// === Implementation details ===

fn register_thread_impl<F>(
    context: BaseAudioContext,
    stack_size: Option<usize>,
    shim_url: Option<&str>,
    task: F,
) -> RegisterThreadFuture
where
    F: 'static + FnOnce() + Send,
{
    if let AudioContextState::Closed = context.state() {
        return RegisterThreadFuture {
            state: Some(RegisterState::Error(io::Error::other(
                "BaseAudioContext is closed",
            ))),
        };
    }

    if let Some(true) = context.unchecked_ref::<BaseAudioContextExtJs>().registered() {
        return RegisterThreadFuture {
            state: Some(RegisterState::Error(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "BaseAudioContext already has a registered thread",
            ))),
        };
    }

    // Build and cache the script URL in thread-local storage.
    // This keeps the blob URL alive for the lifetime of the thread,
    // matching the original wasm-worker implementation.
    //
    // The shim_url is captured on first call; subsequent calls will reuse the cached URL.
    let shim_url_owned = shim_url.map(String::from);
    SCRIPT_URL.with(|cell| {
        let mut url_opt = cell.borrow_mut();
        if url_opt.is_none() {
            let shim_url_string = shim_url_owned
                .as_deref()
                .map(String::from)
                .unwrap_or_else(|| META.with(Meta::url));
            *url_opt = Some(ScriptUrl::new(
                &WORKLET_SCRIPT.replacen("@shim.js", &shim_url_string, 1),
            ));
        }
    });

    let worklet = match context.audio_worklet() {
        Ok(w) => w,
        Err(e) => {
            return RegisterThreadFuture {
                state: Some(RegisterState::Error(error_from_exception(e))),
            }
        }
    };

    // Call add_module inside the .with() callback to ensure the URL stays borrowed
    match SCRIPT_URL.with(|cell| {
        let url_opt = cell.borrow();
        worklet.add_module(url_opt.as_ref().unwrap().as_raw())
    }) {
        Ok(promise) => {
            context
                .unchecked_ref::<BaseAudioContextExtJs>()
                .set_registered(true);

            RegisterThreadFuture {
                state: Some(RegisterState::WaitingForModule {
                    context,
                    promise: JsFuture::from(promise),
                    stack_size,
                    task: Box::new(task),
                }),
            }
        }
        Err(e) => RegisterThreadFuture {
            state: Some(RegisterState::Error(error_from_exception(e))),
        },
    }
}

fn audio_worklet_node_impl<P: 'static + ExtendAudioWorkletProcessor>(
    context: &BaseAudioContext,
    name: &str,
    data: P::Data,
    options: Option<&AudioWorkletNodeOptions>,
) -> Result<AudioWorkletNode, AudioWorkletNodeError<P>> {
    let options: &AudioWorkletNodeOptions = match options {
        Some(o) => o.unchecked_ref(),
        None => &Object::new().unchecked_into(),
    };

    let processor_options = options.get_processor_options();
    let has_processor_options = processor_options.is_some();

    let data = Box::new(Data {
        type_id: TypeId::of::<P>(),
        value: Box::new(data),
        empty: !has_processor_options,
    });

    let processor_options: ProcessorOptions = processor_options.unwrap_or_default().unchecked_into();
    let data_ptr: NonNull<Data> = NonNull::from(Box::leak(data));
    processor_options.set_data(data_ptr);

    if !has_processor_options {
        options.set_processor_options(Some(&processor_options));
    }

    let result = AudioWorkletNode::new_with_options(context, name, options);

    // Clean up the property we added
    if has_processor_options {
        DATA_PROPERTY_NAME
            .with(|name| Reflect::delete_property(&processor_options, name))
            .expect("expected processor_options to be an Object");
    } else {
        PROCESSOR_OPTIONS_PROPERTY_NAME
            .with(|name| Reflect::delete_property(options, name))
            .expect("expected AudioWorkletNodeOptions to be an Object");
    }

    match result {
        Ok(node) => Ok(node),
        Err(error) => {
            // SAFETY: We just created this pointer and it wasn't consumed
            let data = unsafe { Box::from_raw(data_ptr.as_ptr()) };
            Err(AudioWorkletNodeError {
                data: *data.value.downcast().expect("wrong type encoded"),
                error: error_from_exception(error),
            })
        }
    }
}

fn register_processor<P: 'static + ExtendAudioWorkletProcessor>(name: &str) -> Result<(), io::Error> {
    let name = JsString::from_code_point(name.chars().map(u32::from).collect::<Vec<_>>().as_slice())
        .expect("found invalid Unicode");

    __waw_thread_register_processor(
        name,
        __WawThreadProcessorConstructor(Box::new(ProcessorConstructorWrapper::<P>(PhantomData))),
    )
    .map_err(|e| error_from_exception(e.into()))
}

fn error_from_exception(error: JsValue) -> io::Error {
    let error: DomException = error.unchecked_into();
    io::Error::other(format!("{}: {}", error.name(), error.message()))
}

fn i32_to_buffer_index(ptr: *const i32) -> u32 {
    #[allow(clippy::as_conversions)]
    let index = ptr as u32 / 4;
    index
}

/// Data stored in processorOptions to transport processor data.
struct Data {
    type_id: TypeId,
    value: Box<dyn Any>,
    empty: bool,
}

// === Processor registration wasm-bindgen glue ===

#[wasm_bindgen(skip_typescript)]
struct __WawThreadProcessorConstructor(Box<dyn ProcessorConstructor>);

#[wasm_bindgen]
impl __WawThreadProcessorConstructor {
    #[wasm_bindgen]
    #[allow(unreachable_pub)]
    pub fn instantiate(
        &mut self,
        this: AudioWorkletProcessor,
        options: AudioWorkletNodeOptions,
    ) -> __WawThreadProcessor {
        self.0.instantiate(this, options)
    }

    #[wasm_bindgen(js_name = parameterDescriptors)]
    #[allow(unreachable_pub)]
    pub fn parameter_descriptors(&self) -> Iterator {
        self.0.parameter_descriptors()
    }
}

trait ProcessorConstructor {
    fn instantiate(
        &mut self,
        this: AudioWorkletProcessor,
        options: AudioWorkletNodeOptions,
    ) -> __WawThreadProcessor;

    fn parameter_descriptors(&self) -> Iterator;
}

struct ProcessorConstructorWrapper<P: 'static + ExtendAudioWorkletProcessor>(PhantomData<P>);

impl<P: 'static + ExtendAudioWorkletProcessor> ProcessorConstructor
    for ProcessorConstructorWrapper<P>
{
    fn instantiate(
        &mut self,
        this: AudioWorkletProcessor,
        options: AudioWorkletNodeOptions,
    ) -> __WawThreadProcessor {
        let mut processor_data = None;

        if let Some(processor_options) = options.get_processor_options() {
            let processor_options: ProcessorOptions = processor_options.unchecked_into();

            if let Some(data) = processor_options.data() {
                // SAFETY: We only store NonNull<Data> in __waw_thread_data
                let data: Data = *unsafe { Box::from_raw(data.as_ptr()) };

                if data.type_id == TypeId::of::<P>() {
                    processor_data = Some(
                        *data
                            .value
                            .downcast::<P::Data>()
                            .expect("wrong type encoded"),
                    );

                    if data.empty {
                        PROCESSOR_OPTIONS_PROPERTY_NAME
                            .with(|name| Reflect::delete_property(&options, name))
                            .expect("expected AudioWorkletNodeOptions to be an Object");
                    } else {
                        DATA_PROPERTY_NAME
                            .with(|name| Reflect::delete_property(&processor_options, name))
                            .expect("expected processor_options to be an Object");
                    }
                }
            }
        }

        __WawThreadProcessor(Box::new(P::new(this, processor_data, options)))
    }

    fn parameter_descriptors(&self) -> Iterator {
        P::parameter_descriptors()
    }
}

#[wasm_bindgen(skip_typescript)]
struct __WawThreadProcessor(Box<dyn Processor>);

trait Processor {
    fn process(&mut self, inputs: Array, outputs: Array, parameters: Object) -> bool;
}

impl<P: ExtendAudioWorkletProcessor> Processor for P {
    fn process(&mut self, inputs: Array, outputs: Array, parameters: Object) -> bool {
        ExtendAudioWorkletProcessor::process(self, inputs, outputs, parameters)
    }
}

#[wasm_bindgen]
impl __WawThreadProcessor {
    #[wasm_bindgen]
    #[allow(unreachable_pub)]
    pub fn process(&mut self, inputs: Array, outputs: Array, parameters: Object) -> bool {
        self.0.process(inputs, outputs, parameters)
    }
}

/// Entry function called from the worklet script.
///
/// # Safety
///
/// `task_ptr` must be a valid pointer to a `Box<dyn FnOnce() + Send>`.
#[wasm_bindgen]
#[allow(unreachable_pub)]
pub fn __waw_thread_worklet_entry(task_ptr: u32) {
    // SAFETY: Caller guarantees this is a valid pointer created by Box::into_raw
    #[allow(clippy::as_conversions)]
    let task: Box<Box<dyn FnOnce() + Send>> =
        unsafe { Box::from_raw(task_ptr as *mut Box<dyn FnOnce() + Send>) };
    (*task)();
}
