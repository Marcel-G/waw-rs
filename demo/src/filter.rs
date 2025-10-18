use wasm_bindgen::prelude::*;
use waw::{create_node, register, AutomationRate, ParameterDescriptor, ParameterValues, Processor};

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
        params: &ParameterValues,
    ) {
        let cutoff = params.get("cutoff", self.cutoff);

        // Simple one-pole low-pass filter
        let omega = 2.0 * std::f32::consts::PI * cutoff / sample_rate;
        let a = omega / (1.0 + omega).min(1.0);

        if let (Some(input_channel), Some(output_channel)) = (inputs.get(0), outputs.get_mut(0)) {
            for (input_sample, output_sample) in input_channel.iter().zip(output_channel.iter_mut())
            {
                self.z1 = input_sample * a + self.z1 * (1.0 - a);
                *output_sample = self.z1;
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
    node: web_sys::AudioWorkletNode,
}

#[wasm_bindgen]
impl FilterNode {
    #[wasm_bindgen(constructor)]
    pub fn new(ctx: &web_sys::AudioContext, cutoff: f32) -> Result<FilterNode, JsValue> {
        let data = FilterData {
            cutoff,
            resonance: 1.0,
        };
        let node = create_node::<FilterProcessor>(ctx, "filter", data)?;
        Ok(FilterNode { node })
    }

    #[wasm_bindgen(getter)]
    pub fn node(&self) -> web_sys::AudioWorkletNode {
        self.node.clone()
    }
}

register!(FilterProcessor, "filter");
