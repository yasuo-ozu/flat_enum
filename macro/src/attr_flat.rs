use derive_syn_parse::Parse;
use proc_macro2::TokenStream;
use proc_macro_error::abort;
use syn::spanned::Spanned;
use syn::*;
use template_quote::quote;

#[derive(Parse)]
pub struct MacroArg {
    structured_path: Path,
    _at_token: Option<Token![@]>,
    #[parse_if(_at_token.is_some())]
    krate: Option<Path>,
}

pub fn flat(arg: MacroArg, input: ItemEnum) -> TokenStream {
    let MacroArg {
        structured_path,
        krate,
        ..
    } = arg;
    let krate = krate.unwrap_or(parse_quote!(::flat_enum));
    let (g_impl, g_type, g_where) = input.generics.split_for_impl();
    let macro_name =
        if structured_path.leading_colon.is_none() && structured_path.segments.len() == 1 {
            &structured_path.segments[0].ident
        } else {
            abort!(
                structured_path.span(),
                "Should be exist in the same context"
            );
        };
    if input.variants.len() > 0 {
        abort!(input.span(), "Cannot specify variants");
    }
    quote! {
        unsafe impl #g_impl #krate::Flat for #{ &input.ident } #g_type #g_where {
            type Structured = #structured_path;
        }
        #macro_name!(
            @emit_enum
            flat_enum = #{env!("CARGO_PKG_VERSION")},
            enum_decl = {
                #(for attr in &input.attrs) { #attr }
                #{&input.vis}
                #{&input.enum_token}
                #{&input.ident}
                #g_type
            },
        );
    }
}
