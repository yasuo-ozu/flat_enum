use proc_macro_error::abort;
use syn::punctuated::{Pair, Punctuated};
use syn::spanned::Spanned;
use syn::*;
use template_quote::quote;

pub fn getrandom() -> u64 {
    use core::hash::{BuildHasher, Hash, Hasher};
    use proc_macro2::Span;
    use std::collections::hash_map::RandomState;

    let mut hasher = RandomState::new().build_hasher();
    format!("{:?}", &Span::call_site()).hash(&mut hasher);
    hasher.finish()
}

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
        match p {
            Pair::Punctuated(_, p) => args.push_punct(p.clone()),
            _ => (),
        }
    }
    PathArguments::AngleBracketed(AngleBracketedGenericArguments {
        colon2_token: None,
        lt_token: generics.lt_token.unwrap_or(Default::default()),
        args,
        gt_token: generics.gt_token.unwrap_or(Default::default()),
    })
}

pub fn generic_arg_to_type(arg: GenericArgument) -> Type {
    match arg {
        GenericArgument::Lifetime(lt) => {
            syn::parse(quote! {& #lt ()}.into()).expect("Cannot parse reference to type")
        }
        GenericArgument::Type(ty) => ty,
        GenericArgument::Const(expr) => {
            syn::parse(quote! {[(); #expr]}.into()).expect("Cannot parse array to type")
        }
        _ => abort!(arg.span(), "Not supported"),
    }
}
