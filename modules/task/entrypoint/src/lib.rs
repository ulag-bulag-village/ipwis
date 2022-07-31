#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemFn};

mod entrypoint;

#[proc_macro_attribute]
pub fn entrypoint(_attribute: TokenStream, input: TokenStream) -> TokenStream {
    // let mut attribute = parse_macro_input!(attribute as Attributes);
    let input = parse_macro_input!(input as ItemFn);
    self::entrypoint::expand_attribute(input)
        .unwrap_or_else(to_compile_errors)
        .into()
}

fn to_compile_errors(errors: Vec<syn::Error>) -> proc_macro2::TokenStream {
    let compile_errors = errors.iter().map(syn::Error::to_compile_error);
    quote!(#(#compile_errors)*)
}
