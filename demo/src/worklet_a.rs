use dasp::{signal, Signal};
use wasm_worklet::{
    derive_command, derive_event, derive_param,
    types::{AudioModule, EventCallback, ParamMap},
    worklet::sample_rate,
};

derive_event! {
    pub enum TestEvent {
        One(u32),
        Two,
    }
}

derive_command! {
    pub enum TestCommand {
        Count(u32),
    }
}

derive_param! {
    pub enum TestParams {
        #[param(
            automation_rate = "a-rate",
            min_value = 20.0,
            max_value = 20_000.0,
            default_value = 440.
        )]
        Frequency,
    }
}

pub struct WorkletA {
    send_message: Option<EventCallback<Self>>,
}

impl AudioModule for WorkletA {
    type Event = TestEvent;
    type Command = TestCommand;
    type Param = TestParams;

    fn create() -> Self {
        WorkletA { send_message: None }
    }

    fn add_event_listener_with_callback(&mut self, send_event: EventCallback<Self>) {
        self.send_message = Some(send_event);
    }

    fn on_command(&mut self, command: Self::Command) {
        match command {
            TestCommand::Count(number) => {
                if let Some(send) = &self.send_message {
                    (send)(TestEvent::One(number))
                }
            }
        }
    }

    fn process(
        &mut self,
        _input: &[&[[f32; 128]]],
        output: &mut [&mut [[f32; 128]]],
        params: &ParamMap<Self::Param>,
    ) {
        let frequency =
            signal::from_iter(params.get(TestParams::Frequency).as_ref().iter().cloned());

        let mut sig = signal::rate(sample_rate())
            .hz(frequency.map(|f| f as f64))
            .sine();

        if let Some(channels) = output.get_mut(0) {
            for channel in channels.iter_mut() {
                for sample in channel.iter_mut() {
                    *sample = sig.next() as f32
                }
            }
        }
    }
}

wasm_worklet::module!(WorkletA);
