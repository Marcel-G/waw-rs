use std::f32::consts::PI;

use waw::{
    buffer::{AudioBuffer, ParamBuffer},
    worklet::{sample_rate, AudioModule, Emitter},
};

#[waw::derive::derive_event]
pub enum OscillatorEvent {
    One(u32),
    Two,
}

#[waw::derive::derive_command]
pub enum OscillatorCommand {
    Count(u32),
}

#[waw::derive::derive_param]
pub enum OscillatorParams {
    #[param(
        automation_rate = "a-rate",
        min_value = 20.0,
        max_value = 20_000.0,
        default_value = 440.
    )]
    Frequency,
}

// Let's implement a simple sine oscillator with variable frequency
// It also accepts commands that send back dummy events for demonstration
pub struct Oscillator {
    phase: u32,
    emitter: Emitter<OscillatorEvent>,
}

impl AudioModule for Oscillator {
    type Event = OscillatorEvent;
    type Command = OscillatorCommand;
    type Param = OscillatorParams;

    fn create(emitter: Emitter<Self::Event>) -> Self {
        Self { phase: 0, emitter }
    }

    fn on_command(&mut self, command: Self::Command) {
        match command {
            OscillatorCommand::Count(number) => {
                self.emitter.send(OscillatorEvent::One(number));
            }
        }
    }

    fn process(&mut self, audio: &mut AudioBuffer, params: &ParamBuffer<Self::Param>) {
        let frequency = params.get(OscillatorParams::Frequency);
        let sr = sample_rate() as f32;

        for (_, output) in audio.zip() {
            // Write to the first output channel
            for (freq, out_sample) in frequency
                .iter()
                .zip(output.channel_mut(0).unwrap().iter_mut())
            {
                let t = self.phase as f32 / sr;
                *out_sample = (t * freq * 2.0 * PI).sin();

                self.phase = (self.phase + 1) % sr as u32;
            }
        }
    }
}

waw::main!(Oscillator);
