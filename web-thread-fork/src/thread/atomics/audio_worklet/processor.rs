//! [`AudioWorkletProcessor`] related implementation.
//!
//! [`AudioWorkletProcessor`]: https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletProcessor

use std::any::TypeId;
use std::io::Error;
use std::marker::PhantomData;

use js_sys::{Array, Iterator, JsString, Object, Reflect};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsCast;
use web_sys::{AudioWorkletNodeOptions, DomException};

use super::js::ProcessorOptions;
use super::{Data, DATA_PROPERTY_NAME, PROCESSOR_OPTIONS_PROPERTY_NAME};
use crate::web::audio_worklet::ExtendAudioWorkletProcessor;

/// Implementation for
/// [`crate::web::audio_worklet::AudioWorkletGlobalScopeExt::register_processor_ext()`].
pub(in super::super::super) fn register_processor<P: 'static + ExtendAudioWorkletProcessor>(
	name: &str,
) -> Result<(), Error> {
	let name =
		JsString::from_code_point(name.chars().map(u32::from).collect::<Vec<_>>().as_slice())
			.expect("found invalid Unicode");

	__web_thread_register_processor(
		name,
		__WebThreadProcessorConstructor(Box::new(ProcessorConstructorWrapper::<P>(PhantomData))),
	)
	.map_err(|error| super::super::error_from_exception(error.into()))
}

/// Holds the supplied [`ExtendAudioWorkletProcessor`] while type-erasing
/// it.
#[wasm_bindgen(skip_typescript)]
struct __WebThreadProcessorConstructor(Box<dyn ProcessorConstructor>);

#[wasm_bindgen]
impl __WebThreadProcessorConstructor {
	/// Calls the underlying [`ExtendAudioWorkletProcessor::new`].
	#[wasm_bindgen]
	#[allow(unreachable_pub)]
	pub fn instantiate(
		&mut self,
		this: web_sys::AudioWorkletProcessor,
		options: AudioWorkletNodeOptions,
	) -> __WebThreadProcessor {
		self.0.instantiate(this, options)
	}

	/// Calls the underlying
	/// [`ExtendAudioWorkletProcessor::parameter_descriptors`].
	#[wasm_bindgen(js_name = parameterDescriptors)]
	#[allow(unreachable_pub)]
	pub fn parameter_descriptors(&self) -> Iterator {
		self.0.parameter_descriptors()
	}
}

/// Wrapper for the supplied [`ExtendAudioWorkletProcessor`].
struct ProcessorConstructorWrapper<P: 'static + ExtendAudioWorkletProcessor>(PhantomData<P>);

/// Object-safe version of [`ExtendAudioWorkletProcessor`].
trait ProcessorConstructor {
	/// Calls the underlying [`ExtendAudioWorkletProcessor::new`].
	fn instantiate(
		&mut self,
		this: web_sys::AudioWorkletProcessor,
		options: AudioWorkletNodeOptions,
	) -> __WebThreadProcessor;

	/// Calls the underlying
	/// [`ExtendAudioWorkletProcessor::parameter_descriptors`].
	fn parameter_descriptors(&self) -> Iterator;
}

impl<P: 'static + ExtendAudioWorkletProcessor> ProcessorConstructor
	for ProcessorConstructorWrapper<P>
{
	fn instantiate(
		&mut self,
		this: web_sys::AudioWorkletProcessor,
		options: AudioWorkletNodeOptions,
	) -> __WebThreadProcessor {
		let mut processor_data = None;

		if let Some(processor_options) = options.get_processor_options() {
			let processor_options: ProcessorOptions = processor_options.unchecked_into();

			if let Some(data) = processor_options.data() {
				// SAFETY: We only store `NonNull<Data>` in `__web_thread_data` at
				// `super::audio_worklet_node()`.
				let data: Data = *unsafe { Box::<Data>::from_raw(data.as_ptr()) };

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
							.expect("expected `AudioWorkletNodeOptions` to be an `Object`");
					} else {
						DATA_PROPERTY_NAME
							.with(|name| Reflect::delete_property(&processor_options, name))
							.expect("expected `processor_options` to be an `Object`");
					}
				}
			}
		}

		__WebThreadProcessor(Box::new(P::new(this, processor_data, options)))
	}

	fn parameter_descriptors(&self) -> Iterator {
		P::parameter_descriptors()
	}
}

/// Holds the supplied [`ExtendAudioWorkletProcessor`] while type-erasing
/// it.
#[wasm_bindgen(skip_typescript)]
struct __WebThreadProcessor(Box<dyn Processor>);

/// Object-safe version of [`ExtendAudioWorkletProcessor`].
trait Processor {
	/// Calls the underlying [`ExtendAudioWorkletProcessor::process`].
	fn process(&mut self, inputs: Array, outputs: Array, parameters: Object) -> bool;
}

impl<P: ExtendAudioWorkletProcessor> Processor for P {
	fn process(&mut self, inputs: Array, outputs: Array, parameters: Object) -> bool {
		ExtendAudioWorkletProcessor::process(self, inputs, outputs, parameters)
	}
}

#[wasm_bindgen]
impl __WebThreadProcessor {
	/// Calls the underlying [`ExtendAudioWorkletProcessor::new`].
	#[wasm_bindgen]
	#[allow(unreachable_pub)]
	pub fn process(&mut self, inputs: Array, outputs: Array, parameters: Object) -> bool {
		self.0.process(inputs, outputs, parameters)
	}
}

/// Entry function for the worklet.
#[wasm_bindgen]
#[allow(unreachable_pub)]
extern "C" {
	#[wasm_bindgen(catch)]
	fn __web_thread_register_processor(
		name: JsString,
		processor: __WebThreadProcessorConstructor,
	) -> Result<(), DomException>;
}
