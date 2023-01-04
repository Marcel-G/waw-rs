use proc_macro::TokenStream;
mod module;
mod parameter_descriptors;
mod raw_describe;
use proc_macro2::{Ident as ProcIdent, Span};
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(ParameterDescriptor, attributes(param))]
pub fn derive_parameters(input: TokenStream) -> TokenStream {
    parameter_descriptors::derive(input)
}

#[proc_macro]
pub fn module(input: TokenStream) -> TokenStream {
    module::module(input)
}

#[proc_macro_derive(RawHackDescribe)]
pub fn derive_raw_describe(input: TokenStream) -> TokenStream {
    raw_describe::derive(input)
}

/// Adds necessary implementations for event enums
#[proc_macro_attribute]
pub fn derive_event(_metadata: TokenStream, input: TokenStream) -> proc_macro::TokenStream {
    let derive = parse_macro_input!(input as DeriveInput);
    let ident = derive.ident.clone();
    let module = ProcIdent::new(&format!("_mod_{ident}"), Span::call_site());

    quote! {
        mod #module {
            use super::*;
            use waw::tsify as tsify;
            use waw::tsify::Tsify;

            #[derive(
                waw::serde::Serialize,
                waw::serde::Deserialize,
                waw::tsify::Tsify,
                waw::derive::RawHackDescribe,
            )]
            #[tsify(into_wasm_abi, from_wasm_abi)]
            #[serde(crate = "waw::serde")]
            #derive

            impl From<JsValue> for #ident {
                fn from(value: JsValue) -> Self {
                    Self::from_js(value).unwrap()
                }
            }

            impl From<#ident> for JsValue {
                fn from(val: #ident) -> Self {
                    val.into_js().unwrap().into()
                }
            }
        }

        use #module::#ident;
    }
    .into()
}

/// Adds necessary implementations for command enums
#[proc_macro_attribute]
pub fn derive_command(_metadata: TokenStream, input: TokenStream) -> proc_macro::TokenStream {
    let derive = parse_macro_input!(input as DeriveInput);
    let ident = derive.ident.clone();
    let module = ProcIdent::new(&format!("_mod_{ident}"), Span::call_site());

    quote! {
        mod #module {
            use super::*;
            use waw::tsify as tsify;
            use waw::tsify::Tsify;

            #[derive(waw::serde::Serialize, waw::serde::Deserialize, waw::tsify::Tsify, Clone)]
            #[tsify(into_wasm_abi, from_wasm_abi)]
            #[serde(crate = "waw::serde")]
            #derive

            impl From<JsValue> for #ident {
                fn from(value: JsValue) -> Self {
                    Self::from_js(value).unwrap()
                }
            }

            impl From<#ident> for JsValue {
                fn from(val: #ident) -> Self {
                    val.into_js().unwrap().into()
                }
            }
        }

        use #module::#ident;
    }
    .into()
}

/// Adds necessary implementations for parameter enums
#[proc_macro_attribute]
pub fn derive_param(_metadata: TokenStream, input: TokenStream) -> proc_macro::TokenStream {
    let derive = parse_macro_input!(input as DeriveInput);
    let ident = derive.ident.clone();
    let module = ProcIdent::new(&format!("_mod_{ident}"), Span::call_site());

    quote! {
        mod #module {
            use super::*;
            use waw::tsify as tsify;
            use waw::tsify::Tsify;

            #[derive(
                waw::serde::Serialize,
                waw::serde::Deserialize,
                waw::tsify::Tsify,
                waw::enum_map::Enum,
                waw::derive::ParameterDescriptor,
                Debug,
            )]
            #[tsify(into_wasm_abi, from_wasm_abi)]
            #[serde(crate = "waw::serde")]
            #derive

            impl From<JsValue> for #ident {
                fn from(value: JsValue) -> Self {
                    Self::from_js(value).unwrap()
                }
            }

            impl From<#ident> for JsValue {
                fn from(val: #ident) -> Self {
                    val.into_js().unwrap().into()
                }
            }
        }
        use #module::#ident;
    }
    .into()
}
