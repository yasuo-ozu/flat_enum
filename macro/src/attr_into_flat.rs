use crate::attr_flat::flattened_idents;
use crate::util::{
    announce_wrapped, bindings, enum_of, has_flatten, ident_to_path, nested, parse_variants,
    parse_variants_recovering, push_error, strip_flatten, ParsedVariant,
};
use announce::helper::{Cont, ContList, Input, KvArgs};
use proc_macro2::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::*;
use template_quote::quote;

pub struct MacroArg {
    flat_path: Path,
    krate: Option<Path>,
}

impl Parse for MacroArg {
    fn parse(input: ParseStream) -> Result<Self> {
        let flat_path = input.parse()?;
        let krate = if input.parse::<Option<Token![@]>>()?.is_some() {
            Some(input.parse()?)
        } else {
            None
        };
        Ok(MacroArg { flat_path, krate })
    }
}

/// `#[into_flat(F @ krate)] enum S { .. }`
///
/// - emits `S` (without `#[flatten]`) and announces it, so that `#[flat(S)]`
///   can read it; which variants were `#[flatten]` is recorded as
///   `meta = flatten(V1, ..)` and read back from the entry META;
/// - invokes `S! { [S!, into_flat_cb!] .. }`: the callback reads the
///   cfg-evaluated `S` from its own trampoline and gathers the nested enums'
///   variants for the `IntoFlat` impl.
///
/// A bad `#[flatten]` variant or flat path is reported on top of the normal expansion (the
/// variant is taken as a normal one, the path by its last segment), so that `S` and `S!` are
/// still there for `#[flat]`.
pub fn into_flat(arg: MacroArg, input: ItemEnum) -> Result<TokenStream> {
    let MacroArg { flat_path, krate } = arg;
    let krate = match krate {
        Some(krate) => krate,
        None => parse2(quote!(::flat_enum))?,
    };
    let mut errors = None;
    let flat_path = if flat_path.leading_colon.is_some() || flat_path.segments.len() != 1 {
        push_error(
            &mut errors,
            Error::new(flat_path.span(), "Should be exist in the same context"),
        );
        Path {
            leading_colon: None,
            segments: flat_path.segments.last().cloned().into_iter().collect(),
        }
    } else {
        flat_path
    };
    let (variants, variant_errors) = parse_variants_recovering(&input, has_flatten);
    if let Some(e) = variant_errors {
        push_error(&mut errors, e);
    }
    let flattened: Vec<&Ident> = variants
        .iter()
        .filter_map(|v| match v {
            ParsedVariant::Flattened { ident, .. } => Some(ident),
            _ => None,
        })
        .collect();

    let mut stripped = input.clone();
    stripped
        .variants
        .iter_mut()
        .for_each(|v| v.attrs = strip_flatten(&v.attrs));

    let cb = ContList(vec![
        Cont::bang(ident_to_path(&input.ident)),
        Cont::bang(parse2(quote!(#krate::__private::into_flat_cb))?),
    ]);
    Ok(quote! {
        #{errors.map(|e| e.to_compile_error())}
        #stripped
        #{announce_wrapped(&krate, quote!(meta = flatten(#(#flattened),*)), &stripped)}
        #{cb.invoke(quote! { krate = #krate, flat = #flat_path })}
    })
}

/// Callback: `into_flat_cb! { [into_flat_cb!] S! => [..] { mod S { enum S } } ENTRY* krate = .., flat = F }`.
/// `S` is the cfg-evaluated payload of `S`'s own trampoline (oldest entry); its
/// flattened variants come from the META `flatten(V1, ..)`.
///
/// This is where a `#[flatten]` field type that is not an announced enum is reported (once:
/// `#[flat]`'s callback leaves the variants out silently). The `IntoFlat` impl is then a stub,
/// so that the bound of `#[flat]`'s `Flat` impl holds.
pub fn into_flat_cb(input: Input) -> Result<TokenStream> {
    let mut rest: KvArgs = input.rest_as()?;
    let (krate, flat_path): (Path, Path) = (rest.require("krate")?, rest.require("flat")?);
    let s_entry = input
        .entries_in_order()
        .next()
        .ok_or_else(|| Error::new(proc_macro2::Span::call_site(), "flat_enum: no entry"))?;
    let item = enum_of(s_entry)?;
    let flat_ids = flattened_idents(s_entry)?;
    let variants = parse_variants(&item, |v| flat_ids.contains(&v.ident))?;
    let nested = match nested(&variants, &input) {
        Ok(nested) => nested,
        Err(fetch) => return Ok(fetch),
    };
    let (mut items, mut errors) = (Vec::new(), None);
    for n in nested {
        match n {
            Ok(n) => items.push(n),
            Err(e) => push_error(&mut errors, e),
        }
    }
    Ok(match errors {
        None => emit_impl(&krate, &flat_path, &item, &variants, &items),
        Some(e) => {
            let stub = impl_with(
                &krate,
                &flat_path,
                &item,
                quote!(::core::unreachable!()),
                quote!(let _ = this; ::core::unreachable!()),
            );
            quote! { #{e.to_compile_error()} #stub }
        }
    })
}

fn emit_impl(
    krate: &Path,
    flat_path: &Path,
    input: &ItemEnum,
    variants: &[ParsedVariant],
    nested: &[ItemEnum],
) -> TokenStream {
    let flat = &flat_path.segments[0].ident;
    let mut nested_iter = nested.iter();
    let mut into_arms = Vec::new();
    let mut from_arms = Vec::new();
    for v in variants {
        match v {
            ParsedVariant::Normal(v) => {
                let (id, b) = (&v.ident, bindings(&v.fields));
                into_arms.push(quote! { Self::#id #b => #flat::#id #b });
                from_arms.push(quote! { #flat::#id #b => Self::#id #b });
            }
            ParsedVariant::Flattened {
                ident,
                ty,
                macro_path,
                ..
            } => {
                let n = nested_iter.next().unwrap();
                let inner = n.variants.iter().map(|nv| {
                    let (id, b) = (&nv.ident, bindings(&nv.fields));
                    quote! { #macro_path::#id #b => #flat::#id #b }
                });
                into_arms.push(quote! { Self::#ident(item) => match item { #(#inner,)* } });
                for nv in &n.variants {
                    let (id, b) = (&nv.ident, bindings(&nv.fields));
                    // `<T>::V { .. }` is unstable (`more_qualified_paths`),
                    // so named variants go through the plain path.
                    let ctor = match &nv.fields {
                        Fields::Named(_) => quote! { #macro_path::#id #b },
                        _ => quote! { <#ty>::#id #b },
                    };
                    from_arms.push(quote! { #flat::#id #b => Self::#ident(#ctor) });
                }
            }
        }
    }
    impl_with(
        krate,
        flat_path,
        input,
        quote! { match self { #(#into_arms,)* } },
        quote! { match this { #(#from_arms,)* } },
    )
}

/// `impl IntoFlat for S` with the given bodies of `into_flat` and `from_flat`.
fn impl_with(
    krate: &Path,
    flat_path: &Path,
    input: &ItemEnum,
    into_body: TokenStream,
    from_body: TokenStream,
) -> TokenStream {
    let (g_impl, g_type, g_where) = input.generics.split_for_impl();
    quote! {
        #[automatically_derived]
        impl #g_impl #krate::IntoFlat for #{&input.ident} #g_type #g_where {
            type Flat = #flat_path;
            fn into_flat(self) -> Self::Flat {
                #into_body
            }

            fn from_flat(this: Self::Flat) -> Self {
                #from_body
            }
        }
    }
}
