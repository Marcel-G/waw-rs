use std::fmt::Debug;

use enum_map::{Enum, EnumArray};

use serde::{Deserialize, Deserializer, Serialize};

use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
use web_sys::AudioWorkletNodeOptions;

use crate::{inform_char, utils::callback::RawHackDescribe, worklet::AudioModule};

#[derive(Serialize, Deserialize, Debug)]
pub enum InternalMessage {
    Destroy,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
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

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
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

pub type EventCallback<M> = Box<dyn Fn(<M as AudioModule>::Event)>;

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
