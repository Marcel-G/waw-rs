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
pub struct Oscillator {
    accumulator: f32,
    emitter: Emitter<OscillatorEvent>,
}

impl AudioModule for Oscillator {
    type Event = OscillatorEvent;
    type Command = OscillatorCommand;
    type Param = OscillatorParams;

    fn create(emitter: Emitter<Self::Event>) -> Self {
        Self {
            accumulator: 0.0,
            emitter,
        }
    }

    fn on_command(&mut self, command: Self::Command) {
        match command {
            OscillatorCommand::Count(number) => {
                self.emitter.send(OscillatorEvent::One(number));
            }
        }
    }

    fn process(&mut self, audio: &mut AudioBuffer, params: &ParamBuffer<Self::Param>) {
        // @todo -- the pitch seems to randomly drift...
        let frequency = params.get(OscillatorParams::Frequency);
        let sr = sample_rate() as f32;

        for (_, output) in audio.zip() {
            // For each input sample and output sample in buffer
            for out_channel in output.channel_iter_mut() {
                for (freq, out_sample) in frequency.iter().zip(out_channel.iter_mut()) {
                    self.accumulator += freq / sr;
                    *out_sample = self.accumulator.sin();
                }
            }
        }
    }
}

waw::main!(Oscillator);
