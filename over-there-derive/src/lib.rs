extern crate proc_macro;

use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Error)]
pub fn derive_error(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    // TODO: Support parsing attribute adding a display message
    let expanded = quote! {
        impl ::std::fmt::Display for #name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                match &*self {
                    x => write!(f, "{:?}", x),
                }
            }
        }
        impl ::std::error::Error for #name {}
    };

    proc_macro::TokenStream::from(expanded)
}
