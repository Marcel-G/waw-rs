use waw::types::{AudioParamDescriptor, AutomationRate, ParameterDescriptor};
use waw_macros::ParameterDescriptor;

#[test]
fn parameter_descriptors_test() {
    #[allow(dead_code)]
    #[derive(ParameterDescriptor)]
    enum Parameters {
        #[param(
            automation_rate = "a-rate",
            min_value = 0.0,
            max_value = 1.0,
            default_value = 1.0
        )]
        Level,
        #[param(automation_rate = "a-rate", max_value = 1.0, default_value = 2.0)]
        PlaybackRate,
    }

    let descriptors = Parameters::descriptors();

    assert_eq!(
        descriptors,
        vec![
            AudioParamDescriptor {
                name: String::from("Level"),
                automation_rate: Some(AutomationRate::ARate),
                min_value: Some(0.0),
                max_value: Some(1.0),
                default_value: Some(1.0)
            },
            AudioParamDescriptor {
                name: String::from("PlaybackRate"),
                automation_rate: Some(AutomationRate::ARate),
                min_value: None,
                max_value: Some(1.0),
                default_value: Some(2.0)
            },
        ]
    );
}
