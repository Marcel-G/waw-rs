use crate::parameter::ParameterDescriptor;
use std::collections::HashMap;

/// The `Processor` trait defines the interface for audio processing units.
pub trait Processor: 'static + Send {
    /// Associated data type for the processor.
    ///
    /// This type represents the configuration or state data required to construct and operate the processor.
    type Data: 'static + Send + Clone;

    /// Creates a new instance of the processor with the given data.
    fn new(data: Self::Data) -> Self;

    /// Processes audio buffers.
    fn process(
        &mut self,
        inputs: &[&[f32]],
        outputs: &mut [&mut [f32]],
        sample_rate: f32,
        params: &ParameterValues,
    );

    /// Optional: return parameter descriptors
    fn parameter_descriptors() -> Vec<ParameterDescriptor> {
        Vec::new()
    }
}

/// Holds the current values of parameters for audio processing.
pub struct ParameterValues {
    pub(crate) params: HashMap<String, f32>,
}

impl ParameterValues {
    /// Returns the value of the parameter with the given name, or the provided default if not found.
    pub fn get(&self, name: &str, default: f32) -> f32 {
        self.params.get(name).copied().unwrap_or(default)
    }

    /// Returns an iterator over the names of all parameters.
    pub fn names(&self) -> impl Iterator<Item = &String> {
        self.params.keys()
    }
}
