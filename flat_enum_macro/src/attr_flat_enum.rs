use crate::util::generic_arg_to_type;
use proc_macro2::TokenStream;
use proc_macro_error::abort;
use syn::spanned::Spanned;
use syn::*;
use template_quote::quote;

fn attribute_is_flatten(attr: &Attribute) -> bool {
    &attr.style == &AttrStyle::Outer && attr.path.is_ident("flatten")
}

fn split_path_param(mut path: Path) -> (Path, Vec<Type>) {
    let v = if let Some(seg) = path.segments.last_mut() {
        let v = match &seg.arguments {
            PathArguments::AngleBracketed(args) => {
                args.args.iter().cloned().map(generic_arg_to_type).collect()
            }
            PathArguments::None => Vec::new(),
            _ => panic!(),
        };
        seg.arguments = PathArguments::None;
        v
    } else {
        Vec::new()
    };
    (path, v)
}

pub fn flat_enum(_attr: TokenStream, input: ItemEnum) -> TokenStream {
    let mut out = TokenStream::new();
    let mut first_macro_path = None;

    for variant in input.variants.iter() {
        if variant.attrs.iter().any(attribute_is_flatten) {
            match &variant.fields {
                Fields::Unnamed(field) if field.unnamed.len() == 1 => {
                    if field.unnamed[0].attrs.len() > 0 {
                        abort!(
                            field.unnamed[0].attrs[0].span(),
                            "No attribute is allowed in #[flatten] variant"
                        );
                    }
                    match &field.unnamed[0].ty {
                        Type::Path(tp) if tp.qself.is_none() => {
                            let (pat, tys) = split_path_param(tp.path.clone());
                            if first_macro_path.is_none() {
                                first_macro_path = Some(pat.clone());

                                out.extend(quote! {
                                    @ [ #{ &field.unnamed[0].ty }, #(#tys),* ]
                                });
                            } else {
                                out.extend(quote! {
                                    (#pat) [ #{ &field.unnamed[0].ty }, #(#tys),* ]
                                })
                            }
                        }
                        _ => abort!(field.unnamed[0].ty.span(), "Bad type in #[flatten] variant"),
                    }
                }
                _ => {
                    abort!(
                        variant.span(),
                        "enum variant with #[flatten] attribute must have one unnamed type."
                    )
                }
            }
        } else {
            out.extend(quote! {{#variant}})
        }
    }

    if let Some(mac) = first_macro_path {
        quote! {
            #mac!(
                @emit_enum
                #mac
                {
                    #(for attr in &input.attrs) { #attr }
                    #{ &input.vis }
                    #{ &input.enum_token }
                    #{ &input.ident }
                    <#{ &input.generics.params }>
                }
                []
                #out
            );
        }
    } else {
        quote! { #input }
    }
}
