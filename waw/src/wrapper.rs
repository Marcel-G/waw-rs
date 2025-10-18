use crate::{
    buffer::{convert_parameters, InputBuffer, OutputBuffer},
    processor::Processor,
};
use js_sys::{Array, Iterator, Object};
use wasm_bindgen::JsCast;
use web_sys::{AudioWorkletGlobalScope, AudioWorkletNodeOptions, AudioWorkletProcessor};
use web_thread::web::audio_worklet::ExtendAudioWorkletProcessor;

/// A wrapper struct for a type implementing the `Processor` trait, used to interface with the Web Audio API.
pub struct ProcessorWrapper<P: Processor> {
    processor: P,
}

impl<P: Processor> ExtendAudioWorkletProcessor for ProcessorWrapper<P> {
    type Data = P::Data;

    fn new(
        _this: AudioWorkletProcessor,
        data: Option<Self::Data>,
        _options: AudioWorkletNodeOptions,
    ) -> Self {
        let processor = P::new(data.expect("Data required"));
        Self { processor }
    }

    fn process(&mut self, inputs: Array, outputs: Array, parameters: Object) -> bool {
        let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
        let sample_rate = global.sample_rate();

        let input_buffer = InputBuffer::new(&inputs);
        let input_refs = input_buffer.get_refs();
        let mut output_buffer = OutputBuffer::new(&outputs);
        let mut output_refs = output_buffer.get_mut_refs();
        let params = convert_parameters(&parameters);

        self.processor
            .process(&input_refs, &mut output_refs, sample_rate, &params);

        output_buffer.copy_to_js();

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
