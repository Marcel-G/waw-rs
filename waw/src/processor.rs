use crate::{buffer::ParameterValuesRef, parameter::ParameterDescriptor};

/// The `Processor` trait defines the interface for audio processing units.
pub trait Processor: 'static + Send {
    /// Associated data type for the processor.
    ///
    /// This type represents the configuration or state data required to construct and operate the processor.
    type Data: 'static + Send;

    /// Creates a new instance of the processor with the given data.
    fn new(data: Self::Data) -> Self;

    /// Processes audio buffers.
    ///
    /// # Parameters
    /// - `inputs`: Input audio channels (may be empty for generators)
    /// - `outputs`: Output audio channels to fill
    /// - `sample_rate`: Current audio context sample rate
    /// - `params`: Parameter buffers - use `params.get("name")` to access 128-sample buffers
    fn process(
        &mut self,
        inputs: &[&[f32]],
        outputs: &mut [&mut [f32]],
        sample_rate: f32,
        params: &ParameterValuesRef,
    );

    /// Optional: return parameter descriptors
    fn parameter_descriptors() -> Vec<ParameterDescriptor> {
        Vec::new()
    }
}
