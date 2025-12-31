use wasm_bindgen::prelude::*;
use waw::{register, AutomationRate, ParameterDescriptor, ParameterValuesRef, Processor};

#[derive(Clone)]
pub struct FilterData {
    pub cutoff: f32,
    pub resonance: f32,
}

pub struct FilterProcessor {
    cutoff: f32,
    z1: f32,
}

impl Processor for FilterProcessor {
    type Data = FilterData;

    fn new(data: Self::Data) -> Self {
        Self {
            cutoff: data.cutoff,
            z1: 0.0,
        }
    }

    fn process(
        &mut self,
        inputs: &[&[f32]],
        outputs: &mut [&mut [f32]],
        sample_rate: f32,
        params: &ParameterValuesRef,
    ) {
        if let (Some(input_channel), Some(output_channel)) = (inputs.first(), outputs.first_mut()) {
            // Get cutoff parameter buffer (128 samples)
            // For k-rate: all values are the same
            // For a-rate: values may differ for per-sample automation
            let cutoff_buffer = params.get("cutoff");

            if let Some(cutoff) = cutoff_buffer {
                // Simple one-pole low-pass filter with per-sample automation
                for (i, (input_sample, output_sample)) in input_channel
                    .iter()
                    .zip(output_channel.iter_mut())
                    .enumerate()
                {
                    let cutoff_value = cutoff[i];
                    let omega = 2.0 * std::f32::consts::PI * cutoff_value / sample_rate;
                    let a = omega / (1.0 + omega).min(1.0);

                    self.z1 = input_sample * a + self.z1 * (1.0 - a);
                    *output_sample = self.z1;
                }
            } else {
                // Fallback: use initial cutoff value
                let omega = 2.0 * std::f32::consts::PI * self.cutoff / sample_rate;
                let a = omega / (1.0 + omega).min(1.0);

                for (input_sample, output_sample) in
                    input_channel.iter().zip(output_channel.iter_mut())
                {
                    self.z1 = input_sample * a + self.z1 * (1.0 - a);
                    *output_sample = self.z1;
                }
            }
        }
    }

    fn parameter_descriptors() -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor {
                name: "cutoff".to_string(),
                default_value: 1000.0,
                min_value: 20.0,
                max_value: 20000.0,
                automation_rate: AutomationRate::ARate,
            },
            ParameterDescriptor {
                name: "resonance".to_string(),
                default_value: 1.0,
                min_value: 0.1,
                max_value: 30.0,
                automation_rate: AutomationRate::KRate,
            },
        ]
    }
}

#[wasm_bindgen]
pub struct FilterNode {
    wrapper: waw::AudioWorkletNodeWrapper,
}

#[wasm_bindgen]
impl FilterNode {
    #[wasm_bindgen(constructor)]
    pub fn new(ctx: &web_sys::AudioContext, cutoff: f32) -> Result<FilterNode, JsValue> {
        let data = FilterData {
            cutoff,
            resonance: 1.0,
        };

        // Create options for an effect (1 input, 1 output)
        let options = web_sys::AudioWorkletNodeOptions::new();
        options.set_number_of_inputs(1);
        options.set_number_of_outputs(1);

        let wrapper = FilterProcessor::create_node(ctx, data, Some(&options))?;
        Ok(FilterNode { wrapper })
    }

    #[wasm_bindgen(getter)]
    pub fn node(&self) -> web_sys::AudioWorkletNode {
        self.wrapper.node().clone()
    }
}

register!(FilterProcessor, "filter");
