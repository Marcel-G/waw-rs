use std::fmt::Debug;

use enum_map::{Enum, EnumArray, EnumMap, Iter};
use js_sys::{Float32Array, Reflect};
use serde::{Deserialize, Deserializer, Serialize};

use wasm_bindgen::{prelude::wasm_bindgen, JsCast, JsValue};
use web_sys::AudioWorkletNodeOptions;

use crate::{inform_char, utils::callback::RawHackDescribe};

#[derive(Serialize, Deserialize, Debug)]
pub enum InternalMessage {
    Destroy,
}

#[derive(Serialize, Debug, PartialEq)]
pub enum AutomationRate {
    #[serde(rename(serialize = "a-rate"))]
    ARate,
    #[serde(rename(serialize = "k-rate"))]
    KRate,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct AudioParamDescriptor {
    #[serde(rename(serialize = "name"))]
    pub name: String,
    #[serde(rename(serialize = "automationRate"))]
    pub automation_rate: Option<AutomationRate>,
    #[serde(rename(serialize = "minValue"))]
    pub min_value: Option<f32>,
    #[serde(rename(serialize = "maxValue"))]
    pub max_value: Option<f32>,
    #[serde(rename(serialize = "defaultValue"))]
    pub default_value: Option<f32>,
}

fn one() -> u32 {
    1
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct WorkletOptions {
    #[serde(
        rename(serialize = "numberOfInputs", deserialize = "numberOfInputs"),
        default = "one"
    )]
    pub number_of_inputs: u32,
    #[serde(
        rename(serialize = "numberOfOutputs", deserialize = "numberOfOutputs"),
        default = "one"
    )]
    pub number_of_outputs: u32,
    #[serde(
        rename(serialize = "channelCount", deserialize = "channelCount"),
        default = "one"
    )]
    pub channel_count: u32,
    #[serde(
        rename(serialize = "outputChannelCount", deserialize = "outputChannelCount"),
        default = "one"
    )]
    pub output_channel_count: u32,
}

// Bindgen only defines setters for AudioWorkletNodeOptions and no getters.
impl From<AudioWorkletNodeOptions> for WorkletOptions {
    fn from(value: AudioWorkletNodeOptions) -> Self {
        serde_wasm_bindgen::from_value(value.into()).unwrap()
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "never")]
    #[derive(Clone, Debug)]
    pub type Never;
}

impl Enum for Never {
    const LENGTH: usize = 1;

    #[inline]
    fn from_usize(value: usize) -> Self {
        match value {
            0 => Never { obj: JsValue::NULL },
            _ => unreachable!(),
        }
    }
    #[inline]
    fn into_usize(self) -> usize {
        0
    }
}

impl<T> EnumArray<T> for Never {
    type Array = [T; Self::LENGTH];
}

impl ParameterDescriptor for Never {
    fn descriptors() -> Vec<AudioParamDescriptor> {
        vec![]
    }
}

impl Serialize for Never {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        unreachable!()
    }
}

impl<'de> Deserialize<'de> for Never {
    fn deserialize<D>(_deserializer: D) -> Result<Never, D::Error>
    where
        D: Deserializer<'de>,
    {
        unreachable!()
    }
}

impl RawHackDescribe for Never {
    fn len() -> u32 {
        5
    }

    fn raw_describe() {
        use wasm_bindgen::describe::*;
        inform_char!('n', 'e', 'v', 'e', 'r',); // Lol
    }
}

// @todo - reconcile this buffer with buffer.rs
#[derive(Debug)]
pub struct Buffer([f32; 128]);

impl Buffer {
    pub fn fill(&mut self, value: f32) {
        self.0.iter_mut().for_each(|m| *m = value)
    }
}

impl AsRef<[f32]> for Buffer {
    fn as_ref(&self) -> &[f32] {
        self.0.as_ref()
    }
}

impl AsMut<[f32]> for Buffer {
    fn as_mut(&mut self) -> &mut [f32] {
        self.0.as_mut()
    }
}
impl Default for Buffer {
    fn default() -> Self {
        Buffer([0.0; 128])
    }
}

pub struct ParamMap<P>(EnumMap<P, Buffer>)
where
    P: EnumArray<Buffer> + Debug;

impl<P> Default for ParamMap<P>
where
    P: EnumArray<Buffer> + Debug,
{
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<P> ParamMap<P>
where
    P: EnumArray<Buffer> + Debug,
{
    // js_params: An object containing string keys and Float32Array values.
    //      For each custom AudioParam defined using the parameterDescriptors getter, the key in the object is a name of that AudioParam,
    //      and the value is a Float32Array. The values of the array are calculated by taking scheduled automation events into consideration.
    pub fn copy_from(&mut self, js_params: &JsValue) {
        for (param, buffer) in self.0.iter_mut() {
            if let Some(js_array) = Reflect::get(js_params, &format!("{param:?}").into()) // @todo -- Using debug for param names may be a bad idea.
                .ok()
                .filter(|val| !val.is_undefined()) // This case happens when using th Never type.
                .map(|val| val.unchecked_into::<Float32Array>())
            {
                match js_array.length() {
                    // If the automation rate of the parameter is "a-rate", the array will contain 128 values — one for each frame in the current audio block.
                    128 => {
                        js_array.copy_to(buffer.as_mut());
                    }

                    // If the automation rate is "k-rate", the array will contain a single value, which is to be used for each of 128 frames.
                    //
                    // If there's no automation happening during the time represented by the current block,
                    // the array may contain a single value that is constant for the entire block, instead of 128 identical values.
                    1 => {
                        buffer.fill(js_array.get_index(0));
                    }

                    // Other possibilities are not supported. @todo - this just appears as `unreachable` in the console.
                    other => {
                        panic!("Float32Array length {other} not supported.");
                    }
                }
            }
        }
    }

    pub fn get(&self, name: P) -> &Buffer {
        &self.0[name]
    }

    pub fn iter(&self) -> Iter<P, Buffer> {
        self.0.iter()
    }
}

pub type EventCallback<M> = Box<dyn Fn(<M as AudioModule>::Event)>;

pub trait AudioModule {
    type Event: Serialize + for<'de> Deserialize<'de> + RawHackDescribe + Clone = Never;

    type Command: Serialize + for<'de> Deserialize<'de> + Clone = Never;

    type Param: EnumArray<Buffer> + ParameterDescriptor + Debug = Never;

    const INPUTS: u32 = 1;
    const OUTPUTS: u32 = 1;

    fn create() -> Self;

    fn add_event_listener_with_callback(&mut self, _callback: EventCallback<Self>) {}

    fn on_command(&mut self, _command: Self::Command) {}

    fn process(
        &mut self,
        input: &[&[[f32; 128]]],
        output: &mut [&mut [[f32; 128]]],
        params: &ParamMap<Self::Param>,
    );
}

pub trait ModuleEventEmitter<M: AudioModule> {
    fn send_event(&mut self, event: M::Event);
}

pub trait AudioModuleDescriptor {
    fn processor_name() -> &'static str;
    fn parameter_descriptor_json() -> String;
}

pub trait ParameterDescriptor {
    fn descriptors() -> Vec<AudioParamDescriptor>;
}
