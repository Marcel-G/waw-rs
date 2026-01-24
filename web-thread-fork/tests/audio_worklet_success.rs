#![cfg(test)]
#![cfg(all(target_family = "wasm", feature = "audio-worklet"))]

use std::cell::RefCell;
use std::future::Future;

use js_sys::{Array, Iterator, JsString, Object, Reflect};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::{
	AudioContext, AudioWorkletGlobalScope, AudioWorkletNode, AudioWorkletNodeOptions,
	BaseAudioContext, OfflineAudioContext,
};
use web_thread::web::audio_worklet::{AudioWorkletGlobalScopeExt, BaseAudioContextExt};
use web_thread::web::{self, JoinHandleExt, YieldTime};

use super::test_processor::{
	AudioParameter, AudioWorkletNodeOptionsExt, TestProcessor, GLOBAL_DATA,
};
use super::util::Flag;
use crate::test_audio;

async fn test_nested(context: BaseAudioContext) {
	let (sender, receiver) = async_channel::bounded(1);
	context
		.clone()
		.register_thread(None, move || {
			sender.try_send(web_thread::spawn(|| ())).unwrap();
		})
		.await
		.unwrap();

	receiver.recv().await.unwrap().join_async().await.unwrap();
}

test_audio!(nested);

async fn test_register(context: BaseAudioContext) {
	context.register_thread(None, || ()).await.unwrap();
}

test_audio!(register);

async fn test_stack_size(context: BaseAudioContext) {
	#[allow(clippy::large_stack_frames, clippy::missing_const_for_fn)]
	fn allocate_on_stack() {
		#[allow(clippy::large_stack_arrays, clippy::no_effect_underscore_binding)]
		let _test = [0_u8; 1024 * 1024 * 9];
	}

	let flag = Flag::new();

	context.register_thread(Some(1024 * 1024 * 10), {
		let flag = flag.clone();
		move || {
			allocate_on_stack();
			flag.signal();
		}
	});

	flag.await;
}

test_audio!(stack_size);

async fn test_register_release(context: BaseAudioContext) {
	let flag = Flag::new();

	let handle = context
		.register_thread(None, {
			let flag = flag.clone();
			move || flag.signal()
		})
		.await
		.unwrap();

	flag.await;
	// SAFETY: We are sure the thread has spawned by now and we also didn't register
	// any events or promises that could call into the Wasm module later.
	unsafe { handle.release() }.unwrap();
}

test_audio!(register_release);

async fn test_register_drop(context: BaseAudioContext) {
	let flag = Flag::new();

	context.register_thread(None, {
		let flag = flag.clone();
		move || flag.signal()
	});

	flag.await;
}

test_audio!(register_drop);

async fn test_node(context: BaseAudioContext) {
	let start = Flag::new();
	let end = Flag::new();
	context
		.clone()
		.register_thread(None, {
			let start = start.clone();
			let end = end.clone();
			move || {
				GLOBAL_DATA.with(move |data| {
					#[allow(clippy::blocks_in_conditions)]
					if data
						.set(RefCell::new(Some(Box::new(move |_| {
							end.signal();
							None
						}))))
						.is_err()
					{
						panic!()
					}
				});
				let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
				global
					.register_processor_ext::<TestProcessor>("test")
					.unwrap();
				start.signal();
			}
		})
		.await
		.unwrap();

	// Wait until processor is registered.
	start.await;
	web::yield_now_async(YieldTime::UserBlocking).await;

	AudioWorkletNode::new(&context, "test").unwrap();
	end.await;
}

test_audio!(node);

async fn test_node_data(context: BaseAudioContext) {
	let start = Flag::new();
	context
		.clone()
		.register_thread(None, {
			let start = start.clone();
			move || {
				let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
				global
					.register_processor_ext::<TestProcessor>("test")
					.unwrap();
				start.signal();
			}
		})
		.await
		.unwrap();

	// Wait until processor is registered.
	start.await;
	web::yield_now_async(YieldTime::UserBlocking).await;

	let end = Flag::new();
	context
		.audio_worklet_node::<TestProcessor>(
			"test",
			Box::new({
				let end = end.clone();
				move |_| {
					end.signal();
					None
				}
			}),
			None,
		)
		.unwrap();
	end.await;
}

test_audio!(node_data);

async fn test_unpark(context: BaseAudioContext) {
	let start = Flag::new();
	let handle = context
		.clone()
		.register_thread(None, {
			let start = start.clone();
			move || {
				let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
				global
					.register_processor_ext::<TestProcessor>("test")
					.unwrap();
				start.signal();
			}
		})
		.await
		.unwrap();

	// Wait until processor is registered.
	start.await;
	web::yield_now_async(YieldTime::UserBlocking).await;

	handle.thread().unpark();
	let end = Flag::new();
	context
		.audio_worklet_node::<TestProcessor>(
			"test",
			Box::new({
				let end = end.clone();
				move |_| {
					web_thread::park();
					end.signal();
					None
				}
			}),
			None,
		)
		.unwrap();
	end.await;
}

test_audio!(unpark);

async fn test_process<C, F>(context: C, post: impl FnOnce(C) -> F)
where
	C: Clone + BaseAudioContextExt + AsRef<BaseAudioContext>,
	F: Future<Output = ()>,
{
	let start = Flag::new();
	let end = Flag::new();
	context
		.clone()
		.register_thread(None, {
			let start = start.clone();
			move || {
				let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
				global
					.register_processor_ext::<TestProcessor>("test")
					.unwrap();
				start.signal();
			}
		})
		.await
		.unwrap();

	// Wait until processor is registered.
	start.await;
	web::yield_now_async(YieldTime::UserBlocking).await;

	context
		.audio_worklet_node::<TestProcessor>(
			"test",
			Box::new({
				let end = end.clone();
				move |_| {
					Some(Box::new(move || {
						end.signal();
						false
					}))
				}
			}),
			None,
		)
		.unwrap();
	post(context).await;
	end.await;
}

#[wasm_bindgen_test]
// Firefox doesn't support running `AudioContext` without an actual audio device.
// See <https://bugzilla.mozilla.org/show_bug.cgi?id=1881904>.
#[cfg(not(unsupported_headless_audiocontext))]
async fn process() {
	test_process(AudioContext::new().unwrap(), |_| async {}).await;
}

#[wasm_bindgen_test]
async fn offline_process() {
	test_process(
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap(),
		|context| async move {
			JsFuture::from(context.start_rendering().unwrap())
				.await
				.unwrap();
		},
	)
	.await;
}

async fn test_no_options(context: BaseAudioContext) {
	let start = Flag::new();
	let end = Flag::new();
	context
		.clone()
		.register_thread(None, {
			let start = start.clone();
			let end = end.clone();
			move || {
				GLOBAL_DATA.with(move |data| {
					#[allow(clippy::blocks_in_conditions)]
					if data
						.set(RefCell::new(Some(Box::new(move |options| {
							assert!(options.get_processor_options().is_none());
							end.signal();
							None
						}))))
						.is_err()
					{
						panic!()
					}
				});
				let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
				global
					.register_processor_ext::<TestProcessor>("test")
					.unwrap();
				start.signal();
			}
		})
		.await
		.unwrap();

	// Wait until processor is registered.
	start.await;
	web::yield_now_async(YieldTime::UserBlocking).await;

	AudioWorkletNode::new(&context, "test").unwrap();
	end.await;
}

test_audio!(no_options);

async fn test_zero_options(context: BaseAudioContext) {
	let start = Flag::new();
	let end = Flag::new();
	context
		.clone()
		.register_thread(None, {
			let start = start.clone();
			let end = end.clone();
			move || {
				GLOBAL_DATA.with(move |data| {
					#[allow(clippy::blocks_in_conditions)]
					if data
						.set(RefCell::new(Some(Box::new(move |options| {
							assert_eq!(
								Object::keys(&options.get_processor_options().unwrap()).length(),
								0
							);
							end.signal();
							None
						}))))
						.is_err()
					{
						panic!()
					}
				});
				let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
				global
					.register_processor_ext::<TestProcessor>("test")
					.unwrap();
				start.signal();
			}
		})
		.await
		.unwrap();

	// Wait until processor is registered.
	start.await;
	web::yield_now_async(YieldTime::UserBlocking).await;

	let options = AudioWorkletNodeOptions::new();
	options.set_processor_options(Some(&Object::new()));
	AudioWorkletNode::new_with_options(&context, "test", &options).unwrap();
	end.await;
}

test_audio!(zero_options);

async fn test_data_no_options(context: BaseAudioContext) {
	let start = Flag::new();
	context
		.clone()
		.register_thread(None, {
			let start = start.clone();
			move || {
				let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
				global
					.register_processor_ext::<TestProcessor>("test")
					.unwrap();
				start.signal();
			}
		})
		.await
		.unwrap();

	// Wait until processor is registered.
	start.await;
	web::yield_now_async(YieldTime::UserBlocking).await;

	let end = Flag::new();
	context
		.audio_worklet_node::<TestProcessor>(
			"test",
			Box::new({
				let end = end.clone();
				move |options| {
					assert!(options.get_processor_options().is_none());
					end.signal();
					None
				}
			}),
			None,
		)
		.unwrap();
	end.await;
}

test_audio!(data_no_options);

async fn test_data_empty_options(context: BaseAudioContext) {
	let start = Flag::new();
	context
		.clone()
		.register_thread(None, {
			let start = start.clone();
			move || {
				let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
				global
					.register_processor_ext::<TestProcessor>("test")
					.unwrap();
				start.signal();
			}
		})
		.await
		.unwrap();

	// Wait until processor is registered.
	start.await;
	web::yield_now_async(YieldTime::UserBlocking).await;

	let end = Flag::new();
	let options = AudioWorkletNodeOptions::new();
	context
		.audio_worklet_node::<TestProcessor>(
			"test",
			Box::new({
				let end = end.clone();
				move |options| {
					assert!(options.get_processor_options().is_none());
					end.signal();
					None
				}
			}),
			Some(&options),
		)
		.unwrap();
	assert!(options.get_processor_options().is_none());
	end.await;
}

test_audio!(data_empty_options);

async fn test_data_zero_options(context: BaseAudioContext) {
	let start = Flag::new();
	context
		.clone()
		.register_thread(None, {
			let start = start.clone();
			move || {
				let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
				global
					.register_processor_ext::<TestProcessor>("test")
					.unwrap();
				start.signal();
			}
		})
		.await
		.unwrap();

	// Wait until processor is registered.
	start.await;
	web::yield_now_async(YieldTime::UserBlocking).await;

	let end = Flag::new();
	let options = AudioWorkletNodeOptions::new();
	options.set_processor_options(Some(&Object::new()));
	context
		.audio_worklet_node::<TestProcessor>(
			"test",
			Box::new({
				let end = end.clone();
				move |options| {
					assert_eq!(
						Object::keys(&options.get_processor_options().unwrap()).length(),
						0
					);
					end.signal();
					None
				}
			}),
			Some(&options),
		)
		.unwrap();
	assert_eq!(
		Object::keys(&options.get_processor_options().unwrap()).length(),
		0
	);
	end.await;
}

test_audio!(data_zero_options);

async fn test_options(context: BaseAudioContext) {
	let start = Flag::new();
	let end = Flag::new();
	context
		.clone()
		.register_thread(None, {
			let start = start.clone();
			let end = end.clone();
			move || {
				GLOBAL_DATA.with(move |data| {
					#[allow(clippy::blocks_in_conditions, clippy::float_cmp)]
					if data
						.set(RefCell::new(Some(Box::new(move |options| {
							let options: Object = options.get_processor_options().unwrap();
							let var = Reflect::get_u32(&options, 0).unwrap();
							assert_eq!(var, 42.);
							assert_eq!(Object::keys(&options).length(), 1);
							end.signal();
							None
						}))))
						.is_err()
					{
						panic!()
					}
				});
				let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
				global
					.register_processor_ext::<TestProcessor>("test")
					.unwrap();
				start.signal();
			}
		})
		.await
		.unwrap();

	// Wait until processor is registered.
	start.await;
	web::yield_now_async(YieldTime::UserBlocking).await;

	let processor_options = Object::new();
	Reflect::set_u32(&processor_options, 0, &42.into()).unwrap();
	let options = AudioWorkletNodeOptions::new();
	options.set_processor_options(Some(&processor_options));
	AudioWorkletNode::new_with_options(&context, "test", &options).unwrap();
	end.await;
}

test_audio!(options);

async fn test_options_data(context: BaseAudioContext) {
	let start = Flag::new();
	context
		.clone()
		.register_thread(None, {
			let start = start.clone();
			move || {
				let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
				global
					.register_processor_ext::<TestProcessor>("test")
					.unwrap();
				start.signal();
			}
		})
		.await
		.unwrap();

	// Wait until processor is registered.
	start.await;
	web::yield_now_async(YieldTime::UserBlocking).await;

	let end = Flag::new();
	let inner_options = Object::new();
	Reflect::set_u32(&inner_options, 0, &42.into()).unwrap();
	let options = AudioWorkletNodeOptions::new();
	options.set_processor_options(Some(&inner_options));
	context
		.audio_worklet_node::<TestProcessor>(
			"test",
			Box::new({
				let end = end.clone();
				#[allow(clippy::float_cmp)]
				move |options| {
					let options: Object = options.get_processor_options().unwrap();
					let var = Reflect::get_u32(&options, 0).unwrap();
					assert_eq!(var, 42.);
					assert_eq!(Object::keys(&options).length(), 1);
					end.signal();
					None
				}
			}),
			Some(&options),
		)
		.unwrap();
	assert_eq!(Object::keys(&inner_options).length(), 1);
	end.await;
}

test_audio!(options_data);

struct TestParameters;

impl AudioParameter for TestParameters {
	fn parameter_descriptors() -> Iterator {
		let parameters = Array::new();

		let parameter = Object::new();
		Reflect::set(&parameter, &js_string("name"), &js_string("test")).unwrap();

		parameters.push(&parameter);
		parameters.values()
	}
}

async fn test_parameters(context: BaseAudioContext) {
	let start = Flag::new();
	let end = Flag::new();
	context
		.clone()
		.register_thread(None, {
			let start = start.clone();
			let end = end.clone();
			move || {
				GLOBAL_DATA.with(move |data| {
					#[allow(clippy::blocks_in_conditions)]
					if data
						.set(RefCell::new(Some(Box::new(move |options| {
							let parameters = options.get_parameter_data().unwrap();
							let value = Reflect::get(&parameters, &js_string("test")).unwrap();
							assert_eq!(value, 42.);
							end.signal();
							None
						}))))
						.is_err()
					{
						panic!()
					}
				});
				let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
				global
					.register_processor_ext::<TestProcessor<TestParameters>>("test")
					.unwrap();

				start.signal();
			}
		})
		.await
		.unwrap();

	// Wait until processor is registered.
	start.await;
	web::yield_now_async(YieldTime::UserBlocking).await;

	let options: AudioWorkletNodeOptionsExt = AudioWorkletNodeOptions::new().unchecked_into();
	let parameters = Array::new();
	Reflect::set(&parameters, &"test".into(), &42.0.into()).unwrap();
	options.set_parameter_data(Some(&parameters));
	AudioWorkletNode::new_with_options(&context, "test", &options).unwrap();
	end.await;
}

test_audio!(parameters);

fn js_string(string: &str) -> JsString {
	JsString::from_code_point(string.chars().map(u32::from).collect::<Vec<_>>().as_slice())
		.expect("found invalid Unicode")
}
