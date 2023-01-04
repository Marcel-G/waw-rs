// @todo make a macro to implement this trait
// https://github.com/madonoharu/tsify/blob/87b77ef0f81ac25f4bdab79d5cdfbcb86f77f396/tsify-macros/src/wasm_bindgen.rs#L43, convert::FromWasmAbi

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

pub fn derive(input: TokenStream) -> TokenStream {
    let derive = parse_macro_input!(input as DeriveInput);

    let ident = derive.ident;

    // @todo -- Is the TS name always the same as the Rust class?
    let typescript_type = ident.to_string();

    let typescript_type_len = typescript_type.len() as u32;
    let typescript_type_chars = typescript_type.chars().map(|c| c as u32);

    quote! {
        use wasm_bindgen::describe::*;
        impl waw::utils::callback::RawHackDescribe for #ident {
            fn len() -> u32 { #typescript_type_len }
            fn raw_describe() { #(inform(#typescript_type_chars);)* }
        }
    }
    .into()
}
