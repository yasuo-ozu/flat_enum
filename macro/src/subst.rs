use proc_macro2::Span;
use std::collections::{HashMap, HashSet};
use syn::punctuated::Punctuated;
use syn::visit::Visit;
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
    /// The only trait bound of each type parameter (`None` when it has none, several, or
    /// one that cannot be named outside the definition), to resolve `T::Assoc`.
    bounds: HashMap<Ident, Option<Path>>,
    /// The lifetimes that a binder (`for<'b>`) must not take: those that the replacements
    /// mention and the lifetimes of the flat enum.
    taken: HashSet<String>,
    /// The `for<..>` binders around the type being visited: the name in the definition and
    /// the name in the output, innermost last.
    binders: Vec<HashMap<Ident, Ident>>,
    error: Option<Error>,
}

impl Subst {
    /// `nested` is the announced enum, `self_ty` the field type that names it in the
    /// `#[flatten]` variant and `args` its generic arguments. `flat` is the generics of the
    /// flat enum that the field types go into.
    pub fn new(
        nested: &ItemEnum,
        self_ty: &Type,
        args: &[GenericArgument],
        flat: &Generics,
    ) -> Result<Self> {
        // Every replacement comes from the arguments in `self_ty` (or from a default,
        // which only has lifetimes of its own where it binds them).
        let mut taken = Lifetimes(HashSet::new());
        taken.visit_type(self_ty);
        taken
            .0
            .extend(flat.lifetimes().map(|l| l.lifetime.ident.to_string()));
        let mut s = Subst {
            lifetimes: HashMap::new(),
            types: HashMap::new(),
            consts: HashMap::new(),
            bounds: sole_bounds(&nested.generics),
            taken: taken.0,
            binders: Vec::new(),
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

    /// `<repl as Bound>::rest` for `T::rest`, when `T` has just one trait bound; otherwise
    /// `None`, because `T::rest` is resolved through the bounds of `T` in the definition.
    fn qualified(
        &mut self,
        param: &Ident,
        repl: &Type,
        rest: &Punctuated<PathSegment, Token![::]>,
    ) -> Option<Type> {
        let mut bound = self.bounds.get(param)?.clone()?;
        self.visit_path_mut(&mut bound);
        Some(parse_quote!(<#repl as #bound>::#rest))
    }

    /// Enters the scope of a `for<'b>` binder. A lifetime of the binder that a replacement
    /// could capture is renamed (with its uses, see `visit_lifetime_mut`). Returns whether
    /// there was a binder, to be passed to [`Subst::leave`].
    fn enter(&mut self, binder: Option<&BoundLifetimes>) -> bool {
        let binder = match binder {
            Some(binder) => binder,
            None => return false,
        };
        let names: Vec<&Ident> = binder
            .lifetimes
            .iter()
            .filter_map(|p| match p {
                GenericParam::Lifetime(l) => Some(&l.lifetime.ident),
                _ => None,
            })
            .collect();
        let mut taken = self.taken.clone();
        taken.extend(
            self.binders
                .iter()
                .flat_map(|b| b.values().map(|i| i.to_string())),
        );
        let mut used = taken.clone();
        used.extend(names.iter().map(|i| i.to_string()));
        let mut scope = HashMap::new();
        for name in names {
            let mut new = name.clone();
            if taken.contains(&name.to_string()) {
                new = (1..)
                    .map(|n| Ident::new(&format!("{}_{}", name, n), name.span()))
                    .find(|id| !used.contains(&id.to_string()))
                    .unwrap();
                used.insert(new.to_string());
            }
            scope.insert(name.clone(), new);
        }
        self.binders.push(scope);
        true
    }

    fn leave(&mut self, entered: bool) {
        if entered {
            self.binders.pop();
        }
    }

    /// The name of a path that is just one identifier.
    fn simple_ident<'a>(qself: &Option<QSelf>, path: &'a Path) -> Option<&'a Ident> {
        match path.get_ident() {
            Some(i) if qself.is_none() => Some(i),
            _ => None,
        }
    }
}

/// The lifetimes that are named in what is visited.
struct Lifetimes(HashSet<String>);

impl<'ast> Visit<'ast> for Lifetimes {
    fn visit_lifetime(&mut self, lt: &'ast Lifetime) {
        self.0.insert(lt.ident.to_string());
    }
}

/// For each type parameter, its only trait bound, in the parameter list or the `where`
/// clause. `None` when it has no trait bound, several (`T::Assoc` could belong to any of
/// them), or one that is higher-ranked (`for<'a> Tr<'a>`).
fn sole_bounds(generics: &Generics) -> HashMap<Ident, Option<Path>> {
    fn add(bounds: &mut Vec<Option<Path>>, bound: &TypeParamBound, ranked: bool) {
        if let TypeParamBound::Trait(t) = bound {
            if !matches!(t.modifier, TraitBoundModifier::Maybe(_)) {
                bounds.push(if t.lifetimes.is_none() && !ranked {
                    Some(without_constraints(&t.path))
                } else {
                    None
                });
            }
        }
    }
    let mut all: HashMap<Ident, Vec<Option<Path>>> = HashMap::new();
    for param in generics.type_params() {
        let bounds = all.entry(param.ident.clone()).or_default();
        param.bounds.iter().for_each(|b| add(bounds, b, false));
    }
    for pred in generics.where_clause.iter().flat_map(|w| &w.predicates) {
        if let WherePredicate::Type(pt) = pred {
            if let Type::Path(tp) = &pt.bounded_ty {
                if let Some(bounds) = tp.path.get_ident().and_then(|i| all.get_mut(i)) {
                    let ranked = pt.lifetimes.is_some();
                    pt.bounds.iter().for_each(|b| add(bounds, b, ranked));
                }
            }
        }
    }
    all.into_iter()
        .map(|(ident, mut bounds)| {
            let sole = match bounds.len() {
                1 => bounds.pop().unwrap(),
                _ => None,
            };
            (ident, sole)
        })
        .collect()
}

/// `Iterator<Item = u8>` as `Iterator`: a trait in `<T as Trait>` takes no constraints.
fn without_constraints(path: &Path) -> Path {
    let mut path = path.clone();
    for seg in path.segments.iter_mut() {
        if let PathArguments::AngleBracketed(ab) = &mut seg.arguments {
            ab.args = std::mem::take(&mut ab.args)
                .into_iter()
                .filter(|a| {
                    !matches!(
                        a,
                        GenericArgument::AssocType(_)
                            | GenericArgument::AssocConst(_)
                            | GenericArgument::Constraint(_)
                    )
                })
                .collect();
            if ab.args.is_empty() {
                seg.arguments = PathArguments::None;
            }
        }
    }
    path
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
        if let Some(new) = self.binders.iter().rev().find_map(|b| b.get(&lt.ident)) {
            lt.ident = new.clone();
        } else if let Some(new) = self.lifetimes.get(&lt.ident) {
            *lt = new.clone();
        }
    }

    fn visit_type_bare_fn_mut(&mut self, f: &mut TypeBareFn) {
        let scope = self.enter(f.lifetimes.as_ref());
        visit_mut::visit_type_bare_fn_mut(self, f);
        self.leave(scope);
    }

    fn visit_trait_bound_mut(&mut self, b: &mut TraitBound) {
        let scope = self.enter(b.lifetimes.as_ref());
        visit_mut::visit_trait_bound_mut(self, b);
        self.leave(scope);
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
                    let first = tp.path.segments[0].ident.clone();
                    if first == "Self" {
                        *ty = parse_quote!(<#repl>::#rest);
                        return;
                    }
                    // `T::Item` is resolved through the bounds of `T` in the definition,
                    // which `<Arg>::Item` does not have.
                    if let Some(qualified) = self.qualified(&first, &repl, &rest) {
                        *ty = qualified;
                        return;
                    }
                    self.error.get_or_insert_with(|| {
                        Error::new_spanned(
                            &tp,
                            format!(
                                "flat_enum: write an associated type of a generic parameter as \
                                 `<{} as Trait>::Assoc` in a flattened enum (`{}` has no single \
                                 trait bound to take the trait from)",
                                first, first
                            ),
                        )
                    });
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
