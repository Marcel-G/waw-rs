use darling::{ast, util::Ignored, FromDeriveInput, FromMeta, FromVariant};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Ident};

#[derive(Debug, FromMeta)]
enum AutomationRateParam {
    #[darling(rename = "a-rate")]
    ARate,
    #[darling(rename = "k-rate")]
    KRate,
}

#[derive(Debug, FromVariant)]
#[darling(attributes(param))]
struct ParamField {
    ident: Ident,
    automation_rate: Option<AutomationRateParam>,
    min_value: Option<f32>,
    max_value: Option<f32>,
    default_value: Option<f32>,
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(param), supports(enum_any))]
struct Input {
    ident: Ident,
    data: ast::Data<ParamField, Ignored>,
}

fn tokenize_option<T>(
    option: Option<T>,
    tokenize_inner: impl Fn(T) -> proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    match option {
        Some(value) => {
            let inner = tokenize_inner(value);
            quote! { Some(#inner) }
        }
        None => quote! { None },
    }
}

fn tokenize_descriptor(field: &ParamField) -> proc_macro2::TokenStream {
    let name = field.ident.to_string();
    let automation_rate = tokenize_option(field.automation_rate.as_ref(), |v| match v {
        AutomationRateParam::ARate => quote! { waw::types::AutomationRate::ARate },
        AutomationRateParam::KRate => quote! { waw::types::AutomationRate::KRate },
    });
    let min_value = tokenize_option(field.min_value, |v| quote! { #v });
    let max_value = tokenize_option(field.max_value, |v| quote! { #v });
    let default_value = tokenize_option(field.default_value, |v| quote! { #v });

    quote! {
        waw::types::AudioParamDescriptor {
            name: String::from(#name),
            automation_rate: #automation_rate,
            min_value: #min_value,
            max_value: #max_value,
            default_value: #default_value,
        }
    }
}

pub fn derive(input: TokenStream) -> TokenStream {
    let derive = parse_macro_input!(input as DeriveInput);

    let receiver = Input::from_derive_input(&derive).unwrap();

    let ident = receiver.ident;

    let descriptors: Vec<_> = receiver
        .data
        .take_enum()
        .unwrap()
        .iter()
        .map(tokenize_descriptor)
        .collect();

    let result = quote! {
      impl waw::types::ParameterDescriptor for #ident {
        fn descriptors() -> std::vec::Vec<waw::types::AudioParamDescriptor> {
            vec![ #(#descriptors,)* ]
        }
      }
    };

    result.into()
}
