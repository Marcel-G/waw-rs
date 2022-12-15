use dasp::{signal, Signal};
use wasm_worklet::types::{AudioModule, ParamMap};

pub struct WorkletB;

impl AudioModule for WorkletB {
    fn create() -> Self {
        WorkletB
    }

    fn process(
        &mut self,
        _input: &[&[[f32; 128]]],
        output: &mut [&mut [[f32; 128]]],
        _params: &ParamMap<Self::Param>,
    ) {
        let mut sig = signal::rate(44100.0).const_hz(440.0).sine();

        if let Some(channels) = output.get_mut(0) {
            for channel in channels.iter_mut() {
                for sample in channel.iter_mut() {
                    *sample = sig.next() as f32
                }
            }
        }
    }
}

wasm_worklet::module!(WorkletB);
