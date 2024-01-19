mod attr_flat_enum;
mod derive_macro;
mod util;

use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use syn::{parse_macro_input, ItemEnum};

/// This macro derives [`flat_enum::FlatTarget`] trait.
#[proc_macro_error]
#[proc_macro_derive(FlatTarget)]
pub fn flat_target(input: TokenStream) -> TokenStream {
    derive_macro::flat_target(parse_macro_input!(input as ItemEnum)).into()
}

#[proc_macro_error]
#[proc_macro_attribute]
pub fn flat_enum(attr: TokenStream, input: TokenStream) -> TokenStream {
    attr_flat_enum::flat_enum(attr.into(), parse_macro_input!(input as ItemEnum)).into()
}