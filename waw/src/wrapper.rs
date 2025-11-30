use crate::{
    buffer::{InputBuffer, OutputBuffer, ParameterBuffer},
    processor::Processor,
};
use js_sys::{Array, Iterator, Object};
use wasm_bindgen::JsCast;
use web_sys::{AudioWorkletGlobalScope, AudioWorkletNodeOptions, AudioWorkletProcessor};
use web_thread::web::audio_worklet::ExtendAudioWorkletProcessor;

/// A wrapper struct for a type implementing the `Processor` trait, used to interface with the Web Audio API.
pub struct ProcessorWrapper<P: Processor> {
    processor: P,
    input_buffer: InputBuffer,
    output_buffer: OutputBuffer,
    parameter_buffer: ParameterBuffer,
}

impl<P: Processor> ExtendAudioWorkletProcessor for ProcessorWrapper<P> {
    type Data = P::Data;

    fn new(
        _this: AudioWorkletProcessor,
        data: Option<Self::Data>,
        options: AudioWorkletNodeOptions,
    ) -> Self {
        let processor = P::new(data.expect("Data required"));

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
        }
    }

    fn process(&mut self, inputs: Array, outputs: Array, parameters: Object) -> bool {
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
