use crate::util::{generic_arg_to_type, getrandom};
use derive_syn_parse::Parse;
use proc_macro2::{Span, TokenStream};
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

#[allow(unused)]
enum ParsedVariant {
    Normal(Variant),
    Flattened {
        attrs: Vec<Attribute>,
        ident: Ident,
        #[allow(unused)]
        paren_token: syn::token::Paren,
        ty: Type,
        macro_path: Path,
        discriminant: Option<(syn::token::Eq, Expr)>,
        arg_tys: Vec<Type>,
    },
}

impl ParsedVariant {
    fn new(variant: Variant) -> Self {
        if variant.attrs.iter().any(attribute_is_flatten) {
            match &variant.fields {
                Fields::Unnamed(field)
                    if field.unnamed.len() == 1 && field.unnamed[0].attrs.len() == 0 =>
                {
                    let paren_token = field.paren_token.clone();
                    let field = &field.unnamed[0];
                    match &field.ty {
                        Type::Path(tp) if tp.qself.is_none() => {
                            let (macro_path, arg_tys) = split_path_param(tp.path.clone());
                            return ParsedVariant::Flattened {
                                attrs: variant
                                    .attrs
                                    .into_iter()
                                    .filter(|attr| !attribute_is_flatten(attr))
                                    .collect(),
                                ident: variant.ident,
                                paren_token,
                                ty: field.ty.clone(),
                                macro_path,
                                discriminant: variant.discriminant.clone(),
                                arg_tys,
                            };
                        }
                        _ => (),
                    }
                }
                _ => (),
            }
            abort!(variant.span(), "Bad variant for #[flatten]")
        } else {
            ParsedVariant::Normal(variant)
        }
    }
}

fn serialize_variants(variants: &[ParsedVariant], is_pat: bool) -> (TokenStream, Option<&Path>) {
    let mut first_macro_path = None;
    let mut out = TokenStream::new();
    for variant in variants {
        match variant {
            ParsedVariant::Normal(variant) => {
                match (&variant.fields, is_pat) {
                    (Fields::Unnamed(_), true) => {
                        out.extend(quote!{{
                            #{&variant.ident} (
                                #(for i in 0..variant.fields.len()), {
                                    #{Ident::new(&format!("a{}", i), Span::call_site())}
                                }
                            )
                        }});
                    }
                    (Fields::Named(_), true) => {
                        out.extend(quote!{{
                            #{&variant.ident} {
                                #(for field in &variant.fields), {
                                    #{&field.ident}
                                }
                            }
                        }})
                    }
                    _ => {
                        out.extend(quote! {{#variant}});
                    }
                }
            }
            ParsedVariant::Flattened {
                ident,
                ty,
                macro_path,
                arg_tys,
                ..
            } => {
                if first_macro_path.is_none() {
                    first_macro_path = Some(macro_path);
                    out.extend(quote! { @ [ #ident, (#ty), #(#arg_tys),* ] });
                } else {
                    out.extend(quote! { (#macro_path) [ #ident, (#ty), #(#arg_tys),* ] })
                }
            }
        }
    }
    (out, first_macro_path)
}

fn emit_macro_inner(variants: &[ParsedVariant], enum_decl: &TokenStream) -> TokenStream {
    let (out, first_macro_path) = serialize_variants(variants, false);
    if let Some(mac) = first_macro_path {
        quote! {
            #mac!(
                @emit_enum
                #mac
                { #enum_decl }
                []
                #out
            );
        }
    } else {
        quote! {
            #enum_decl {
                #(for variant in variants) {
                    #(if let ParsedVariant::Normal(variant) = variant) {
                        #variant,
                    }
                }
            }
        }
    }
}

fn emit_macro(ident: &Ident, variants: &[ParsedVariant]) -> TokenStream {
    let random_module_ident = Ident::new(
        &format!("flat_enum_module_{:x}_{}", getrandom(), ident.to_string()),
        Span::call_site(),
    );
    let random_macro_ident = Ident::new(
        &format!(
            "flat_enum_macro_{:x}_{}",
            getrandom(),
            ident
        ),
        Span::call_site(),
    );
    let inner = emit_macro_inner(variants, &quote! { $($enum_decl)* });
    quote! {
        #[allow(non_snake_case)]
        mod #random_module_ident {
            #[macro_export]
            macro_rules! #random_macro_ident {
                (@emit_enum
                    flat_enum = #{env!("CARGO_PKG_VERSION")},
                    enum_decl = { $($enum_decl:tt)* },
                ) => { #inner };
            }
            #[allow(unused)]
            pub use #random_macro_ident as #ident;
        }
        #[allow(unused)]
        use #random_module_ident::*;
    }
}

fn emit_into_flat(ident_flat: &Ident, variants: &[ParsedVariant]) -> TokenStream {
    quote! {
        #(for variant in variants) {
            #(if let ParsedVariant::Normal(variant) = variant) {
                Self :: #{ &variant.ident }
                #(if let Fields::Named(fields) = &variant.fields) {
                    {
                        #(for field in &fields.named) {
                            #{&field.ident}
                        }
                    } => #ident_flat :: #{ &variant.ident } {
                        #(for field in &fields.named) {
                            #{&field.ident}
                        }
                    }
                }
                #(if let Fields::Unnamed(fields) = &variant.fields) {
                    #(let ids = (0..fields.unnamed.len()).map(|i| Ident::new(&format!("a{}", i), Span::call_site())).collect::<Vec<_>>()){
                        ( #(#ids),* ) => #ident_flat :: #{ &variant.ident } (#(#ids),*)
                    }
                }
                #(if let Fields::Unit = &variant.fields) {
                    => #ident_flat:: #{ &variant.ident }
                }
            }
            #(if let ParsedVariant::Flattened{ident, macro_path, ..} = variant) {
                Self :: #ident (item) => {
                    #macro_path ! (@emit_flat item, (#macro_path), #ident_flat)
                }
            },
        }
    }
}

fn emit_from_flat(ident_flat: &Ident, variants: &[ParsedVariant]) -> TokenStream {
    let (out, first_macro_path) = serialize_variants(variants, true);
    if let Some(mac) = first_macro_path {
        quote! {
            #mac!(
                @emit_unflat #mac []
                (this, #ident_flat, Self)
                #out
            );
        }
    } else {
        quote! {
            match this {
                #(for variant in variants) {
                    #(if let ParsedVariant::Normal(variant) = variant) {
                        #ident_flat :: #{ &variant.ident }
                        #(if let Fields::Named(fields) = &variant.fields) {
                            {
                                #(for field in &fields.named) {
                                    #{&field.ident}
                                }
                            } => Self :: #{ &variant.ident } {
                                #(for field in &fields.named) {
                                    #{&field.ident}
                                }
                            }
                        }
                        #(if let Fields::Unnamed(fields) = &variant.fields) {
                            #(let ids = (0..fields.unnamed.len()).map(|i| Ident::new(&format!("a{}", i), Span::call_site())).collect::<Vec<_>>()){
                                ( #(#ids),* ) => Self :: #{ &variant.ident } (#(#ids),*)
                            }
                        }
                        #(if let Fields::Unit = &variant.fields) {
                            => Self:: #{ &variant.ident }
                        }
                    },
                }
            }
        }
    }
}

#[derive(Parse)]
pub struct MacroArg {
    flat_path: Path,
    _at_token: Option<Token![@]>,
    #[parse_if(_at_token.is_some())]
    krate: Option<Path>,
}

pub fn into_flat(arg: MacroArg, mut input: ItemEnum) -> TokenStream {
    let MacroArg {
        flat_path, krate, ..
    } = arg;
    let krate = krate.unwrap_or(parse_quote!(::flat_enum));
    let (g_impl, g_type, g_where) = input.generics.split_for_impl();
    let flat_name = if flat_path.leading_colon.is_none() && flat_path.segments.len() == 1 {
        &flat_path.segments[0].ident
    } else {
        abort!(flat_path.span(), "Should be exist in the same context");
    };
    let variants: Vec<_> = input
        .variants
        .iter()
        .cloned()
        .map(ParsedVariant::new)
        .collect();
    let mac_def = emit_macro(&input.ident, &variants[..]);
    let fn_into_flat = emit_into_flat(&flat_name, &variants[..]);
    let fn_from_flat = emit_from_flat(&flat_name, &variants[..]);
    input.variants.iter_mut().for_each(|variant| {
        let attrs = variant.attrs.clone();
        variant.attrs = attrs
            .into_iter()
            .filter(|attr| !attr.path.is_ident("flatten"))
            .collect();
    });
    quote! {
        #input
        #mac_def
        #[automatically_derived]
        unsafe impl #g_impl #krate::IntoFlat for #{&input.ident} #g_type #g_where {
            type Flat = #flat_path;
            fn into_flat(self) -> Self::Flat {
                match self {
                    #fn_into_flat
                }
            }

            fn from_flat(this: Self::Flat) -> Self {
                #fn_from_flat
            }
        }
    }
}
