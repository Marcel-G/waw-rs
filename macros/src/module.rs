use proc_macro::TokenStream;
use proc_macro2::{Ident as ProcIdent, Span};
use quote::quote;
use syn::{parse_macro_input, Ident};

fn processor(ident: &Ident) -> proc_macro2::TokenStream {
    let worklet_ident = ProcIdent::new(&format!("{ident}Processor"), Span::call_site());

    let worklet_ident_name = worklet_ident.to_string();

    quote! {
      use wasm_bindgen::prelude::*;

      #[wasm_bindgen]
      pub struct #worklet_ident (wasm_worklet::worklet::Processor<#ident>);

      #[wasm_bindgen]
      impl #worklet_ident {
        #[wasm_bindgen(constructor)]
        pub fn new(js_processor: wasm_worklet::web_sys::AudioWorkletProcessor) -> Self {
          #worklet_ident (wasm_worklet::worklet::Processor::new(
            #ident::create(),
            js_processor
          ))
        }

        pub fn connect(&mut self) {
          self.0.connect();
        }

        pub fn process(
          &mut self,
          input: &wasm_worklet::js_sys::Array,
          output: &wasm_worklet::js_sys::Array,
          params: &wasm_bindgen::JsValue
        ) -> bool {
          self.0.process(input, output, params)
        }
      }

      impl wasm_worklet::types::AudioModuleDescriptor for #ident {
        fn processor_name() -> &'static str {
            &#worklet_ident_name
        }

        fn parameter_descriptor_json() -> String {
          wasm_worklet::serde_json::to_string(
            &<<#ident as wasm_worklet::types::AudioModule>::Param as wasm_worklet::types::ParameterDescriptor>::descriptors()
          ).unwrap()
        }
      }
    }
}

fn node(ident: &Ident) -> proc_macro2::TokenStream {
    let node_ident = ProcIdent::new(&format!("{ident}Node"), Span::call_site());

    quote! {
      #[wasm_bindgen]
      pub struct #node_ident (wasm_worklet::node::Node<#ident>);

      #[wasm_bindgen]
      impl #node_ident {
          pub async fn install(ctx: wasm_worklet::web_sys::AudioContext) -> Result<#node_ident, wasm_bindgen::JsValue> {
              let result = wasm_worklet::node::Node::<#ident>::install(ctx).await?;
              Ok(#node_ident(result))
          }
          pub fn node(&self) -> Result<wasm_worklet::web_sys::AudioWorkletNode, wasm_bindgen::JsValue> {
            Ok(self.0.inner.clone())
          }

          pub fn get_param(&self, param: <#ident as wasm_worklet::types::AudioModule>::Param) -> wasm_worklet::web_sys::AudioParam {
            self.0.get_param(param)
          }

          pub fn command(&self, message: <#ident as wasm_worklet::types::AudioModule>::Command) {
              self.0.command(message)
          }

          pub fn subscribe(&mut self, callback: wasm_worklet::utils::callback::Callback<<#ident as wasm_worklet::types::AudioModule>::Event>) {
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
