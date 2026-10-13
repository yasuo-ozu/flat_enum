use proc_macro2::Span;
use std::collections::HashMap;
use syn::punctuated::Punctuated;
use syn::visit_mut::{self, VisitMut};
use syn::*;

/// Rewrites the field types of a nested enum for the use site.
///
/// `enum E<'a, T, const N: usize> { V(&'a [T; N]) }` is flattened through
/// `#[flatten] W(E<'x, u8, 4>)`, where the field `&'a [T; N]` has to become
/// `&'x [u8; 4]`. The definition arrives as announced tokens, so the generic
/// parameters are replaced by the arguments of the `#[flatten]` field type
/// (`Self` by that field type itself). The announced tokens already resolve in the
/// calling crate: announce checks that at the definition, and rewrites the types of a
/// `repeater` to a path through it. Only the names of the generic parameters are
/// local to the definition.
pub struct Subst {
    lifetimes: HashMap<Ident, Lifetime>,
    /// The type parameters and `Self`.
    types: HashMap<Ident, Type>,
    /// The const parameters, as the argument was written: a `3`, a `{ N + 1 }`, or
    /// a path (which syn parses as a type).
    consts: HashMap<Ident, GenericArgument>,
    error: Option<Error>,
}

impl Subst {
    /// `nested` is the announced enum, `self_ty` the field type that names it in the
    /// `#[flatten]` variant and `args` its generic arguments.
    pub fn new(nested: &ItemEnum, self_ty: &Type, args: &[GenericArgument]) -> Result<Self> {
        let mut s = Subst {
            lifetimes: HashMap::new(),
            types: HashMap::new(),
            consts: HashMap::new(),
            error: None,
        };
        s.types
            .insert(Ident::new("Self", Span::call_site()), self_ty.clone());
        let expected = |what: &str, name: &dyn std::fmt::Display| {
            Error::new_spanned(
                self_ty,
                format!(
                    "flat_enum: `{}` needs {} argument for `{}`",
                    nested.ident, what, name
                ),
            )
        };
        let mut args = args.iter();
        for param in &nested.generics.params {
            match param {
                GenericParam::Lifetime(l) => match args.next() {
                    Some(GenericArgument::Lifetime(lt)) => {
                        s.lifetimes.insert(l.lifetime.ident.clone(), lt.clone());
                    }
                    _ => return Err(expected("a lifetime", &l.lifetime)),
                },
                GenericParam::Type(t) => {
                    let ty = match (args.next(), &t.default) {
                        (Some(GenericArgument::Type(ty)), _) => ty.clone(),
                        (None, Some(default)) => {
                            let mut default = default.clone();
                            s.visit_type_mut(&mut default);
                            default
                        }
                        _ => return Err(expected("a type", &t.ident)),
                    };
                    s.types.insert(t.ident.clone(), ty);
                }
                GenericParam::Const(c) => {
                    let arg = match (args.next(), &c.default) {
                        (Some(arg @ GenericArgument::Const(_)), _) => arg.clone(),
                        (Some(GenericArgument::Type(ty @ Type::Path(tp))), _)
                            if tp.qself.is_none() =>
                        {
                            GenericArgument::Type(ty.clone())
                        }
                        (None, Some(default)) => {
                            let mut default = default.clone();
                            s.visit_expr_mut(&mut default);
                            GenericArgument::Const(default)
                        }
                        _ => return Err(expected("a constant", &c.ident)),
                    };
                    s.consts.insert(c.ident.clone(), arg);
                }
            }
        }
        if let Some(extra) = args.next() {
            return Err(Error::new_spanned(
                extra,
                format!(
                    "flat_enum: too many generic arguments for `{}`",
                    nested.ident
                ),
            ));
        }
        Ok(s)
    }

    pub fn apply(&mut self, ty: &Type) -> Result<Type> {
        let mut ty = ty.clone();
        self.visit_type_mut(&mut ty);
        match self.error.take() {
            Some(e) => Err(e),
            None => Ok(ty),
        }
    }

    /// The type or const parameter (or `Self`) that `path` starts with, when it is a
    /// longer path (`T::Output`, `Self::LEN`): the replacement and the rest.
    fn split_assoc(&mut self, path: &Path) -> Option<(Type, Punctuated<PathSegment, Token![::]>)> {
        if path.leading_colon.is_some() || path.segments.len() < 2 {
            return None;
        }
        let first = &path.segments[0];
        if !first.arguments.is_none() {
            return None;
        }
        let repl = self.types.get(&first.ident)?.clone();
        let mut rest: Punctuated<PathSegment, Token![::]> =
            path.segments.iter().skip(1).cloned().collect();
        rest.iter_mut().for_each(|s| self.visit_path_segment_mut(s));
        Some((repl, rest))
    }

    /// The name of a path that is just one identifier.
    fn simple_ident<'a>(qself: &Option<QSelf>, path: &'a Path) -> Option<&'a Ident> {
        match path.get_ident() {
            Some(i) if qself.is_none() => Some(i),
            _ => None,
        }
    }
}

fn const_expr(arg: &GenericArgument) -> Expr {
    match arg {
        GenericArgument::Const(e) => e.clone(),
        GenericArgument::Type(Type::Path(tp)) => Expr::Path(ExprPath {
            attrs: Vec::new(),
            qself: None,
            path: tp.path.clone(),
        }),
        _ => unreachable!("Subst::new only stores these"),
    }
}

// The replacements are never visited again: they are written in the use site's
// names, which may be the nested enum's names again (`E<B, A>` for `E<A, B>`).
impl VisitMut for Subst {
    fn visit_lifetime_mut(&mut self, lt: &mut Lifetime) {
        if let Some(new) = self.lifetimes.get(&lt.ident) {
            *lt = new.clone();
        }
    }

    fn visit_type_mut(&mut self, ty: &mut Type) {
        if let Type::Path(tp) = ty {
            if let Some(ident) = Self::simple_ident(&tp.qself, &tp.path) {
                if let Some(repl) = self.types.get(ident) {
                    *ty = repl.clone();
                    return;
                }
            } else if tp.qself.is_none() {
                if let Some((repl, rest)) = self.split_assoc(&tp.path) {
                    // `T::Item` is resolved through the bounds of `T` in the definition,
                    // which `<Arg>::Item` does not have.
                    if tp.path.segments[0].ident != "Self" {
                        self.error.get_or_insert_with(|| {
                            Error::new_spanned(
                                &tp,
                                "flat_enum: write an associated type of a generic parameter as \
                                 `<T as Trait>::Assoc` in a flattened enum",
                            )
                        });
                        return;
                    }
                    *ty = parse_quote!(<#repl>::#rest);
                    return;
                }
            }
        }
        visit_mut::visit_type_mut(self, ty);
    }

    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        if let Expr::Path(ep) = expr {
            if let Some(ident) = Self::simple_ident(&ep.qself, &ep.path) {
                if let Some(arg) = self.consts.get(ident) {
                    *expr = const_expr(arg);
                    return;
                }
            } else if ep.qself.is_none() {
                if let Some((repl, rest)) = self.split_assoc(&ep.path) {
                    *expr = parse_quote!(<#repl>::#rest);
                    return;
                }
            }
        }
        visit_mut::visit_expr_mut(self, expr);
    }

    // `Foo<N>`: syn reads a constant argument that is a path as a type.
    fn visit_generic_argument_mut(&mut self, arg: &mut GenericArgument) {
        if let GenericArgument::Type(Type::Path(tp)) = arg {
            if let Some(ident) = Self::simple_ident(&tp.qself, &tp.path) {
                if let Some(new) = self.consts.get(ident) {
                    *arg = new.clone();
                    return;
                }
            }
        }
        visit_mut::visit_generic_argument_mut(self, arg);
    }
}
