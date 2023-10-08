use crate::util::{generic_arg_to_type, generics_to_arguments, getrandom};
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

pub fn flat_enum(_attr: TokenStream, input: ItemEnum) -> TokenStream {
    let mut out = TokenStream::new();
    let mut first_macro_path = None;
    let krate: Type = parse_quote!(::flat_enum);
    let variants: Vec<_> = input
        .variants
        .iter()
        .cloned()
        .map(ParsedVariant::new)
        .collect();
    for variant in &variants {
        match variant {
            ParsedVariant::Normal(variant) => out.extend(quote! {{#variant}}),
            ParsedVariant::Flattened {
                ty,
                macro_path,
                arg_tys,
                ..
            } => {
                if first_macro_path.is_none() {
                    first_macro_path = Some(macro_path.clone());
                    out.extend(quote! { @ [ #ty, #(#arg_tys),* ] });
                } else {
                    out.extend(quote! { (#macro_path) [ #ty, #(#arg_tys),* ] })
                }
            }
        }
    }
    let args = match generics_to_arguments(&input.generics) {
        PathArguments::None => Vec::new(),
        PathArguments::AngleBracketed(abga) => abga.args.into_iter().collect(),
        _ => unreachable!(),
    };
    let random = getrandom();
    let ident_unflat = Ident::new(
        &format!("FlattenEnum_Unflat_{}_{}", &input.ident.to_string(), random),
        input.ident.span(),
    );
    let ident_unflat_ref = Ident::new(
        &format!(
            "FlattenEnum_UnflatRef_{}_{}",
            &input.ident.to_string(),
            random
        ),
        input.ident.span(),
    );
    let ident_unflat_mut = Ident::new(
        &format!(
            "FlattenEnum_UnflatMut_{}_{}",
            &input.ident.to_string(),
            random
        ),
        input.ident.span(),
    );
    let lt = Lifetime::new("'__lt_for_ref", Span::call_site());

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
            #[allow(non_camel_case_types)]
            #{ &input.vis } #{ &input.enum_token } #ident_unflat <#{ &input.generics.params }> {
                #(for variant in variants.iter()) {
                    #(if let ParsedVariant::Normal(variant) = variant) { #variant }
                    #(if let ParsedVariant::Flattened{attrs, ident, ty, discriminant, ..} = variant) {
                        #(#attrs)* #ident (#ty)
                        #(if let Some((eq, expr)) = discriminant) { #eq #expr }
                    },
                }
            }
            #[allow(non_camel_case_types)]
            #{ &input.vis } #{ &input.enum_token } #ident_unflat_ref <#lt, #{ &input.generics.params }> {
                #(for variant in variants.iter()) {
                    #(if let ParsedVariant::Normal(variant) = variant) { #variant }
                    #(if let ParsedVariant::Flattened{attrs, ident, ty, discriminant, ..} = variant) {
                        #(#attrs)* #ident (&#lt #ty)
                        #(if let Some((eq, expr)) = discriminant) { #eq #expr }
                    },
                }
            }
            #[allow(non_camel_case_types)]
            #{ &input.vis } #{ &input.enum_token } #ident_unflat_mut <#lt, #{ &input.generics.params }> {
                #(for variant in variants.iter()) {
                    #(if let ParsedVariant::Normal(variant) = variant) { #variant }
                    #(if let ParsedVariant::Flattened{attrs, ident, ty, discriminant, ..} = variant) {
                        #(#attrs)* #ident (&#lt mut #ty)
                        #(if let Some((eq, expr)) = discriminant) { #eq #expr }
                    },
                }
            }
            unsafe impl <#{ &input.generics.params }> #krate::FlattenEnum for #{ &input.ident } <#(#args),*>
            #{ &input.generics.where_clause }
            {
                type Unflat = #ident_unflat <#(#args),*>;
                type UnflatRef<#lt>
                where
                    Self: #lt =  #ident_unflat_ref <#lt #(,#args)*>;
                type UnflatMut<#lt>
                where
                    Self: #lt = #ident_unflat_mut <#lt #(,#args)*>;

                fn flat(this: Self::Unflat) -> Self {
                    match this {
                        #(for variant in variants.iter()) {
                            #(if let ParsedVariant::Normal(variant) = variant) {
                                #ident_unflat :: #{ &variant.ident }
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
                            }
                            #(if let ParsedVariant::Flattened{ident, macro_path, ..} = variant) {
                                #ident_unflat :: #ident (item) => {
                                    #macro_path ! (@emit_flat item, #ident_unflat, Self);
                                }
                            },
                        }
                    }
                }
                fn unflat(self) -> Self::Unflat { todo!() }
                fn unflat_ref<#lt>(&#lt self) -> Self::UnflatRef<#lt>
                where
                    Self: #lt { todo!() }
                fn unflat_mut<#lt>(&#lt self) -> Self::UnflatMut<#lt>
                where
                    Self: #lt { todo!() }
            }
        }
    } else {
        quote! {
            #input

            impl <#{ &input.generics.params }> #krate::FlattenEnum for #{ &input.ident } <#(#args),*>
            #{ &input.generics.where_clause }
            {
                type Unflat = #{ &input.ident } <#(#args),*>;
                type UnflatRef<#lt>
                where
                    Self: #lt =  #{ &input.ident } <#(#args),*>;
                type UnflatMut<#lt>
                where
                    Self: #lt = #{ &input.ident } <#(#args),*>;

                fn flat(this: Self::Unflat) -> Self { this }
                fn unflat(self) -> Self::Unflat { self }
                fn unflat_ref<#lt>(&#lt self) -> Self::UnflatRef<#lt>
                where
                    Self: #lt { self }
                fn unflat_mut<#lt>(&#lt self) -> Self::UnflatMut<#lt>
                where
                    Self: #lt { self }
            }
        }
    }
}
