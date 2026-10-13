use announce::helper::{Cont, Entry, Gather, Input};
use proc_macro2::{Span, TokenStream};
use syn::spanned::Spanned;
use syn::visit_mut::{self, VisitMut};
use syn::*;
use template_quote::quote;

pub fn ident_to_path(ident: &Ident) -> Path {
    Path {
        leading_colon: None,
        segments: Some(PathSegment {
            ident: ident.clone(),
            arguments: PathArguments::None,
        })
        .into_iter()
        .collect(),
    }
}

/// The enum of an entry: an enum announced with `#[derive(announce::Announce)]` or
/// `#[announce]`, or the module-wrapped copy that `#[into_flat]` announces
/// (see [`announce_wrapped`]).
///
/// The types of an enum announced with a `repeater` are written through it
/// (announce's `type_leak`), so that they resolve in the calling crate.
pub fn enum_of(entry: &Entry) -> Result<ItemEnum> {
    let bad = |span: Span| {
        Error::new(
            span,
            "flat_enum: #[flatten] field type must be an enum announced with \
             #[derive(announce::Announce)] or #[announce]",
        )
    };
    let mut item = entry.item()?;
    if let Some(type_leak) = entry.type_leak()? {
        type_leak.try_apply(&mut item)?;
        Reparse.visit_item_mut(&mut item);
    }
    match item {
        Item::Enum(e) => Ok(e),
        Item::Mod(ItemMod {
            content: Some((_, items)),
            ..
        }) => match <[Item; 1]>::try_from(items) {
            Ok([Item::Enum(e)]) => Ok(e),
            _ => Err(bad(entry.key.path().span())),
        },
        other => Err(bad(other.span())),
    }
}

/// `TypeLeak` writes the types it replaces as opaque tokens. Parse them back, so that
/// [`Subst`](crate::subst::Subst) can see the generic parameters in them.
struct Reparse;

impl VisitMut for Reparse {
    fn visit_type_mut(&mut self, ty: &mut Type) {
        if let Type::Verbatim(tokens) = ty {
            if let Ok(parsed) = parse2::<Type>(tokens.clone()) {
                *ty = parsed;
            }
        }
        visit_mut::visit_type_mut(self, ty);
    }
}

/// `#[announce(emit_item = false, ..)] <vis> mod E { enum E }`: the trampoline
/// `E!` without emitting anything else. The module is never emitted; it only
/// wraps the payload because announce does not run its type-context check on
/// inline modules. The structured enum of `#[into_flat]` names the nested enums
/// by their paths in the module of the flat enum (`Enum1<A>`), which would not
/// resolve for another crate.
pub fn announce_wrapped(krate: &Path, meta: TokenStream, item: &ItemEnum) -> TokenStream {
    quote! {
        #[#krate::announce::announce(
            crate = #krate::announce,
            emit_item = false,
            docs = true,
            #meta
        )]
        #{&item.vis} mod #{&item.ident} { #item }
    }
}

/// `(a0, a1)`, `{ x, y }` or nothing: usable both as a pattern and as a
/// constructor (field shorthand).
pub fn bindings(fields: &Fields) -> TokenStream {
    match fields {
        Fields::Named(f) => {
            let ids = f.named.iter().map(|f| f.ident.as_ref().unwrap());
            quote! { { #(#ids),* } }
        }
        Fields::Unnamed(f) => {
            let ids =
                (0..f.unnamed.len()).map(|i| Ident::new(&format!("a{}", i), Span::call_site()));
            quote! { ( #(#ids),* ) }
        }
        Fields::Unit => TokenStream::new(),
    }
}

fn attribute_is_flatten(attr: &Attribute) -> bool {
    attr.style == AttrStyle::Outer && attr.path().is_ident("flatten")
}

pub fn strip_flatten(attrs: &[Attribute]) -> Vec<Attribute> {
    attrs
        .iter()
        .filter(|a| !attribute_is_flatten(a))
        .cloned()
        .collect()
}

pub fn has_flatten(variant: &Variant) -> bool {
    variant.attrs.iter().any(attribute_is_flatten)
}

fn split_path_param(mut path: Path) -> Result<(Path, Vec<GenericArgument>)> {
    let v = if let Some(seg) = path.segments.last_mut() {
        let v = match &seg.arguments {
            PathArguments::AngleBracketed(args) => args.args.iter().cloned().collect(),
            PathArguments::None => Vec::new(),
            other => return Err(Error::new(other.span(), "Not supported")),
        };
        seg.arguments = PathArguments::None;
        v
    } else {
        Vec::new()
    };
    Ok((path, v))
}

pub enum ParsedVariant {
    Normal(Variant),
    Flattened {
        ident: Ident,
        /// The full field type, e.g. `Enum1<A>`.
        ty: Type,
        /// The type path without generics; also the announce macro path.
        macro_path: Path,
        /// Generic arguments of `ty`.
        args: Vec<GenericArgument>,
    },
}

impl ParsedVariant {
    pub fn new(variant: Variant, flatten: bool) -> Result<Self> {
        if !flatten {
            return Ok(ParsedVariant::Normal(variant));
        }
        if let Fields::Unnamed(field) = &variant.fields {
            if field.unnamed.len() == 1 && field.unnamed[0].attrs.is_empty() {
                if let Type::Path(tp) = &field.unnamed[0].ty {
                    if tp.qself.is_none() {
                        let (macro_path, args) = split_path_param(tp.path.clone())?;
                        return Ok(ParsedVariant::Flattened {
                            ident: variant.ident,
                            ty: field.unnamed[0].ty.clone(),
                            macro_path,
                            args,
                        });
                    }
                }
            }
        }
        Err(Error::new(variant.span(), "Bad variant for #[flatten]"))
    }

    pub fn macro_path(&self) -> Option<&Path> {
        match self {
            ParsedVariant::Flattened { macro_path, .. } => Some(macro_path),
            _ => None,
        }
    }
}

/// Variants of the structured enum. `flatten` decides which are flattened.
pub fn parse_variants(
    input: &ItemEnum,
    flatten: impl Fn(&Variant) -> bool,
) -> Result<Vec<ParsedVariant>> {
    input
        .variants
        .iter()
        .map(|v| ParsedVariant::new(v.clone(), flatten(v)))
        .collect()
}

/// The nested enums' announce payloads, one per flattened variant, in order;
/// `Err(tokens)` fetches the missing ones and calls back.
pub fn nested(
    variants: &[ParsedVariant],
    input: &Input,
) -> Result<std::result::Result<Vec<ItemEnum>, TokenStream>> {
    let needed = variants
        .iter()
        .filter_map(|v| v.macro_path())
        .cloned()
        .map(Cont::bang);
    let entries = match input.gather(needed) {
        Gather::Ready(entries) => entries,
        Gather::Fetch(tokens) => return Ok(Err(tokens)),
    };
    entries
        .into_iter()
        .map(enum_of)
        .collect::<Result<_>>()
        .map(Ok)
}

pub fn into_ts(r: Result<TokenStream>) -> proc_macro::TokenStream {
    r.unwrap_or_else(|e| e.to_compile_error()).into()
}
