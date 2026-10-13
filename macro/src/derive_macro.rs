use crate::util::{
    announce_wrapped, generic_arg_to_type, generics_remove_defaults, generics_to_arguments,
    leak_fields,
};
use proc_macro2::TokenStream;
use syn::*;
use template_quote::quote;

/// The definition side: the derive announces its own input without re-emitting
/// it (`emit_item = false`), which gives the trampoline macro `E!`.
fn emit_trampoline(input: &ItemEnum, flat_enum: &Path) -> TokenStream {
    let mut copy = input.clone();
    copy.attrs.retain(|a| !a.path().is_ident("flat_enum"));
    announce_wrapped(flat_enum, TokenStream::new(), &copy)
}

fn emit_impl(input: &ItemEnum, flat_enum: &Path) -> Result<TokenStream> {
    let generic_impl = generics_remove_defaults(&input.generics);
    let arg = generics_to_arguments(&input.generics);
    let arg_items = if let PathArguments::AngleBracketed(abga) = &arg {
        abga.args
            .iter()
            .cloned()
            .map(generic_arg_to_type)
            .collect::<Result<Vec<_>>>()?
    } else {
        unreachable!()
    };
    Ok(quote! {
        #[automatically_derived]
        unsafe impl <#{ &generic_impl.params }> #flat_enum :: FlatTarget for #{ &input.ident } #arg
        #{ &generic_impl.where_clause }
        { }

        #(for (n, ty) in leak_fields(input).map(|f| &f.ty).enumerate()) {
            #[automatically_derived]
            unsafe impl <#{ &generic_impl.params }> #flat_enum :: Leak <
                {#n}, (#(#arg_items,)*),
            > for #{ &input.ident } #arg
            #{ &generic_impl.where_clause }
            {
                type Ty = #{ty};
            }
        }
    })
}

pub fn flat_target(input: ItemEnum) -> Result<TokenStream> {
    let mut flat_enum: Path = parse_quote!(::flat_enum);
    for attr in &input.attrs {
        if attr.path().is_ident("flat_enum") {
            flat_enum = attr.parse_args().map_err(|_| {
                Error::new_spanned(attr, "Only path item is acceptable in #[flat_enum(_)]")
            })?;
        }
    }
    Ok(quote! {
        #{emit_impl(&input, &flat_enum)?}
        #{emit_trampoline(&input, &flat_enum)}
    })
}
