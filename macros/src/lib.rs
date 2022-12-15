use proc_macro::TokenStream;
mod module;
mod parameter_descriptors;
mod raw_describe;

#[proc_macro_derive(Param, attributes(param))]
pub fn derive_parameters(input: TokenStream) -> TokenStream {
    parameter_descriptors::derive(input)
}

#[proc_macro]
pub fn module(input: TokenStream) -> TokenStream {
    module::module(input)
}

#[proc_macro_derive(RawDescribe)]
pub fn derive_raw_describe(input: TokenStream) -> TokenStream {
    raw_describe::derive(input)
}
