mod attr_flat;
mod attr_into_flat;
mod derive_macro;
mod util;

use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use syn::{parse_macro_input, ItemEnum};

#[proc_macro_error]
#[proc_macro_derive(FlatTarget, attributes(flat_enum))]
pub fn flat_target(input: TokenStream) -> TokenStream {
    derive_macro::flat_target(parse_macro_input!(input as ItemEnum)).into()
}

#[proc_macro_error]
#[proc_macro_attribute]
pub fn into_flat(attr: TokenStream, input: TokenStream) -> TokenStream {
    let ret = attr_into_flat::into_flat(
        parse_macro_input!(attr),
        parse_macro_input!(input as ItemEnum),
    );
    ret
    .into()
}

#[proc_macro_error]
#[proc_macro_attribute]
pub fn flat(attr: TokenStream, input: TokenStream) -> TokenStream {
    let ret = attr_flat::flat(
        parse_macro_input!(attr),
        parse_macro_input!(input as ItemEnum),
    );
    ret
    .into()
}
