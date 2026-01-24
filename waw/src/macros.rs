/// Registers an audio processor with the Web Audio API.
///
/// This macro generates the necessary boilerplate code to register a processor type
/// with the AudioWorkletGlobalScope and provides a convenient `create_node` method
/// for instantiating the processor in an AudioContext.
///
/// # Arguments
///
/// * `$processor` - The type implementing the `Processor` trait
/// * `$name` - A string literal identifying the processor (must be unique)
///
/// # Example
///
/// ```ignore
/// use waw::{register, Processor};
///
/// struct MyProcessor;
/// impl Processor for MyProcessor {
///     // ... implementation
/// }
///
/// register!(MyProcessor, "my-processor");
/// ```
#[macro_export]
macro_rules! register {
    ($processor:ty, $name:literal) => {
        // Create the registration function
        fn register_processor() -> Result<(), $crate::wasm_bindgen::JsValue> {
            use $crate::wasm_bindgen::JsCast;
            use $crate::waw_thread::AudioWorkletGlobalScopeExt;

            let global: $crate::web_sys::AudioWorkletGlobalScope =
                $crate::js_sys::global().unchecked_into();
            global
                .register_processor_ext::<$crate::ProcessorWrapper<$processor>>($name)
                .map_err(|e| $crate::wasm_bindgen::JsValue::from_str(&format!("{:?}", e)))
        }

        $crate::inventory::submit! {
            $crate::registry::ProcessorRegistration::new($name, register_processor)
        }

        impl $processor {
            /// Create a new audio worklet node for this processor
            pub fn create_node(
                ctx: &$crate::web_sys::AudioContext,
                data: <$processor as $crate::Processor>::Data,
                options: Option<&$crate::web_sys::AudioWorkletNodeOptions>,
            ) -> Result<$crate::AudioWorkletNodeWrapper, $crate::wasm_bindgen::JsValue> {
                $crate::create_node::<$processor>(ctx, $name, data, options)
            }
        }
    };
}
