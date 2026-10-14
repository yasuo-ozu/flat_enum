mod attr_flat;
mod attr_into_flat;
mod subst;
mod util;

use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemEnum};

#[proc_macro_attribute]
pub fn into_flat(attr: TokenStream, input: TokenStream) -> TokenStream {
    attr_into_flat::into_flat(
        parse_macro_input!(attr),
        parse_macro_input!(input as ItemEnum),
    )
    .unwrap_or_else(|e| e.to_compile_error())
    .into()
}

#[proc_macro_attribute]
pub fn flat(attr: TokenStream, input: TokenStream) -> TokenStream {
    attr_flat::flat(
        parse_macro_input!(attr),
        parse_macro_input!(input as ItemEnum),
    )
    .unwrap_or_else(|e| e.to_compile_error())
    .into()
}

announce::helper::callback! {
    /// announce callback of `#[flat]`.
    #[doc(hidden)]
    pub fn __flat_cb(input) { attr_flat::flat_cb(input) }
}

announce::helper::callback! {
    /// announce callback of `#[into_flat]`.
    #[doc(hidden)]
    pub fn __into_flat_cb(input) { attr_into_flat::into_flat_cb(input) }
}
