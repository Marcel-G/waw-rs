use proc_macro2::{Ident as ProcIdent, Span};
use quote::quote;
use syn::{Ident};

// Generates the worklet wrapper to bind with JS in the audio thread
pub fn worklet_wrapper(ident: &Ident) -> proc_macro2::TokenStream {
    let worklet_ident = ProcIdent::new(&format!("_{ident}Processor"), Span::call_site());

    let worklet_ident_name = worklet_ident.to_string();

    quote! {
      use wasm_bindgen::prelude::*;

      #[wasm_bindgen]
      pub struct #worklet_ident (waw::worklet::Processor<#ident>);

      #[wasm_bindgen]
      impl #worklet_ident {
        #[wasm_bindgen(constructor)]
        pub fn new(js_processor: waw::web_sys::AudioWorkletProcessor) -> Self {
          let emitter = waw::worklet::Emitter::<
            <#ident as waw::worklet::AudioModule>::Event
          >::new(js_processor.port().unwrap());
          #worklet_ident (waw::worklet::Processor::new(
            #ident::create(emitter),
            js_processor
          ))
        }

        pub fn connect(&mut self) {
          self.0.connect();
        }

        pub fn process(
          &mut self,
          input: &waw::js_sys::Array,
          output: &waw::js_sys::Array,
          params: &wasm_bindgen::JsValue
        ) -> bool {
          self.0.process(input, output, params)
        }
      }

      impl waw::types::AudioModuleDescriptor for #ident {
        fn processor_name() -> &'static str {
            &#worklet_ident_name
        }

        fn parameter_descriptor_json() -> String {
          waw::serde_json::to_string(
            &<<#ident as waw::worklet::AudioModule>::Param as waw::types::ParameterDescriptor>::descriptors()
          ).unwrap()
        }
      }
    }
}
