#[macro_export]
macro_rules! register {
    ($processor:ty, $name:literal) => {
        // Create the registration function
        fn register_processor() -> Result<(), $crate::wasm_bindgen::JsValue> {
            use $crate::wasm_bindgen::JsCast;
            use $crate::web_thread::web::audio_worklet::AudioWorkletGlobalScopeExt;

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
            ) -> Result<$crate::web_sys::AudioWorkletNode, $crate::wasm_bindgen::JsValue> {
                $crate::create_node::<$processor>(ctx, $name, data)
            }
        }
    };
}
