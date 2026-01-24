#![cfg(all(target_family = "wasm", feature = "audio-worklet"))]
#![allow(
	clippy::missing_panics_doc,
	missing_copy_implementations,
	missing_debug_implementations,
	unreachable_pub
)]

use std::cell::{OnceCell, RefCell};
use std::marker::PhantomData;

use js_sys::{Array, Iterator, Object};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsCast;
use web_sys::{AudioWorkletNodeOptions, AudioWorkletProcessor};
use web_thread::web::audio_worklet::ExtendAudioWorkletProcessor;

thread_local! {
	#[allow(clippy::type_complexity)]
	pub static GLOBAL_DATA: OnceCell<RefCell<Option<Box<dyn FnOnce(AudioWorkletNodeOptionsExt) -> Option<Box<dyn FnMut() -> bool>>>>>> = OnceCell::new();
}

pub struct TestProcessor<P: AudioParameter = ()> {
	process: Option<Box<dyn FnMut() -> bool>>,
	parameter: PhantomData<P>,
}

impl<P: AudioParameter> ExtendAudioWorkletProcessor for TestProcessor<P> {
	type Data =
		Box<dyn FnOnce(AudioWorkletNodeOptionsExt) -> Option<Box<dyn FnMut() -> bool>> + Send>;

	fn new(
		_: AudioWorkletProcessor,
		data: Option<Self::Data>,
		options: AudioWorkletNodeOptions,
	) -> Self {
		let process = if let Some(data) =
			GLOBAL_DATA.with(|data| data.get().and_then(|data| data.borrow_mut().take()))
		{
			data(options.unchecked_into())
		} else if let Some(data) = data {
			data(options.unchecked_into())
		} else {
			None
		};

		Self {
			process,
			parameter: PhantomData,
		}
	}

	fn process(&mut self, _: Array, _: Array, _: Object) -> bool {
		if let Some(fun) = &mut self.process {
			if fun() {
				true
			} else {
				self.process = None;
				false
			}
		} else {
			false
		}
	}

	fn parameter_descriptors() -> Iterator {
		P::parameter_descriptors()
	}
}

pub trait AudioParameter {
	#[must_use]
	fn parameter_descriptors() -> Iterator;
}

impl AudioParameter for () {
	fn parameter_descriptors() -> Iterator {
		Array::new().values()
	}
}

#[wasm_bindgen]
extern "C" {
	#[wasm_bindgen(extends = AudioWorkletNodeOptions)]
	#[derive(Default)]
	pub type AudioWorkletNodeOptionsExt;

	#[wasm_bindgen(getter, method, js_name = parameterData)]
	pub fn get_parameter_data(this: &AudioWorkletNodeOptionsExt) -> Option<Array>;

	#[wasm_bindgen(setter, method, js_name = parameterData)]
	pub fn set_parameter_data(this: &AudioWorkletNodeOptionsExt, value: Option<&Array>);
}

#[macro_export]
macro_rules! test_audio {
	($name:ident$(, should_panic = $should_panic:tt)?) => {
		::paste::paste! {
			#[::wasm_bindgen_test::wasm_bindgen_test]
			$(#[should_panic = $should_panic])?
			async fn $name() {
				[<test_ $name>](::web_sys::AudioContext::new().unwrap().into()).await;
			}

			#[::wasm_bindgen_test::wasm_bindgen_test]
			$(#[should_panic = $should_panic])?
			async fn [<offline_ $name>]() {
				let context = ::web_sys::OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(
					1, 1, 8000.,
				)
				.unwrap();
				[<test_ $name>](context.into()).await;
			}
		}
	};
}
