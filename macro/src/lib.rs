use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

#[proc_macro_derive(Fieldnames)]
pub fn derive_fieldnames(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields_named) => fields_named
                .named
                .iter()
                .map(|f| {
                    let field_name = f.ident.as_ref().unwrap().to_string();
                    quote! { #field_name }
                })
                .collect::<Vec<_>>(),
            _ => {
                return syn::Error::new_spanned(
                    &input,
                    "Fieldnames can only be derived for structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(&input, "Fieldnames can only be derived for structs")
                .to_compile_error()
                .into();
        }
    };

    let expanded = quote! {
        impl Fieldnames for #name {
            fn field_names() -> &'static [&'static str] {
                &[#(#fields),*]
            }
        }
    };

    TokenStream::from(expanded)
}
