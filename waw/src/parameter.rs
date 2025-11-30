use js_sys::{Object, Reflect};
use wasm_bindgen::JsValue;

/// Parameter descriptor for Web Audio API
///
/// The `ParameterDescriptor` struct is used to specify properties for an AudioParam object
/// that is used in an AudioWorkletNode, following the AudioParamDescriptor dictionary specification.
#[derive(Debug, Clone)]
pub struct ParameterDescriptor {
    /// The name of the parameter.
    pub name: String,
    /// The default value of the parameter.
    pub default_value: f32,
    /// The minimum value the parameter can take.
    pub min_value: f32,
    /// The maximum value the parameter can take.
    pub max_value: f32,
    /// The rate at which the parameter is automated (ARate or KRate).
    pub automation_rate: AutomationRate,
}

impl From<ParameterDescriptor> for JsValue {
    fn from(val: ParameterDescriptor) -> Self {
        let obj = Object::new();
        Reflect::set(&obj, &"name".into(), &val.name.into()).unwrap();
        Reflect::set(&obj, &"defaultValue".into(), &val.default_value.into()).unwrap();
        Reflect::set(&obj, &"minValue".into(), &val.min_value.into()).unwrap();
        Reflect::set(&obj, &"maxValue".into(), &val.max_value.into()).unwrap();
        Reflect::set(&obj, &"automationRate".into(), &val.automation_rate.into()).unwrap();
        obj.into()
    }
}

/// The automation rate of an AudioParam.
///
/// The automation rate can be selected by setting the `automationRate` attribute
/// with one of the following values. Some AudioParams may have constraints on
/// whether the automation rate can be changed.
#[derive(Debug, Clone)]
pub enum AutomationRate {
    /// Audio-rate automation. The parameter is updated for every sample frame.
    ARate,
    /// Control-rate automation. The parameter is updated for every render quantum.
    KRate,
}

impl From<AutomationRate> for JsValue {
    fn from(val: AutomationRate) -> Self {
        match val {
            AutomationRate::ARate => "a-rate".into(),
            AutomationRate::KRate => "k-rate".into(),
        }
    }
}
