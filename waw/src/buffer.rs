use js_sys::{Array, Float32Array, Object, Reflect};
use std::collections::HashMap;
use wasm_bindgen::JsCast;

/// A generic multi-channel buffer for audio data.
/// Provides common functionality for managing channel storage and interacting with JS Float32Arrays.
pub struct ChannelBuffer {
    /// Storage for each channel's audio data.
    storage: Vec<Vec<f32>>,
    /// Cached buffer size (number of samples per channel)
    buffer_size: usize,
}

impl ChannelBuffer {
    /// Creates a new `ChannelBuffer` with the specified number of channels and initial buffer size.
    pub fn new(num_channels: usize, buffer_size: usize) -> Self {
        let storage = vec![vec![0.0f32; buffer_size]; num_channels];
        ChannelBuffer {
            storage,
            buffer_size,
        }
    }

    /// Ensures all channel buffers match the expected size, resizing if necessary.
    pub fn ensure_size(&mut self, buffer_size: usize) {
        if buffer_size != self.buffer_size {
            for channel in &mut self.storage {
                channel.resize(buffer_size, 0.0);
            }
            self.buffer_size = buffer_size;
        }
    }

    /// Ensures the buffer has at least the specified number of channels, adding new channels if necessary.
    pub fn ensure_channels(&mut self, num_channels: usize) {
        while self.storage.len() < num_channels {
            self.storage.push(vec![0.0; self.buffer_size]);
        }
    }

    /// Zeros out all channel buffers.
    pub fn clear(&mut self) {
        for channel in &mut self.storage {
            channel.fill(0.0);
        }
    }

    /// Returns the current buffer size.
    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }

    /// Returns the number of channels.
    pub fn num_channels(&self) -> usize {
        self.storage.len()
    }

    /// Returns immutable references to each channel's audio data.
    pub fn get_refs(&self) -> Vec<&[f32]> {
        self.storage.iter().map(|v| v.as_slice()).collect()
    }

    /// Returns mutable references to each channel's audio data.
    pub fn get_mut_refs(&mut self) -> Vec<&mut [f32]> {
        self.storage.iter_mut().map(|v| v.as_mut_slice()).collect()
    }

    /// Copies data from a JS Array to the buffer.
    /// Automatically detects and adjusts to the buffer size and channel count from JS.
    /// Zeros out buffers first, then copies available data.
    pub fn copy_from_js(&mut self, js_array: &Array) {
        // Count total number of channels in the JS array
        let mut total_channels = 0;
        for i in 0..js_array.length() {
            let channels: Array = js_array.get(i).unchecked_into();
            total_channels += channels.length() as usize;
        }

        // Determine the actual buffer size from the first channel
        let actual_buffer_size = if js_array.length() > 0 {
            let channels: Array = js_array.get(0).unchecked_into();
            if channels.length() > 0 {
                let float_array: Float32Array = channels.get(0).unchecked_into();
                float_array.length() as usize
            } else {
                128
            }
        } else {
            128
        };

        // Ensure we have enough channels and the right buffer size
        self.ensure_size(actual_buffer_size);
        self.ensure_channels(total_channels);

        // Zero out all buffers first
        self.clear();

        // Copy data from JS to our buffers
        let mut channel_idx = 0;
        for i in 0..js_array.length() {
            let channels: Array = js_array.get(i).unchecked_into();
            for j in 0..channels.length() {
                if channel_idx < self.storage.len() {
                    let float_array: Float32Array = channels.get(j).unchecked_into();
                    let length = float_array.length() as usize;

                    let copy_len = length.min(actual_buffer_size);
                    float_array.copy_to(&mut self.storage[channel_idx][..copy_len]);

                    // If JS provided less data than expected, the rest is already zeroed
                    if length < actual_buffer_size {
                        self.storage[channel_idx][length..].fill(0.0);
                    }

                    channel_idx += 1;
                }
            }
        }
    }

    /// Copies data from the buffer to a JS Array
    pub fn copy_to_js(&self, js_array: &Array) {
        let mut channel_idx = 0;
        for i in 0..js_array.length() {
            let channels: Array = js_array.get(i).unchecked_into();
            for j in 0..channels.length() {
                if channel_idx < self.storage.len() {
                    let float_array: Float32Array = channels.get(j).into();

                    float_array.copy_from(&self.storage[channel_idx]);

                    channel_idx += 1;
                }
            }
        }
    }
}

/// Copies data from a JS Float32Array to a Rust Vec<f32> buffer.
/// Handles Web Audio API parameter buffer semantics.
fn copy_param_from_js(js_array: &Float32Array, buffer: &mut Vec<f32>) {
    // Ensure buffer is sized to 128 samples (Web Audio render quantum size)
    buffer.resize(128, 0.0);

    match js_array.length() {
        // If the automation rate of the parameter is "a-rate", the array will contain 128 values
        // — one for each frame in the current audio block.
        128 => {
            js_array.copy_to(buffer.as_mut());
        }

        // If the automation rate is "k-rate", the array will contain a single value,
        // which is to be used for each of 128 frames.
        //
        // If there's no automation happening during the time represented by the current block,
        // the array may contain a single value that is constant for the entire block,
        // instead of 128 identical values.
        1 => {
            buffer.fill(js_array.get_index(0));
        }

        // Other possibilities are not supported.
        other => {
            panic!(
                "Float32Array length {other} not supported. Expected 1 (k-rate) or 128 (a-rate)."
            );
        }
    }
}

/// A buffer that holds input audio data for processing, organized as a vector of channels.
pub struct InputBuffer {
    inner: ChannelBuffer,
}

impl InputBuffer {
    /// Creates a new `InputBuffer` with the specified number of channels and initial buffer size.
    pub fn new(num_channels: usize, buffer_size: usize) -> Self {
        InputBuffer {
            inner: ChannelBuffer::new(num_channels, buffer_size),
        }
    }

    /// Fills the buffer with data from a JS Array of input channels.
    /// Zeros out buffers first, then copies available data.
    /// If JS provides less data than expected, remaining space stays zeroed.
    pub fn fill_from_js(&mut self, inputs: &Array) {
        self.inner.copy_from_js(inputs);
    }

    /// Returns immutable references to each channel's audio data.
    pub fn get_refs(&self) -> Vec<&[f32]> {
        self.inner.get_refs()
    }

    /// Returns the current buffer size.
    pub fn buffer_size(&self) -> usize {
        self.inner.buffer_size()
    }
}

/// A buffer that holds output audio data for processing and manages copying data back to JS.
pub struct OutputBuffer {
    inner: ChannelBuffer,
}

impl OutputBuffer {
    /// Creates a new `OutputBuffer` with the specified number of channels and initial buffer size.
    pub fn new(num_channels: usize, buffer_size: usize) -> Self {
        OutputBuffer {
            inner: ChannelBuffer::new(num_channels, buffer_size),
        }
    }

    /// Ensures buffers match the expected size, resizing if necessary.
    pub fn ensure_size(&mut self, buffer_size: usize) {
        self.inner.ensure_size(buffer_size);
    }

    /// Ensures the buffer has the right number of channels based on what JS provides in the outputs array.
    pub fn ensure_channels_from_js(&mut self, outputs: &Array) {
        let mut total_channels = 0;
        for i in 0..outputs.length() {
            let channels: Array = outputs.get(i).unchecked_into();
            total_channels += channels.length() as usize;
        }

        self.inner.ensure_channels(total_channels);
    }

    /// Zeros out all output buffers.
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Returns mutable references to all output channels' audio data.
    pub fn get_mut_refs(&mut self) -> Vec<&mut [f32]> {
        self.inner.get_mut_refs()
    }

    /// Copies all data from Rust storage back to the corresponding JS Float32Arrays.
    pub fn copy_to_js(&self, outputs: &Array) {
        self.inner.copy_to_js(outputs);
    }
}

/// A buffer that holds parameter values for audio processing.
/// Each parameter has a buffer of 128 samples (one render quantum).
/// For a-rate parameters, all 128 values may be different.
/// For k-rate parameters, all 128 values will be the same.
pub struct ParameterBuffer {
    /// Storage for parameter buffers. Each parameter gets a Vec<f32> with 128 samples.
    params: HashMap<String, Vec<f32>>,
    /// Buffer size (typically 128 for Web Audio API)
    buffer_size: usize,
}

impl ParameterBuffer {
    /// Creates a new empty `ParameterBuffer`.
    pub fn new() -> Self {
        ParameterBuffer {
            params: HashMap::new(),
            buffer_size: 128,
        }
    }

    /// Fills the buffer with parameter values from a JS Object.
    /// Handles both a-rate (128 values) and k-rate (1 value) parameters.
    ///
    /// According to Web Audio API spec:
    /// - If automation rate is "a-rate", array contains 128 values (one per frame)
    /// - If no automation, array may contain 1 value that's constant for entire block
    /// - If automation rate is "k-rate", array contains 1 value for all 128 frames
    pub fn fill_from_js(&mut self, params: &Object) {
        // Clear existing parameter data
        for buffer in self.params.values_mut() {
            buffer.clear();
        }

        let keys = Object::keys(params);
        for i in 0..keys.length() {
            let key_str = keys.get(i).as_string().unwrap_or_default();
            if let Ok(value) = Reflect::get(params, &key_str.clone().into()) {
                if let Some(param_array) = value.dyn_ref::<Float32Array>() {
                    // Get or create the buffer for this parameter
                    let buffer = self
                        .params
                        .entry(key_str)
                        .or_insert_with(|| Vec::with_capacity(self.buffer_size));

                    // Copy parameter data from JS
                    copy_param_from_js(param_array, buffer);
                }
            }
        }
    }

    /// Returns a reference to the parameter values without cloning.
    /// This is more efficient than cloning and the returned reference
    /// provides access to the full parameter buffers.
    pub fn get_ref(&self) -> ParameterValuesRef<'_> {
        ParameterValuesRef {
            params: &self.params,
        }
    }
}

impl Default for ParameterBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// A reference to parameter values without ownership.
/// Provides access to parameter buffers (128 samples per parameter).
pub struct ParameterValuesRef<'a> {
    params: &'a HashMap<String, Vec<f32>>,
}

impl<'a> ParameterValuesRef<'a> {
    /// Returns a reference to the parameter buffer (128 samples) with the given name.
    /// Returns None if the parameter is not found.
    ///
    /// Each parameter buffer contains 128 samples:
    /// - For k-rate parameters: all 128 values are identical
    /// - For a-rate parameters: each value may be different (automation)
    ///
    /// # Example
    /// ```ignore
    /// // Access parameter buffer
    /// if let Some(cutoff) = params.get("cutoff") {
    ///     for (i, sample) in output.iter_mut().enumerate() {
    ///         // Use cutoff[i] for per-sample automation
    ///         process_sample(sample, cutoff[i]);
    ///     }
    /// }
    /// ```
    pub fn get(&self, name: &str) -> Option<&[f32]> {
        self.params.get(name).map(|v| v.as_slice())
    }
}
