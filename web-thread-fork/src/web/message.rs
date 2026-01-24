//! Platform-specific extensions for [`web-thread`](crate) on the Web platform
//! to send [`Serializable`] and [`Transferable`] to other threads.
//!
//! [`Serializable`]: https://developer.mozilla.org/en-US/docs/Glossary/Serializable_object
//! [`Transferable`]: https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API/Transferable_objects

use std::{array, iter, mem};

use js_sys::Array;
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use js_sys::{
	ArrayBuffer, BigInt, BigInt64Array, BigUint64Array, Boolean, DataView, Date, Error,
	Float32Array, Float64Array, Int16Array, Int32Array, Int8Array, JsString, Map, Number, RegExp,
	Set, Uint16Array, Uint32Array, Uint8Array, Uint8ClampedArray,
};
use wasm_bindgen::{JsCast, JsValue};
#[cfg(all(
	target_family = "wasm",
	target_os = "unknown",
	feature = "message",
	web_sys_unstable_apis
))]
use web_sys::{AudioData, GpuCompilationInfo, GpuCompilationMessage, VideoFrame};
#[cfg(all(target_family = "wasm", target_os = "unknown", feature = "message"))]
use web_sys::{
	Blob, CryptoKey, DomException, DomMatrix, DomMatrixReadOnly, DomPoint, DomPointReadOnly,
	DomQuad, DomRect, DomRectReadOnly, File, FileList, FileSystemDirectoryHandle,
	FileSystemFileHandle, FileSystemHandle, ImageBitmap, ImageData, MessagePort, OffscreenCanvas,
	ReadableStream, RtcCertificate, RtcDataChannel, TransformStream, WritableStream,
};

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
mod js_sys {
	pub(super) struct Array;
}

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
mod wasm_bindgen {
	pub(super) struct JsValue;
	pub(super) trait JsCast {}
}

/// Implement a trait for a tuple.
macro_rules! impl_for_tuple {
	($_0:literal, $trait:ident, $($generic:ident => $_1:tt),+) => {
		impl<$($generic),+> $trait for ($($generic,)+) {}
	};
}

/// Implement a trait for tuples up to a size of 12.
macro_rules! impl_for_tuples {
	($macro:ident, $trait:ident) => {
		$macro!(1, $trait, T1 => 0);
		$macro!(2, $trait, T1 => 0, T2 => 1);
		$macro!(3, $trait, T1 => 0, T2 => 1, T3 => 2);
		$macro!(4, $trait, T1 => 0, T2 => 1, T3 => 2, T4 => 3);
		$macro!(5, $trait, T1 => 0, T2 => 1, T3 => 2, T4 => 3, T5 => 4);
		$macro!(6, $trait, T1 => 0, T2 => 1, T3 => 2, T4 => 3, T5 => 4, T6 => 5);
		$macro!(7, $trait, T1 => 0, T2 => 1, T3 => 2, T4 => 3, T5 => 4, T6 => 5, T7 => 6);
		$macro!(8, $trait, T1 => 0, T2 => 1, T3 => 2, T4 => 3, T5 => 4, T6 => 5, T7 => 6, T8 => 7);
		$macro!(9, $trait, T1 => 0, T2 => 1, T3 => 2, T4 => 3, T5 => 4, T6 => 5, T7 => 6, T8 => 7, T9 => 8);
		$macro!(10, $trait, T1 => 0, T2 => 1, T3 => 2, T4 => 3, T5 => 4, T6 => 5, T7 => 6, T8 => 7, T9 => 8, T10 => 9);
		$macro!(11, $trait, T1 => 0, T2 => 1, T3 => 2, T4 => 3, T5 => 4, T6 => 5, T7 => 6, T8 => 7, T9 => 8, T10 => 9, T11 => 10);
		$macro!(12, $trait, T1 => 0, T2 => 1, T3 => 2, T4 => 3, T5 => 4, T6 => 5, T7 => 6, T8 => 7, T9 => 8, T10 => 9, T11 => 10, T12 => 11);
	};
}

/// Allows a [`Serializable`] and/or [`Transferable`] value to be sent to
/// another threads. See [`spawn_with_message()`](super::spawn_with_message).
///
/// # Notes
///
/// - Sending a [`JsValue::UNDEFINED`] might be interpreted at the receiving end
///   as [`None`].
/// - No values will be [transferred] if [`RawMessage::serialize`] is [`None`].
///
/// # Example
///
/// ```
/// # #[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
/// # wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
/// # #[cfg_attr(all(target_feature = "atomics", not(unsupported_spawn)), wasm_bindgen_test::wasm_bindgen_test)]
/// # async fn test() {
/// use js_sys::{Array, ArrayBuffer};
/// use wasm_bindgen::{JsCast, JsValue};
/// use web_sys::{HtmlCanvasElement, OffscreenCanvas};
/// use web_thread::web::{self, JoinHandleExt};
/// use web_thread::web::message::{MessageSend, RawMessage};
///
/// struct Struct {
/// 	a: u8,
/// 	b: ArrayBuffer,
/// 	c: OffscreenCanvas,
/// }
///
/// impl MessageSend for Struct {
/// 	type Send = u8;
///
/// 	fn send<E: Extend<JsValue>>(self, transfer: &mut E) -> RawMessage<Self::Send> {
/// 		let serialize = Array::of2(&self.b, &self.c);
/// 		transfer.extend([self.c.into()]);
///
/// 		RawMessage {
/// 			send: Some(self.a),
/// 			serialize: Some(serialize.into()),
/// 		}
/// 	}
///
/// 	fn receive(serialized: Option<JsValue>, sent: Option<Self::Send>) -> Self {
/// 		let array: Array = serialized.unwrap().unchecked_into();
///
/// 		Self {
/// 			a: sent.unwrap(),
/// 			b: array.get(0).unchecked_into(),
/// 			c: array.get(1).unchecked_into(),
/// 		}
/// 	}
/// }
///
/// # let canvas = web_sys::window().unwrap().document().unwrap().create_element("canvas").unwrap().unchecked_into();
/// let canvas: HtmlCanvasElement = canvas;
/// let message = Struct {
/// 	a: 42,
/// 	b: ArrayBuffer::new(1000),
/// 	c: canvas.transfer_control_to_offscreen().unwrap(),
/// };
/// web::spawn_with_message(
/// 	|message| async move {
/// 		// Do work.
/// #       let _ = message;
/// 	},
/// 	message,
/// )
/// .join_async()
/// .await
/// .unwrap();
/// # }
/// # #[cfg(not(all(target_feature = "atomics", not(unsupported_spawn))))]
/// # let _ = test();
/// ```
///
/// [`Serializable`]: https://developer.mozilla.org/en-US/docs/Glossary/Serializable_object
/// [`Transferable`]: https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API/Transferable_objects
/// [transferred]: https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API/Transferable_objects
#[cfg_attr(
	not(all(target_family = "wasm", target_os = "unknown", feature = "message")),
	doc = "[`JsValue::UNDEFINED`]: https://docs.rs/wasm-bindgen/0.2.92/wasm_bindgen/struct.JsValue.html#associatedconstant.UNDEFINED"
)]
pub trait MessageSend {
	/// [`Send`] type.
	type Send: Send;

	/// Serialize into [`RawMessage`] to send.
	fn send<E: Extend<JsValue>>(self, transfer: &mut E) -> RawMessage<Self::Send>;

	/// Deserialize from [`RawMessage::serialize`] and [`RawMessage::send`].
	fn receive(serialized: Option<JsValue>, sent: Option<Self::Send>) -> Self;
}

/// Contains data necessary to send [`MessageSend`] to another thread.
#[derive(Debug, PartialEq)]
pub struct RawMessage<T> {
	/// Value to be [serialized](https://developer.mozilla.org/en-US/docs/Glossary/Serializable_object).
	pub serialize: Option<JsValue>,
	/// [`Send`] value.
	pub send: Option<T>,
}

impl<const SIZE: usize, T: MessageSend> MessageSend for [T; SIZE] {
	type Send = [Option<T::Send>; SIZE];

	fn send<E: Extend<JsValue>>(self, transfer: &mut E) -> RawMessage<Self::Send> {
		let mut serialize_builder = None;

		let mut empty_serialize_count = 0;
		let mut has_send = false;
		let send = self.map(|message| {
			let message = message.send(transfer);

			if let Some(serialize) = message.serialize {
				serialize_builder
					.get_or_insert_with(|| {
						let mut builder = ArrayBuilder::new();
						builder.extend(iter::repeat(JsValue::NULL).take(empty_serialize_count));
						builder
					})
					.push(serialize);
			} else {
				empty_serialize_count += 1;
			}

			if message.send.is_some() {
				has_send = true;
			}

			message.send
		});

		RawMessage {
			serialize: serialize_builder
				.and_then(ArrayBuilder::finish)
				.map(Array::unchecked_into),
			send: has_send.then_some(send),
		}
	}

	fn receive(serialized: Option<JsValue>, mut sent: Option<Self::Send>) -> Self {
		let serialized = serialized.map(Array::unchecked_from_js);

		if let Some(serialized) = &serialized {
			debug_assert_eq!(
				serialized.length(),
				usize_is_u32(SIZE),
				"unexpected array size during message receival"
			);
		}

		array::from_fn(|index| {
			let serialized = serialized
				.as_ref()
				.map(|serialized| serialized.get(usize_is_u32(index)))
				.filter(|value| !value.is_null());
			let sent = sent
				.as_mut()
				.and_then(|sent| sent.get_mut(index))
				.and_then(Option::take);
			T::receive(serialized, sent)
		})
	}
}

/// Implement [`MessageSend`] for a tuple.
macro_rules! message_send_for_tuple {
	($size:literal, $_:ident, $($generic:ident => $index:tt),+) => {
		impl<$($generic: MessageSend),+> MessageSend for ($($generic,)+) {
			type Send = ($(Option<$generic::Send>,)+);

			fn send<E: Extend<JsValue>>(self, transfer: &mut E) -> RawMessage<Self::Send> {
				let mut serialize_builder = None;

				let mut empty_serialize_count = 0;
				let mut has_send = false;
				let send = ($({
					let message = self.$index.send(transfer);

					#[allow(clippy::mixed_read_write_in_expression, unused_assignments)]
					if let Some(serialize) = message.serialize {
						serialize_builder
							.get_or_insert_with(|| {
								let mut builder = ArrayBuilder::new();
								builder.extend(iter::repeat(JsValue::NULL).take(empty_serialize_count));
								builder
							})
							.push(serialize);
					} else {
						empty_serialize_count += 1;
					}

					if message.send.is_some() {
						has_send = true;
					}

					message.send
				},)+);

				RawMessage {
					send: has_send.then_some(send),
					serialize: serialize_builder
						.and_then(ArrayBuilder::finish)
						.map(Array::unchecked_into),
				}
			}

			fn receive(serialized: Option<JsValue>, mut sent: Option<Self::Send>) -> Self {
				let serialized = serialized.map(Array::unchecked_from_js);

				if let Some(serialized) = &serialized {
					debug_assert_eq!(
						serialized.length(),
						$size,
						"unexpected array size during message receival"
					);
				}

				($({
					let serialized = serialized
						.as_ref()
						.map(|serialized| serialized.get($index))
						.filter(|value| !value.is_null());
					let sent = sent.as_mut().and_then(|sent| sent.$index.take());
					$generic::receive(serialized, sent)
				},)+)

			}
		}
	};
}

impl_for_tuples!(message_send_for_tuple, MessageSend);

/// Value can be [serialized](https://developer.mozilla.org/en-US/docs/Glossary/Serializable_object).
pub trait Serializable {}

impl<const SIZE: usize, T: Serializable> Serializable for [T; SIZE] {}
impl_for_tuples!(impl_for_tuple, Serializable);

// Primitive Types
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
impl Serializable for BigInt {}
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
impl Serializable for Boolean {}
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
impl Serializable for JsString {}
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
impl Serializable for Number {}

// JS Types
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
impl Serializable for Array {}
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
impl Serializable for ArrayBuffer {}
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
impl Serializable for DataView {}
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
impl Serializable for Date {}
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
impl Serializable for Error {}
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
impl Serializable for Map {}
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
impl Serializable for RegExp {}
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
impl Serializable for Set {}

// `TypedArray`
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
impl Serializable for BigInt64Array {}
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
impl Serializable for BigUint64Array {}
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
impl Serializable for Float32Array {}
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
impl Serializable for Float64Array {}
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
impl Serializable for Int8Array {}
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
impl Serializable for Int16Array {}
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
impl Serializable for Int32Array {}
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
impl Serializable for Uint8Array {}
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
impl Serializable for Uint8ClampedArray {}
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
impl Serializable for Uint16Array {}
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
impl Serializable for Uint32Array {}

// Web/API types
#[cfg(all(
	target_family = "wasm",
	target_os = "unknown",
	feature = "message",
	web_sys_unstable_apis
))]
#[cfg_attr(docsrs, doc(cfg(web_sys_unstable_apis)))]
impl Serializable for AudioData {}
#[cfg(all(target_family = "wasm", target_os = "unknown", feature = "message"))]
impl Serializable for Blob {}
#[cfg(all(target_family = "wasm", target_os = "unknown", feature = "message"))]
impl Serializable for CryptoKey {}
#[cfg(all(target_family = "wasm", target_os = "unknown", feature = "message"))]
impl Serializable for DomException {}
#[cfg(all(target_family = "wasm", target_os = "unknown", feature = "message"))]
impl Serializable for DomMatrix {}
#[cfg(all(target_family = "wasm", target_os = "unknown", feature = "message"))]
impl Serializable for DomMatrixReadOnly {}
#[cfg(all(target_family = "wasm", target_os = "unknown", feature = "message"))]
impl Serializable for DomPoint {}
#[cfg(all(target_family = "wasm", target_os = "unknown", feature = "message"))]
impl Serializable for DomPointReadOnly {}
#[cfg(all(target_family = "wasm", target_os = "unknown", feature = "message"))]
impl Serializable for DomQuad {}
#[cfg(all(target_family = "wasm", target_os = "unknown", feature = "message"))]
impl Serializable for DomRect {}
#[cfg(all(target_family = "wasm", target_os = "unknown", feature = "message"))]
impl Serializable for DomRectReadOnly {}
#[cfg(all(target_family = "wasm", target_os = "unknown", feature = "message"))]
impl Serializable for File {}
#[cfg(all(target_family = "wasm", target_os = "unknown", feature = "message"))]
impl Serializable for FileList {}
#[cfg(all(target_family = "wasm", target_os = "unknown", feature = "message"))]
impl Serializable for FileSystemDirectoryHandle {}
#[cfg(all(target_family = "wasm", target_os = "unknown", feature = "message"))]
impl Serializable for FileSystemFileHandle {}
#[cfg(all(target_family = "wasm", target_os = "unknown", feature = "message"))]
impl Serializable for FileSystemHandle {}
#[cfg(all(
	target_family = "wasm",
	target_os = "unknown",
	feature = "message",
	web_sys_unstable_apis
))]
#[cfg_attr(docsrs, doc(cfg(web_sys_unstable_apis)))]
impl Serializable for GpuCompilationInfo {}
#[cfg(all(
	target_family = "wasm",
	target_os = "unknown",
	feature = "message",
	web_sys_unstable_apis
))]
#[cfg_attr(docsrs, doc(cfg(web_sys_unstable_apis)))]
impl Serializable for GpuCompilationMessage {}
#[cfg(all(target_family = "wasm", target_os = "unknown", feature = "message"))]
impl Serializable for ImageBitmap {}
#[cfg(all(target_family = "wasm", target_os = "unknown", feature = "message"))]
impl Serializable for ImageData {}
#[cfg(all(target_family = "wasm", target_os = "unknown", feature = "message"))]
impl Serializable for RtcCertificate {}
#[cfg(all(
	target_family = "wasm",
	target_os = "unknown",
	feature = "message",
	web_sys_unstable_apis
))]
#[cfg_attr(docsrs, doc(cfg(web_sys_unstable_apis)))]
impl Serializable for VideoFrame {}

/// Wrapper that implements [`MessageSend`] for values implementing
/// [`Serializable`].
///
/// # Example
///
/// ```
/// # #[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
/// # wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
/// # #[cfg_attr(all(target_feature = "atomics", not(unsupported_spawn)), wasm_bindgen_test::wasm_bindgen_test)]
/// # async fn test() {
/// use js_sys::ArrayBuffer;
/// use web_thread::web::{self, JoinHandleExt};
/// use web_thread::web::message::SerializableWrapper;
///
/// let message = SerializableWrapper(ArrayBuffer::new(1000));
/// web::spawn_with_message(
/// 	|message| async move {
/// 		// Do work.
/// #   	let _ = message;
/// 	},
/// 	message,
/// )
/// .join_async()
/// .await
/// .unwrap();
/// # }
/// # #[cfg(not(all(target_feature = "atomics", not(unsupported_spawn))))]
/// # let _ = test();
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SerializableWrapper<T>(pub T)
where
	T: Into<JsValue> + JsCast + Serializable;

impl<T: Into<JsValue> + JsCast + Serializable> From<T> for SerializableWrapper<T> {
	fn from(value: T) -> Self {
		Self(value)
	}
}

impl<T: Into<JsValue> + JsCast + Serializable> MessageSend for SerializableWrapper<T> {
	type Send = ();

	fn send<E: Extend<JsValue>>(self, _: &mut E) -> RawMessage<Self::Send> {
		RawMessage {
			serialize: Some(self.0.into()),
			send: None,
		}
	}

	fn receive(serialized: Option<JsValue>, sent: Option<Self::Send>) -> Self {
		debug_assert_eq!(sent, None, "unexpected `Send` value");

		Self(
			serialized
				.expect("expected serialized value")
				.unchecked_into(),
		)
	}
}

/// Value can be [transferred](https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API/Transferable_objects).
pub trait Transferable {}

impl<const SIZE: usize, T: Transferable> Transferable for [T; SIZE] {}
impl_for_tuples!(impl_for_tuple, Transferable);

#[cfg(all(target_family = "wasm", target_os = "unknown", feature = "message"))]
impl Transferable for ArrayBuffer {}
#[cfg(all(
	target_family = "wasm",
	target_os = "unknown",
	feature = "message",
	web_sys_unstable_apis
))]
#[cfg_attr(docsrs, doc(cfg(web_sys_unstable_apis)))]
impl Transferable for AudioData {}
#[cfg(all(target_family = "wasm", target_os = "unknown", feature = "message"))]
impl Transferable for ImageBitmap {}
#[cfg(all(target_family = "wasm", target_os = "unknown", feature = "message"))]
impl Transferable for MessagePort {}
#[cfg(all(target_family = "wasm", target_os = "unknown", feature = "message"))]
impl Transferable for OffscreenCanvas {}
#[cfg(all(target_family = "wasm", target_os = "unknown", feature = "message"))]
impl Transferable for ReadableStream {}
#[cfg(all(target_family = "wasm", target_os = "unknown", feature = "message"))]
impl Transferable for RtcDataChannel {}
#[cfg(all(target_family = "wasm", target_os = "unknown", feature = "message"))]
impl Transferable for TransformStream {}
#[cfg(all(
	target_family = "wasm",
	target_os = "unknown",
	feature = "message",
	web_sys_unstable_apis
))]
#[cfg_attr(docsrs, doc(cfg(web_sys_unstable_apis)))]
impl Transferable for VideoFrame {}
#[cfg(all(target_family = "wasm", target_os = "unknown", feature = "message"))]
impl Transferable for WritableStream {}

/// Wrapper that implements [`MessageSend`] for values implementing
/// [`Transferable`].
///
/// For a more complete documentation see
/// [`web::spawn_with_message()`](super::spawn_with_message).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferableWrapper<T>(pub T)
where
	T: Into<JsValue> + JsCast + Transferable;

impl<T: Into<JsValue> + JsCast + Transferable> From<T> for TransferableWrapper<T> {
	fn from(value: T) -> Self {
		Self(value)
	}
}

impl<T: Into<JsValue> + JsCast + Transferable> MessageSend for TransferableWrapper<T> {
	type Send = ();

	fn send<E: Extend<JsValue>>(self, transfer: &mut E) -> RawMessage<Self::Send> {
		let serialize = self.0.into();
		transfer.extend([serialize.clone()]);

		RawMessage {
			serialize: Some(serialize),
			send: None,
		}
	}

	fn receive(serialized: Option<JsValue>, sent: Option<Self::Send>) -> Self {
		debug_assert_eq!(sent, None, "unexpected `Send` value");

		Self(
			serialized
				.expect("expected serialized value")
				.unchecked_into(),
		)
	}
}

/// Wrapper that implements [`MessageSend`] for values implementing [`Send`].
///
/// # Example
///
/// ```
/// # #[cfg(all(target_feature = "atomics", not(unsupported_spawn)))]
/// # wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
/// # #[cfg_attr(all(target_feature = "atomics", not(unsupported_spawn)), wasm_bindgen_test::wasm_bindgen_test)]
/// # async fn test() {
/// use std::sync::Arc;
/// use web_thread::web::{self, JoinHandleExt};
/// use web_thread::web::message::SendWrapper;
///
/// let data = Arc::new(vec![0, 1, 2, 3, 4]);
/// let message = SendWrapper(Arc::clone(&data));
/// let mut handle = web::spawn_with_message(
/// 	|message| async move {
/// 		// Do work.
/// #       drop(message);
/// 	},
/// 	message,
/// );
///
/// // Do work.
/// # /*
/// data;
/// # */
/// # let _ =data;
///
/// handle.join_async().await.unwrap();
/// # }
/// # #[cfg(not(all(target_feature = "atomics", not(unsupported_spawn))))]
/// # let _ = test();
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SendWrapper<T>(pub T)
where
	T: Send;

impl<T: Send> From<T> for SendWrapper<T> {
	fn from(value: T) -> Self {
		Self(value)
	}
}

impl<T: Send> MessageSend for SendWrapper<T> {
	type Send = T;

	fn send<E: Extend<JsValue>>(self, _: &mut E) -> RawMessage<Self::Send> {
		RawMessage {
			serialize: None,
			send: Some(self.0),
		}
	}

	fn receive(serialized: Option<JsValue>, sent: Option<Self::Send>) -> Self {
		debug_assert_eq!(serialized, None, "unexpected serialized `JsValue`");

		Self(sent.expect("expected serialized value"))
	}
}

/// Helper type to minimize FFI calls when building [`Array`]s.
pub(crate) struct ArrayBuilder {
	/// The [`Array`].
	this: Option<Array>,
	/// Caching [`JsValue`]s.
	buffer: Buffer,
}

/// [`JsValue`] cache.
#[derive(Default)]
enum Buffer {
	/// No values.
	#[default]
	None,
	/// One value.
	One([JsValue; 1]),
	/// Two values.
	Two([JsValue; 2]),
	/// Three values.
	Three([JsValue; 3]),
	/// Four values.
	Four([JsValue; 4]),
}

impl ArrayBuilder {
	/// Creates a new [`ArrayBuilder`].
	pub(crate) const fn new() -> Self {
		Self {
			this: None,
			buffer: Buffer::None,
		}
	}

	/// Push a [`JsValue`] into the [`Array`].
	fn push(&mut self, value: JsValue) {
		self.buffer = match mem::take(&mut self.buffer) {
			Buffer::None => Buffer::One([value]),
			Buffer::One([value_1]) => Buffer::Two([value_1, value]),
			Buffer::Two([value_1, value_2]) => Buffer::Three([value_1, value_2, value]),
			Buffer::Three([value_1, value_2, value_3]) => {
				Buffer::Four([value_1, value_2, value_3, value])
			}
			Buffer::Four([value_1, value_2, value_3, value_4]) => {
				let new = Array::of5(&value_1, &value_2, &value_3, &value_4, &value);

				let new = if let Some(this) = self.this.take() {
					this.concat(&new)
				} else {
					new
				};

				self.this = Some(new);

				Buffer::None
			}
		};
	}

	/// Finish building the [`Array`]. Returns [`None`] if no [`JsValue`]s were
	/// supplied.
	pub(crate) fn finish(mut self) -> Option<Array> {
		Some(match self.buffer {
			Buffer::None => return self.this,
			Buffer::One([value_1]) => {
				if let Some(this) = self.this {
					this.push(&value_1);
					this
				} else {
					Array::of1(&value_1)
				}
			}
			Buffer::Two(values) => {
				if let Some(mut this) = self.this {
					this.extend(values);
					this
				} else {
					Array::of2(&values[0], &values[1])
				}
			}
			Buffer::Three([value_1, value_2, value_3]) => {
				let new = Array::of3(&value_1, &value_2, &value_3);

				if let Some(this) = self.this.take() {
					this.concat(&new)
				} else {
					new
				}
			}
			Buffer::Four([value_1, value_2, value_3, value_4]) => {
				let new = Array::of4(&value_1, &value_2, &value_3, &value_4);

				if let Some(this) = self.this.take() {
					this.concat(&new)
				} else {
					new
				}
			}
		})
	}
}

impl Extend<JsValue> for ArrayBuilder {
	fn extend<T: IntoIterator<Item = JsValue>>(&mut self, iter: T) {
		for value in iter {
			self.push(value);
		}
	}
}

#[doc(hidden)]
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
pub mod __internal {
	use wasm_bindgen::{JsCast, JsValue};

	use super::{
		MessageSend, RawMessage, SendWrapper, Serializable, SerializableWrapper, Transferable,
		TransferableWrapper,
	};

	macro_rules! impl_internal {
		($name:ident for $priority:ty, $wrapper:ident, $($bound:path)|+) => {
			pub trait $name<T> {
				type Send;

				fn __web_thread_send<E: Extend<JsValue>>(self, extend: &mut E) -> RawMessage<Self::Send>;

				fn __web_thread_receive(self, serialized: Option<JsValue>, sent: Option<Self::Send>) -> T;
			}

			#[allow(clippy::mut_mut)]
			impl<T: $($bound+)+> $name<T> for $priority {
				type Send = <$wrapper<T> as MessageSend>::Send;

				fn __web_thread_send<E: Extend<JsValue>>(self, extend: &mut E) -> RawMessage<Self::Send> {
					$wrapper(self.take().expect("found empty `Option` while sending")).send(extend)
				}

				fn __web_thread_receive(self, serialized: Option<JsValue>, sent: Option<Self::Send>) -> T {
					debug_assert!(self.is_none(), "found filled `Option` while receiving");
					$wrapper::receive(serialized, sent).0
				}
			}
		};
	}

	impl_internal!(InternalSerializable for &mut &mut Option<T>, SerializableWrapper, Serializable | Into<JsValue> | JsCast);
	impl_internal!(InternalTransferable for &mut Option<T>, TransferableWrapper, Transferable | Into<JsValue> | JsCast);
	impl_internal!(InternalSend for &mut Option<T>, SendWrapper, Send);
}

/// We assume that we aren't targeting `wasm64`.
fn usize_is_u32(value: usize) -> u32 {
	value.try_into().expect("found 64-bit Wasm")
}
