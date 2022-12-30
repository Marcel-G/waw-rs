use tsify::Tsify;
use waw::{
    buffer::{AudioBuffer, ParamBuffer},
    enum_map::Enum,
    serde::{Deserialize, Serialize},
    types::EventCallback,
    worklet::AudioModule,
};
use waw_macros::RawDescribe;

#[test]
fn audio_worklet_test() {
    #[derive(Serialize, Deserialize, Tsify, Clone, RawDescribe)]
    #[tsify(into_wasm_abi, from_wasm_abi)]
    #[serde(crate = "waw::serde")]
    pub enum TestEvent {
        One(bool),
        Two,
    }

    #[derive(Serialize, Deserialize, Tsify, Clone)]
    #[tsify(into_wasm_abi, from_wasm_abi)]
    #[serde(crate = "waw::serde")]
    pub enum TestCommand {
        Three(Vec<f32>),
        Four,
    }

    #[derive(waw_macros::Param, Serialize, Deserialize, Enum, Debug, Tsify)]
    #[tsify(into_wasm_abi, from_wasm_abi)]
    #[serde(crate = "waw::serde")]
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

        fn process(&mut self, _audio: &mut AudioBuffer, _params: &ParamBuffer<Self::Param>) {
            todo!()
        }
    }

    waw_macros::module!(TestWorklet);
}
