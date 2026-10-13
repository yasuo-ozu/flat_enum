use crate::attr_flat::flattened_idents;
use crate::util::{
    announce_wrapped, bindings, enum_of, has_flatten, ident_to_path, nested, parse_variants,
    strip_flatten, ParsedVariant,
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
pub fn into_flat(arg: MacroArg, input: ItemEnum) -> Result<TokenStream> {
    let MacroArg { flat_path, krate } = arg;
    let krate = krate.unwrap_or(parse_quote!(::flat_enum));
    if flat_path.leading_colon.is_some() || flat_path.segments.len() != 1 {
        return Err(Error::new(
            flat_path.span(),
            "Should be exist in the same context",
        ));
    }
    let variants = parse_variants(&input, has_flatten)?;
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
        Cont::bang(parse_quote!(#krate::__private::into_flat_cb)),
    ]);
    Ok(quote! {
        #stripped
        #{announce_wrapped(&krate, quote!(meta = flatten(#(#flattened),*)), &stripped)}
        #{cb.invoke(quote! { krate = #krate, flat = #flat_path })}
    })
}

/// Callback: `into_flat_cb! { [into_flat_cb!] S! => [..] { mod S { enum S } } ENTRY* krate = .., flat = F }`.
/// `S` is the cfg-evaluated payload of `S`'s own trampoline (oldest entry); its
/// flattened variants come from the META `flatten(V1, ..)`.
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
    match nested(&variants, &input)? {
        Ok(nested) => emit_impl(&krate, &flat_path, &item, &variants, &nested),
        Err(fetch) => Ok(fetch),
    }
}
fn emit_impl(
    krate: &Path,
    flat_path: &Path,
    input: &ItemEnum,
    variants: &[ParsedVariant],
    nested: &[ItemEnum],
) -> Result<TokenStream> {
    let (g_impl, g_type, g_where) = input.generics.split_for_impl();
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
    Ok(quote! {
        #[automatically_derived]
        unsafe impl #g_impl #krate::IntoFlat for #{&input.ident} #g_type #g_where {
            type Flat = #flat_path;
            fn into_flat(self) -> Self::Flat {
                match self {
                    #(#into_arms,)*
                }
            }

            fn from_flat(this: Self::Flat) -> Self {
                match this {
                    #(#from_arms,)*
                }
            }
        }
    })
}
