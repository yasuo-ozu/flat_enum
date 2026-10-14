//! `#[derive(Helper)]` emits nothing. It only owns the helper attribute `#[helper(..)]`, like
//! `#[derive(serde::Serialize)]` owns `#[serde(..)]`, so that a test can write one on a field.

use proc_macro::TokenStream;

#[proc_macro_derive(Helper, attributes(helper))]
pub fn helper(_: TokenStream) -> TokenStream {
    TokenStream::new()
}
