use tsify::Tsify;
use wasm_worklet::{
    enum_map::Enum,
    serde::{Deserialize, Serialize},
    types::{AudioModule, EventCallback, ParamMap},
};
use wasm_worklet_macros::RawDescribe;

#[test]
fn audio_worklet_test() {
    #[derive(Serialize, Deserialize, Tsify, Clone, RawDescribe)]
    #[tsify(into_wasm_abi, from_wasm_abi)]
    #[serde(crate = "wasm_worklet::serde")]
    pub enum TestEvent {
        One(bool),
        Two,
    }

    #[derive(Serialize, Deserialize, Tsify, Clone)]
    #[tsify(into_wasm_abi, from_wasm_abi)]
    #[serde(crate = "wasm_worklet::serde")]
    pub enum TestCommand {
        Three(Vec<f32>),
        Four,
    }

    #[derive(wasm_worklet_macros::Param, Serialize, Deserialize, Enum, Debug, Tsify)]
    #[tsify(into_wasm_abi, from_wasm_abi)]
    #[serde(crate = "wasm_worklet::serde")]
    pub enum TestParam {
        #[param(automation_rate = "a-rate", max_value = 1.0, default_value = 2.0)]
        Test,
    }

    pub struct TestWorklet {
        send_message: Option<EventCallback<TestWorklet>>,
    }

    impl AudioModule for TestWorklet {
        type Event = TestEvent;
        type Command = TestCommand;
        type Param = TestParam;

        fn create() -> Self {
            TestWorklet { send_message: None }
        }

        fn add_event_listener_with_callback(&mut self, send_event: EventCallback<Self>) {
            self.send_message = Some(send_event);
        }

        fn on_command(&mut self, command: Self::Command) {
            match command {
                TestCommand::Three(_) => self.send_message.as_ref().unwrap()(TestEvent::One(false)),
                TestCommand::Four => self.send_message.as_ref().unwrap()(TestEvent::Two),
            }
        }

        fn process(
            &mut self,
            _input: &[&[[f32; 128]]],
            _output: &mut [&mut [[f32; 128]]],
            _params: &ParamMap<Self::Param>,
        ) {
            todo!()
        }
    }

    wasm_worklet_macros::module!(TestWorklet);
}
