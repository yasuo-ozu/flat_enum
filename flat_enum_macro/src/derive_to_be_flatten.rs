use crate::util::{
    generic_arg_to_type, generics_remove_defaults, generics_to_arguments, getrandom,
};
use proc_macro2::{Span, TokenStream};
use proc_macro_error::abort;
use std::collections::HashMap;
use syn::spanned::Spanned;
use syn::*;
use template_quote::quote;

fn generate_type_num(n: usize, span: &Span) -> Type {
    if n > 0 {
        let acc = generate_type_num(n - 1, span);
        syn::parse(quote! { (#acc,) }.into()).expect("Cannot parse to type")
    } else {
        syn::parse(quote! { () }.into()).expect("Cannot parse to type")
    }
}

fn emit_macro(
    input: &ItemEnum,
    macro_ident: &Ident,
    flat_enum: &Path,
    leak_dict: &HashMap<Type, usize>,
) -> TokenStream {
    use Fields::*;
    quote! {
        #[macro_export]
        macro_rules! #macro_ident {
            (@emit_enum $self:path { $($enum_decl:tt)* } [ $($out:tt)* ]) => {
                $($enum_decl)* { $($out)* }
            };
            (@emit_enum $self:path { $($enum_decl:tt)* } [ $($out:tt)* ] { $($raw:tt)* } $($t:tt)*) => {
                $self!(@emit_enum $self {$($enum_decl)*} [$($out)* $($raw)*]  $($t)*);
            };
            (@emit_enum $self:path { $($enum_decl:tt)* } [ $($out:tt)* ] ($mac:path) [$($marg:tt)*] $($t:tt)*) => {
                $mac!(@emit_enum $self {$($enum_decl)*} [$($out)*] @[$($marg)*] $($t)*);
            };
            (@emit_enum $self:path { $($enum_decl:tt)* } [ $($out:tt)* ] @[$typ:ty, $($enum_type_params:ty),* $(,)?] $($t:tt)*) => {
                $self!(
                    @emit_enum $self { $($enum_decl)* } [
                        $($out)*
                        #(for variant in &input.variants) {
                            #{ &variant.ident }
                            #(if let Named(fields) = &variant.fields) {
                                {
                                    #(for field in &fields.named) {
                                        #(for attrs in &field.attrs) { #{attrs} }
                                        #{&field.vis}
                                        #{field.ident.as_ref().unwrap()}
                                        #{field.colon_token.as_ref().unwrap()}
                                        <
                                            $typ
                                            as #flat_enum::Leak<
                                                ($($enum_type_params,)*),
                                                #{ leak_dict.get(&field.ty).unwrap() }
                                            >
                                        >::Ty,
                                    }
                                }
                            }
                            #(if let Unnamed(fields) = &variant.fields) {
                                (
                                    #(for field in &fields.unnamed), {
                                        #(for attrs in &field.attrs) { #{attrs} }
                                        #{&field.vis}

                                        <
                                            $typ
                                            as #flat_enum::Leak<
                                                ($($enum_type_params,)*),
                                                #{ generate_type_num(*leak_dict.get(&field.ty).unwrap(), &Span::call_site()) }
                                            >
                                        >::Ty
                                    }
                                )
                            },
                        }
                    ]
                    $($t)*
                );
            };
        }
    }
}

fn emit_macro_export_in_macro_namespace(
    input: &ItemEnum,
    flat_enum: &Path,
    leak_dict: &HashMap<Type, usize>,
) -> TokenStream {
    let random_module_ident = Ident::new(
        &format!(
            "flat_enum_module_{:x}_{}",
            getrandom(),
            input.ident.to_string()
        ),
        Span::call_site(),
    );
    let random_macro_ident = Ident::new(
        &format!(
            "flat_enum_macro_{:x}_{}",
            getrandom(),
            input.ident.to_string()
        ),
        Span::call_site(),
    );
    quote! {
        mod #random_module_ident {
            #{emit_macro(input, &random_macro_ident, flat_enum, leak_dict)}
            pub use #random_macro_ident as #{ &input.ident };
        }
        #{ &input.vis } use #random_module_ident::*;
    }
}

fn emit_impl(input: &ItemEnum, flat_enum: &Path, leak_dict: &HashMap<Type, usize>) -> TokenStream {
    let generic_impl = generics_remove_defaults(&input.generics);
    let arg = generics_to_arguments(&input.generics);
    let arg_items = if let PathArguments::AngleBracketed(abga) = &arg {
        abga.args
            .iter()
            .cloned()
            .map(generic_arg_to_type)
            .collect::<Vec<_>>()
    } else {
        panic!()
    };
    quote! {
        #[automatically_derived]
        unsafe impl <#{ &generic_impl.params }> #flat_enum :: ToBeFlatten for #{ &input.ident } #arg
        #{ &generic_impl.where_clause }
        { }

        #(for (ty, n) in leak_dict.iter()) {
            #[automatically_derived]
            unsafe impl <#{ &generic_impl.params }> #flat_enum :: Leak <
                (#(#arg_items,)*),
                #{ generate_type_num(*n, &ty.span()) }
            > for #{ &input.ident } #arg
            #{ &generic_impl.where_clause }
            {
                type Ty = #{ty};
            }
        }
    }
}

fn generate_leak_dict(input: &ItemEnum) -> HashMap<Type, usize> {
    let mut ret = HashMap::new();
    let mut num = 0;
    for variant in input.variants.iter() {
        for field in variant.fields.iter() {
            ret.entry(field.ty.clone()).or_insert_with(|| {
                num += 1;
                num - 1
            });
        }
    }
    ret
}

pub fn to_be_flatten(input: ItemEnum) -> TokenStream {
    let mut flat_enum: Path = parse_quote!(::flat_enum);
    for attr in &input.attrs {
        if attr.path.is_ident("flat_enum") {
            flat_enum = match attr.parse_args() {
                Ok(v) => v,
                Err(_) => abort!(
                    &attr.bracket_token.span,
                    "Only path item is acceptable in #[flat_enum(_)]"
                ),
            };
        }
    }
    let leak_dict = generate_leak_dict(&input);
    quote! {
        #{emit_impl(&input, &flat_enum, &leak_dict)}
        #{emit_macro_export_in_macro_namespace(&input, &flat_enum, &leak_dict)}
    }
}