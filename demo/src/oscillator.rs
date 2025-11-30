use wasm_bindgen::prelude::*;
use waw::{register, AutomationRate, ParameterDescriptor, ParameterValuesRef, Processor};

#[derive(Clone)]
pub struct OscillatorData {
    pub frequency: f32,
    pub waveform: Waveform,
}

#[derive(Clone)]
pub enum Waveform {
    Sine,
    Sawtooth,
    Square,
}

pub struct OscillatorProcessor {
    phase: f32,
    frequency: f32,
    waveform: Waveform,
}

impl Processor for OscillatorProcessor {
    type Data = OscillatorData;

    fn new(data: Self::Data) -> Self {
        Self {
            phase: 0.0,
            frequency: data.frequency,
            waveform: data.waveform,
        }
    }

    fn process(
        &mut self,
        _inputs: &[&[f32]],
        outputs: &mut [&mut [f32]],
        sample_rate: f32,
        params: &ParameterValuesRef,
    ) {
        if let Some(output_channel) = outputs.first_mut() {
            // Get frequency parameter buffer (128 samples)
            let freq_buffer = params.get("frequency");

            if let Some(freq) = freq_buffer {
                // Per-sample frequency automation
                for (i, sample) in output_channel.iter_mut().enumerate() {
                    let frequency = freq[i];
                    let phase_increment = frequency / sample_rate;

                    *sample = match self.waveform {
                        Waveform::Sine => (self.phase * 2.0 * std::f32::consts::PI).sin(),
                        Waveform::Sawtooth => 2.0 * (self.phase - (self.phase + 0.5).floor()),
                        Waveform::Square => {
                            if self.phase < 0.5 {
                                1.0
                            } else {
                                -1.0
                            }
                        }
                    } * 0.3; // Reduce volume

                    self.phase += phase_increment;
                    if self.phase >= 1.0 {
                        self.phase -= 1.0;
                    }
                }
            } else {
                // Fallback: use initial frequency
                let phase_increment = self.frequency / sample_rate;

                for sample in output_channel.iter_mut() {
                    *sample = match self.waveform {
                        Waveform::Sine => (self.phase * 2.0 * std::f32::consts::PI).sin(),
                        Waveform::Sawtooth => 2.0 * (self.phase - (self.phase + 0.5).floor()),
                        Waveform::Square => {
                            if self.phase < 0.5 {
                                1.0
                            } else {
                                -1.0
                            }
                        }
                    } * 0.3; // Reduce volume

                    self.phase += phase_increment;
                    if self.phase >= 1.0 {
                        self.phase -= 1.0;
                    }
                }
            }
        }
    }

    fn parameter_descriptors() -> Vec<ParameterDescriptor> {
        vec![ParameterDescriptor {
            name: "frequency".to_string(),
            default_value: 440.0,
            min_value: 20.0,
            max_value: 20000.0,
            automation_rate: AutomationRate::ARate,
        }]
    }
}

#[wasm_bindgen]
pub struct OscillatorNode {
    node: web_sys::AudioWorkletNode,
}

#[wasm_bindgen]
impl OscillatorNode {
    #[wasm_bindgen(constructor)]
    pub fn new(ctx: &web_sys::AudioContext, frequency: f32) -> Result<OscillatorNode, JsValue> {
        let data = OscillatorData {
            frequency,
            waveform: Waveform::Sine,
        };

        // Create options for a generator (0 inputs, 1 output)
        let options = web_sys::AudioWorkletNodeOptions::new();
        options.set_number_of_inputs(0);
        options.set_number_of_outputs(1);

        let node = OscillatorProcessor::create_node(ctx, data, Some(&options))?;
        Ok(OscillatorNode { node })
    }

    #[wasm_bindgen(getter)]
    pub fn node(&self) -> web_sys::AudioWorkletNode {
        self.node.clone()
    }
}

register!(OscillatorProcessor, "oscillator");
