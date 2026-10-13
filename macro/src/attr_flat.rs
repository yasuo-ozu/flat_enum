use crate::subst::Subst;
use crate::util::{enum_of, nested, parse_variants, ParsedVariant};
use announce::helper::{Cont, ContList, Entry, Input, KvArgs};
use proc_macro2::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::*;
use template_quote::quote;

pub struct MacroArg {
    structured_path: Path,
    krate: Option<Path>,
}

impl Parse for MacroArg {
    fn parse(input: ParseStream) -> Result<Self> {
        let structured_path = input.parse()?;
        let krate = if input.parse::<Option<Token![@]>>()?.is_some() {
            Some(input.parse()?)
        } else {
            None
        };
        Ok(MacroArg {
            structured_path,
            krate,
        })
    }
}

/// The head of the flat enum, up to its body: attributes, visibility, name, generic
/// parameters (with their bounds) and the `where` clause.
struct Decl {
    attrs: Vec<Attribute>,
    vis: Visibility,
    enum_token: Token![enum],
    ident: Ident,
    generics: Generics,
}

impl Parse for Decl {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        let enum_token = input.parse()?;
        let ident = input.parse()?;
        let mut generics: Generics = input.parse()?;
        generics.where_clause = input.parse()?;
        Ok(Decl {
            attrs,
            vis,
            enum_token,
            ident,
            generics,
        })
    }
}

/// `#[flat(S<..> @ krate)] enum F<..> {}`: implements `Flat` and starts
/// `S! { [S!, flat_cb!] decl = <F's declaration head> }`.
pub fn flat(arg: MacroArg, input: ItemEnum) -> Result<TokenStream> {
    let MacroArg {
        structured_path,
        krate,
    } = arg;
    let krate = krate.unwrap_or(parse_quote!(::flat_enum));
    let (g_impl, g_type, g_where) = input.generics.split_for_impl();
    let macro_name =
        if structured_path.leading_colon.is_none() && structured_path.segments.len() == 1 {
            &structured_path.segments[0].ident
        } else {
            return Err(Error::new(
                structured_path.span(),
                "Should be exist in the same context",
            ));
        };
    if !input.variants.is_empty() {
        return Err(Error::new(input.span(), "Cannot specify variants"));
    }
    let list = ContList(vec![
        Cont::bang(parse_quote!(#macro_name)),
        Cont::bang(parse_quote!(#krate::__private::flat_cb)),
    ]);
    let trampoline = list.invoke(quote! {
        decl =
            #(for attr in &input.attrs) { #attr }
            #{&input.vis}
            #{&input.enum_token}
            #{&input.ident}
            #{&input.generics}
            #{&input.generics.where_clause}
    });
    Ok(quote! {
        #[automatically_derived]
        impl #g_impl #krate::Flat for #{ &input.ident } #g_type #g_where {
            type Structured = #structured_path;
        }
        #trampoline
    })
}

/// Variant idents recorded by `#[into_flat]` as the META `flatten(V1, ..)`.
pub fn flattened_idents(entry: &Entry) -> Result<Vec<Ident>> {
    match entry.find_meta("flatten") {
        None => Ok(Vec::new()),
        Some(m) => Ok(m
            .parse_args_with(Punctuated::<Ident, Token![,]>::parse_terminated)?
            .into_iter()
            .collect()),
    }
}

/// Callback: `[flat_cb!] S! => [..] { enum S } decl = ..` gathers the nested
/// enums' definitions (one more round) and emits the flat enum.
pub fn flat_cb(input: Input) -> Result<TokenStream> {
    let mut rest: KvArgs = input.rest_as()?;
    let decl: Decl = parse2(rest.tokens("decl").unwrap_or_default())?;
    let s_entry = input
        .entries_in_order()
        .next()
        .ok_or_else(|| Error::new(proc_macro2::Span::call_site(), "flat_enum: no entry"))?;
    let s_item = enum_of(s_entry)?;
    let flat_ids = flattened_idents(s_entry)?;
    let variants = parse_variants(&s_item, |v| flat_ids.contains(&v.ident))?;
    let nested = match nested(&variants, &input)? {
        Ok(nested) => nested,
        Err(fetch) => return Ok(fetch),
    };
    let mut nested_iter = nested.iter();
    let mut out = Vec::new();
    for v in &variants {
        match v {
            ParsedVariant::Normal(v) => out.push(quote! { #v }),
            ParsedVariant::Flattened { ty, args, .. } => {
                // The nested enum's field types, with its generic parameters
                // replaced by the arguments of the `#[flatten]` field type.
                let n = nested_iter.next().unwrap();
                let mut subst = Subst::new(n, ty, args, &decl.generics)?;
                for nv in &n.variants {
                    let fields = substituted_fields(&mut subst, &nv.fields)?;
                    // Only the docs: other attributes may belong to a derive of the
                    // nested enum, which the flat enum does not have.
                    let docs = nv.attrs.iter().filter(|a| a.path().is_ident("doc"));
                    out.push(quote! { #(#docs)* #{&nv.ident} #fields });
                }
            }
        }
    }
    let Decl {
        attrs,
        vis,
        enum_token,
        ident,
        generics,
    } = &decl;
    Ok(quote! {
        #(#attrs)* #vis #enum_token #ident #generics #{&generics.where_clause} { #(#out,)* }
    })
}

/// The fields of a variant of a nested enum, as the flat enum has them.
fn substituted_fields(subst: &mut Subst, fields: &Fields) -> Result<TokenStream> {
    let mut out = Vec::new();
    for f in fields {
        let (attrs, vis, ty) = (&f.attrs, &f.vis, subst.apply(&f.ty)?);
        out.push(match &f.ident {
            Some(id) => quote! { #(#attrs)* #vis #id : #ty },
            None => quote! { #(#attrs)* #vis #ty },
        });
    }
    Ok(match fields {
        Fields::Named(_) => quote! { { #(#out),* } },
        Fields::Unnamed(_) => quote! { ( #(#out),* ) },
        Fields::Unit => TokenStream::new(),
    })
}
