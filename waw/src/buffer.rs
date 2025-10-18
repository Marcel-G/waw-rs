use crate::processor::ParameterValues;
use js_sys::{Array, Float32Array, Object, Reflect};
use std::collections::HashMap;
use wasm_bindgen::JsCast;

/// A buffer that holds input audio data for processing, organized as a vector of channels.
pub struct InputBuffer {
    /// Storage for each channel's audio data.
    pub storage: Vec<Vec<f32>>,
}

impl InputBuffer {
    /// Creates a new `InputBuffer` from a JS Array of input channels.
    pub fn new(inputs: &Array) -> Self {
        let mut storage = Vec::new();
        for i in 0..inputs.length() {
            let channels: Array = inputs.get(i).unchecked_into();

            for j in 0..channels.length() {
                let js_array: Float32Array = channels.get(j).unchecked_into();
                let length = js_array.length() as usize;

                let mut channel_data = vec![0.0f32; length];
                js_array.copy_to(&mut channel_data);

                storage.push(channel_data);
            }
        }

        InputBuffer { storage }
    }

    /// Returns immutable references to each channel's audio data.
    pub fn get_refs(&self) -> Vec<&[f32]> {
        self.storage.iter().map(|v| v.as_slice()).collect()
    }
}

/// A buffer that holds output audio data for processing and manages copying data back to JS.
pub struct OutputBuffer {
    /// JS Float32Array objects for each output channel.
    pub js_arrays: Vec<Float32Array>,
    /// Storage for each channel's audio data.
    pub storage: Vec<Vec<f32>>,
}

impl OutputBuffer {
    /// Creates a new `OutputBuffer` from a JS Array of output channels.
    pub fn new(outputs: &Array) -> Self {
        let mut js_arrays = Vec::new();
        let mut storage = Vec::new();

        for i in 0..outputs.length() {
            let channels: Array = outputs.get(i).unchecked_into();

            for j in 0..channels.length() {
                let js_array: Float32Array = channels.get(j).into();
                let length = js_array.length() as usize;
                let channel_data = vec![0.0f32; length];

                js_arrays.push(js_array);
                storage.push(channel_data);
            }
        }

        Self { js_arrays, storage }
    }

    /// Returns mutable references to all output channels' audio data.
    pub fn get_mut_refs(&mut self) -> Vec<&mut [f32]> {
        self.storage.iter_mut().map(|v| v.as_mut_slice()).collect()
    }

    /// Copies all data from Rust storage back to the corresponding JS Float32Arrays efficiently.
    pub fn copy_to_js(&self) {
        for (js_array, storage) in self.js_arrays.iter().zip(self.storage.iter()) {
            js_array.copy_from(storage);
        }
    }
}

/// Converts a JS parameters Object to a Rust `ParameterValues` struct.
pub fn convert_parameters(params: &Object) -> ParameterValues {
    let mut param_map = HashMap::new();

    let keys = Object::keys(params);
    for i in 0..keys.length() {
        let key_str = keys.get(i).as_string().unwrap_or_default();
        if let Ok(value) = Reflect::get(params, &key_str.clone().into()) {
            if let Some(param_array) = value.dyn_ref::<Float32Array>() {
                if param_array.length() > 0 {
                    param_map.insert(key_str, param_array.get_index(0));
                }
            }
        }
    }

    ParameterValues { params: param_map }
}
