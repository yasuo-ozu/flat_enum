use announce::helper::{Cont, Entry, Gather, Input};
use proc_macro2::{Span, TokenStream};
use syn::punctuated::{Pair, Punctuated};
use syn::spanned::Spanned;
use syn::*;
use template_quote::quote;

pub fn generics_remove_defaults(generics: &Generics) -> Generics {
    let mut ret = generics.clone();
    ret.params.iter_mut().for_each(|gp| match gp {
        GenericParam::Type(tp) => {
            tp.eq_token = None;
            tp.default = None;
        }
        GenericParam::Const(cp) => {
            cp.eq_token = None;
            cp.default = None;
        }
        _ => (),
    });
    ret
}

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

pub fn generics_to_arguments(generics: &Generics) -> PathArguments {
    let mut args = Punctuated::new();
    for p in generics.params.pairs() {
        let v = match p.value() {
            GenericParam::Lifetime(lt) => GenericArgument::Lifetime(lt.lifetime.clone()),
            GenericParam::Type(tp) => GenericArgument::Type(Type::Path(TypePath {
                qself: None,
                path: ident_to_path(&tp.ident),
            })),
            GenericParam::Const(cp) => GenericArgument::Const(Expr::Path(ExprPath {
                attrs: Vec::new(),
                qself: None,
                path: ident_to_path(&cp.ident),
            })),
        };
        args.push_value(v);
        if let Pair::Punctuated(_, p) = p {
            args.push_punct(*p)
        }
    }
    PathArguments::AngleBracketed(AngleBracketedGenericArguments {
        colon2_token: None,
        lt_token: generics.lt_token.unwrap_or_default(),
        args,
        gt_token: generics.gt_token.unwrap_or_default(),
    })
}

pub fn generic_arg_to_type(arg: GenericArgument) -> Result<Type> {
    Ok(match arg {
        GenericArgument::Lifetime(lt) => parse_quote!(& #lt ()),
        GenericArgument::Type(ty) => ty,
        GenericArgument::Const(expr) => parse_quote!([(); #expr]),
        _ => return Err(Error::new(arg.span(), "Not supported")),
    })
}

/// Every field of the enum in (variant, field) order: `Leak<N, _>` index `N`
/// is the field's position here. Computed on the definition side (for the
/// `Leak` impls) and again on the consumer side from the announce payload;
/// both see the same cfg-evaluated item, and the index never depends on how
/// a type is spelled (announce rewrites `crate::` in the payload).
pub fn leak_fields(input: &ItemEnum) -> impl Iterator<Item = &Field> {
    input.variants.iter().flat_map(|v| v.fields.iter())
}

/// The enum of an entry: announced wrapped in a same-named module (see
/// `derive_macro::emit_trampoline`), or as a plain enum.
pub fn enum_of(entry: &Entry) -> Result<ItemEnum> {
    let bad = |span: Span| {
        Error::new(
            span,
            "flat_enum: #[flatten] field type must be an enum with #[derive(FlatTarget)]",
        )
    };
    match entry.item()? {
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

/// `#[announce(emit_item = false, ..)] <vis> mod E { enum E }`: the trampoline
/// `E!` without emitting anything else. The module is never emitted; it only
/// wraps the payload because announce does not run its type-context check on
/// inline modules. That check would demand announce's own `repeater` for the
/// field types, which flat_enum resolves through its own `Leak` impls (or
/// re-emits in the caller's scope), and it rejects `[A; N]` outright.
pub fn announce_wrapped(krate: &Path, meta: TokenStream, item: &ItemEnum) -> TokenStream {
    quote! {
        #[#krate::__private::announce::announce(
            crate = #krate::__private::announce,
            emit_item = false,
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

fn split_path_param(mut path: Path) -> Result<(Path, Vec<Type>)> {
    let v = if let Some(seg) = path.segments.last_mut() {
        let v = match &seg.arguments {
            PathArguments::AngleBracketed(args) => args
                .args
                .iter()
                .cloned()
                .map(generic_arg_to_type)
                .collect::<Result<_>>()?,
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
        /// Generic arguments of `ty`, as types (for `Leak<N, (..)>`).
        arg_tys: Vec<Type>,
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
                        let (macro_path, arg_tys) = split_path_param(tp.path.clone())?;
                        return Ok(ParsedVariant::Flattened {
                            ident: variant.ident,
                            ty: field.unnamed[0].ty.clone(),
                            macro_path,
                            arg_tys,
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
