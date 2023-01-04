use proc_macro::TokenStream;
use proc_macro2::{Ident as ProcIdent, Span};
use quote::quote;
use syn::{parse_macro_input, Ident};

fn processor(ident: &Ident) -> proc_macro2::TokenStream {
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

fn node(ident: &Ident) -> proc_macro2::TokenStream {
    let node_ident = ProcIdent::new(&format!("{ident}Node"), Span::call_site());

    quote! {
      #[wasm_bindgen(js_name = #ident)]
      pub struct #node_ident (waw::node::Node<#ident>);

      #[wasm_bindgen(js_class = #ident)]
      impl #node_ident {
          pub async fn install(ctx: waw::web_sys::AudioContext) -> Result<#node_ident, wasm_bindgen::JsValue> {
              let result = waw::node::Node::<#ident>::install(ctx).await?;
              Ok(#node_ident(result))
          }
          pub fn node(&self) -> Result<waw::web_sys::AudioWorkletNode, wasm_bindgen::JsValue> {
            Ok(self.0.inner.clone())
          }

          pub fn get_param(&self, param: <#ident as waw::worklet::AudioModule>::Param) -> waw::web_sys::AudioParam {
            self.0.get_param(param)
          }

          pub fn command(&self, message: <#ident as waw::worklet::AudioModule>::Command) {
              self.0.command(message)
          }

          pub fn subscribe(&mut self, callback: waw::utils::callback::Callback<<#ident as waw::worklet::AudioModule>::Event>) {
              self.0.subscribe(callback.0)
          }

          pub fn destroy(&mut self) {
            self.0.destroy();
          }
      }
    }
}

pub fn module(item: TokenStream) -> TokenStream {
    let ident = parse_macro_input!(item as Ident);

    let processor = processor(&ident);
    // Build the bound node struct
    let node = node(&ident);

    quote! {
      #processor
      #node
    }
    .into()
}
