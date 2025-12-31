use crate::{
    buffer::{InputBuffer, OutputBuffer, ParameterBuffer},
    processor::Processor,
};
use js_sys::{Array, Iterator, Object};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use wasm_bindgen::JsCast;
use web_sys::{AudioWorkletGlobalScope, AudioWorkletNodeOptions, AudioWorkletProcessor};
use web_thread::web::audio_worklet::ExtendAudioWorkletProcessor;

/// Internal data structure that wraps user data with lifecycle management.
pub struct ProcessorWrapperData<D> {
    /// The user's processor data
    pub user_data: D,
    /// Shared flag indicating if the processor should continue processing
    pub is_active: Arc<AtomicBool>,
}

/// A wrapper struct for a type implementing the `Processor` trait, used to interface with the Web Audio API.
pub struct ProcessorWrapper<P: Processor> {
    processor: P,
    input_buffer: InputBuffer,
    output_buffer: OutputBuffer,
    parameter_buffer: ParameterBuffer,
    is_active: Arc<AtomicBool>,
}

impl<P: Processor> ExtendAudioWorkletProcessor for ProcessorWrapper<P> {
    type Data = ProcessorWrapperData<P::Data>;

    fn new(
        _this: AudioWorkletProcessor,
        data: Option<Self::Data>,
        options: AudioWorkletNodeOptions,
    ) -> Self {
        let wrapper_data = data.expect("Data required");
        let processor = P::new(wrapper_data.user_data);
        let is_active = wrapper_data.is_active;

        // Initialize with minimal buffers - they will dynamically resize on first process() call
        // based on what JavaScript provides in the inputs/outputs arrays.
        // Web Audio API typically uses 128 samples per render quantum.
        let initial_buffer_size = 128;

        let channel_count = options.get_channel_count().unwrap_or(1);
        let input_count = options.get_number_of_inputs().unwrap_or(0);
        let output_count = options.get_number_of_outputs().unwrap_or(1);

        let input_buffer = InputBuffer::new(
            (input_count * channel_count).try_into().unwrap(),
            initial_buffer_size,
        );

        let output_buffer = OutputBuffer::new(
            (output_count * channel_count).try_into().unwrap(),
            initial_buffer_size,
        );

        let parameter_buffer = ParameterBuffer::new();

        Self {
            processor,
            input_buffer,
            output_buffer,
            parameter_buffer,
            is_active,
        }
    }

    fn process(&mut self, inputs: Array, outputs: Array, parameters: Object) -> bool {
        if !self.is_active.load(Ordering::Acquire) {
            return false;
        }

        let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
        let sample_rate = global.sample_rate();

        // Fill input buffers from JS, handling resizing and zeroing
        self.input_buffer.fill_from_js(&inputs);

        // Ensure output buffer matches the configuration from JS
        self.output_buffer
            .ensure_size(self.input_buffer.buffer_size());
        self.output_buffer.ensure_channels_from_js(&outputs);
        self.output_buffer.clear();

        self.parameter_buffer.fill_from_js(&parameters);

        // Get references for processing
        let input_refs = self.input_buffer.get_refs();
        let mut output_refs = self.output_buffer.get_mut_refs();
        let params = self.parameter_buffer.get_ref();

        // Process audio
        self.processor
            .process(&input_refs, &mut output_refs, sample_rate, &params);

        // Copy output data back to JS
        self.output_buffer.copy_to_js(&outputs);

        true
    }

    fn parameter_descriptors() -> Iterator {
        let arr = Array::new();
        for desc in P::parameter_descriptors() {
            arr.push(&desc.into());
        }
        arr.values()
    }
}
