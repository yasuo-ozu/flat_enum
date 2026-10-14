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
    /// The trait bounds of each type parameter, to resolve `T::Assoc`.
    bounds: HashMap<Ident, Bounds>,
    /// The type parameters of the flat enum: an argument that is one of them is written
    /// as it is in `Arg::Assoc`, and its own bounds resolve the associated type.
    flat_params: HashSet<Ident>,
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
            flat_params: flat.type_params().map(|p| p.ident.clone()).collect(),
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
        // A default can have an error of its own (`T::Assoc` without a bound).
        match s.error.take() {
            Some(e) => Err(e),
            None => Ok(s),
        }
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

    /// `T::rest` for the replacement `repl` of `T`, as a qualified self type and a path.
    ///
    /// - `Arg::rest` when `Arg` is a type parameter of the flat enum: its bounds there are
    ///   the bounds of `T` (the structured enum needs them for `E<Arg>` to be well-formed),
    ///   and they resolve `rest` whichever trait declares it, a supertrait included;
    /// - `<Arg as Bound>::rest` when `T` has exactly one trait bound, which then has to
    ///   declare `rest` itself;
    /// - otherwise the message of an error: `T::rest` is resolved through the bounds of `T`
    ///   in the definition, which `<Arg>::rest` does not have.
    fn assoc_path(
        &mut self,
        param: &Ident,
        repl: &Type,
        rest: &Punctuated<PathSegment, Token![::]>,
    ) -> std::result::Result<(Option<QSelf>, Path), String> {
        let mut path = Path {
            leading_colon: None,
            segments: Punctuated::new(),
        };
        if let Some(arg) = bare_ident(repl) {
            if self.flat_params.contains(arg) {
                path.segments.push(PathSegment::from(arg.clone()));
                path.segments.extend(rest.iter().cloned());
                return Ok((None, path));
            }
        }
        let why = match self.bounds.get(param) {
            Some(Bounds::One(bound)) => {
                let mut bound = bound.clone();
                self.visit_path_mut(&mut bound);
                path.leading_colon = bound.leading_colon;
                path.segments = bound.segments;
                let position = path.segments.len();
                path.segments.extend(rest.iter().cloned());
                return Ok((Some(qself(repl, position, true)), path));
            }
            Some(Bounds::Several) => "has several trait bounds, and flat_enum cannot tell which \
                                      of them declares it"
                .to_string(),
            Some(Bounds::HigherRanked) => "has a higher-ranked trait bound".to_string(),
            Some(Bounds::Parenthesized) => "has a bound with parenthesized arguments \
                                            (`Fn(..) -> ..`), which cannot be named in \
                                            `<.. as Trait>`"
                .to_string(),
            Some(Bounds::None) | None => "has no trait bound to take the trait from".to_string(),
        };
        let assoc = rest
            .first()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        Err(format!(
            "flat_enum: write `<{p} as Trait>::{a}` instead of `{p}::{a}` in a flattened \
             enum: `{p}` {why}",
            p = param,
            a = assoc,
            why = why
        ))
    }

    /// `<repl>::rest` for `Self::rest`.
    fn self_assoc(
        repl: &Type,
        rest: &Punctuated<PathSegment, Token![::]>,
    ) -> (Option<QSelf>, Path) {
        let path = Path {
            leading_colon: Some(Default::default()),
            segments: rest.clone(),
        };
        (Some(qself(repl, 0, false)), path)
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

/// The trait bounds of a type parameter (those of `Sized` and `?Sized` left out), to tell
/// which trait `T::Assoc` belongs to.
enum Bounds {
    None,
    /// Exactly one trait (the same bound written twice counts once), without its
    /// associated type constraints.
    One(Path),
    Several,
    /// `for<'a> Tr<'a>`, which `<Arg as Tr<'a>>` cannot express.
    HigherRanked,
    /// `Fn(u8) -> R`. The associated type of such a bound belongs to a supertrait
    /// (`FnOnce::Output`), and the parenthesized sugar cannot be written in `<.. as ..>`.
    Parenthesized,
}

/// One trait bound of a type parameter, as written (apart from its constraints).
#[derive(PartialEq)]
struct Bound {
    path: Path,
    ranked: bool,
}

fn is_sized(path: &Path) -> bool {
    let names: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    path.segments.iter().all(|s| s.arguments.is_none())
        && matches!(
            names[..],
            ["Sized"] | ["core", "marker", "Sized"] | ["std", "marker", "Sized"]
        )
}

/// For each type parameter, its trait bounds, in the parameter list and the `where` clause.
fn sole_bounds(generics: &Generics) -> HashMap<Ident, Bounds> {
    fn add(bounds: &mut Vec<Bound>, bound: &TypeParamBound, ranked: bool) {
        if let TypeParamBound::Trait(t) = bound {
            if matches!(t.modifier, TraitBoundModifier::Maybe(_)) || is_sized(&t.path) {
                return;
            }
            let bound = Bound {
                path: without_constraints(&t.path),
                ranked: ranked || t.lifetimes.is_some(),
            };
            if !bounds.contains(&bound) {
                bounds.push(bound);
            }
        }
    }
    let mut all: HashMap<Ident, Vec<Bound>> = HashMap::new();
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
            let sole = match bounds.pop() {
                None => Bounds::None,
                Some(_) if !bounds.is_empty() => Bounds::Several,
                Some(Bound { ranked: true, .. }) => Bounds::HigherRanked,
                Some(Bound { path, .. }) => {
                    if path
                        .segments
                        .iter()
                        .any(|s| matches!(s.arguments, PathArguments::Parenthesized(_)))
                    {
                        Bounds::Parenthesized
                    } else {
                        Bounds::One(path)
                    }
                }
            };
            (ident, sole)
        })
        .collect()
}

/// The identifier that `ty` is, when it is just a name.
fn bare_ident(ty: &Type) -> Option<&Ident> {
    match ty {
        Type::Path(tp) if tp.qself.is_none() => tp.path.get_ident(),
        Type::Group(g) => bare_ident(&g.elem),
        Type::Paren(p) => bare_ident(&p.elem),
        _ => None,
    }
}

/// `<ty as` / `<ty>` of a qualified path whose trait takes `position` segments.
fn qself(ty: &Type, position: usize, with_trait: bool) -> QSelf {
    QSelf {
        lt_token: Default::default(),
        ty: Box::new(ty.clone()),
        position,
        as_token: if with_trait {
            Some(Default::default())
        } else {
            None
        },
        gt_token: Default::default(),
    }
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
                    let assoc = if first == "Self" {
                        Ok(Self::self_assoc(&repl, &rest))
                    } else {
                        // `T::Item` is resolved through the bounds of `T` in the
                        // definition, which `<Arg>::Item` does not have.
                        self.assoc_path(&first, &repl, &rest)
                    };
                    match assoc {
                        Ok((qself, path)) => {
                            *ty = Type::Path(TypePath { qself, path });
                        }
                        Err(message) => {
                            self.error
                                .get_or_insert_with(|| Error::new_spanned(&*tp, message));
                        }
                    }
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
                // `T::LEN` or `Self::LEN`, as a type is resolved. (On stable Rust a
                // generic parameter cannot be used in a constant expression, so this is
                // reached for `Self` and with `generic_const_exprs`.)
                if let Some((repl, rest)) = self.split_assoc(&ep.path) {
                    let first = ep.path.segments[0].ident.clone();
                    let assoc = if first == "Self" {
                        Ok(Self::self_assoc(&repl, &rest))
                    } else {
                        self.assoc_path(&first, &repl, &rest)
                    };
                    match assoc {
                        Ok((qself, path)) => {
                            *expr = Expr::Path(ExprPath {
                                attrs: ep.attrs.clone(),
                                qself,
                                path,
                            });
                        }
                        Err(message) => {
                            self.error
                                .get_or_insert_with(|| Error::new_spanned(&*ep, message));
                        }
                    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use template_quote::quote;

    /// `field` of the enum `nested`, flattened as `ty` into an enum with the generics `flat`.
    fn run(nested: &str, field: &str, flat: &str, ty: &str) -> std::result::Result<String, String> {
        let nested: ItemEnum = parse_str(nested).unwrap();
        let self_ty: Type = parse_str(ty).unwrap();
        let args: Vec<GenericArgument> = match &self_ty {
            Type::Path(tp) => match &tp.path.segments.last().unwrap().arguments {
                PathArguments::AngleBracketed(a) => a.args.iter().cloned().collect(),
                _ => Vec::new(),
            },
            _ => unreachable!(),
        };
        let flat: Generics = parse_str(flat).unwrap();
        let field: Type = parse_str(field).unwrap();
        Subst::new(&nested, &self_ty, &args, &flat)
            .and_then(|mut s| s.apply(&field))
            .map(|t| quote!(#t).to_string().replace(' ', ""))
            .map_err(|e| e.to_string())
    }

    #[test]
    fn fn_family_bound_is_an_error() {
        // Used to panic on the `<_ as ::core::ops::Fn(u8) -> R>` it built.
        let err = run(
            "enum E<F: ::core::ops::Fn(u8) -> R, R> { V(F::Output, F) }",
            "F::Output",
            "<>",
            "E<fn(u8) -> u32, u32>",
        )
        .unwrap_err();
        assert!(err.contains("parenthesized"), "{}", err);
        assert!(err.contains("`<F as Trait>::Output`"), "{}", err);
    }

    #[test]
    fn fn_family_bound_with_a_parameter_of_the_flat_enum() {
        // The flat enum's own bound resolves `Output` (declared by `FnOnce`).
        let ok = run(
            "enum E<F: ::core::ops::Fn(u8) -> R, R> { V(F::Output, F) }",
            "F::Output",
            "<G: ::core::ops::Fn(u8) -> S, S>",
            "E<G, S>",
        );
        assert_eq!(ok.unwrap(), "G::Output");
    }

    #[test]
    fn supertrait_associated_type() {
        let nested = "enum E<T: ::probe::Sub> { V(T::Assoc) }";
        // A parameter of the flat enum: its bounds find the supertrait's `Assoc`.
        let ok = run(nested, "T::Assoc", "<P: ::probe::Sub>", "E<P>").unwrap();
        assert_eq!(ok, "P::Assoc");
        // A concrete type: the bound has to declare `Assoc` itself.
        let ok = run(nested, "T::Assoc", "<>", "E<u8>").unwrap();
        assert_eq!(ok, "<u8as::probe::Sub>::Assoc");
        // A parameter of the flat enum that is wrapped is not "bare".
        let ok = run(nested, "T::Assoc", "<P: ::probe::Sub>", "E<Vec<P>>").unwrap();
        assert_eq!(ok, "<Vec<P>as::probe::Sub>::Assoc");
        // Not a parameter of the flat enum.
        let ok = run(nested, "T::Assoc", "<Q>", "E<P>").unwrap();
        assert_eq!(ok, "<Pas::probe::Sub>::Assoc");
    }

    #[test]
    fn parameter_of_the_flat_enum_needs_no_sole_bound() {
        for nested in &[
            "enum E<T> { V(T::A) }",
            "enum E<T: ::probe::Tr + ::probe::Other> { V(T::A) }",
            "enum E<T> where for<'a> T: ::probe::Lt<'a> { V(T::A) }",
        ] {
            let ok = run(nested, "T::A", "<P: ::probe::Tr>", "E<P>").unwrap();
            assert_eq!(ok, "P::A", "{}", nested);
        }
    }

    #[test]
    fn sized_is_not_a_bound() {
        for nested in &[
            "enum E<T: Sized + ::probe::Tr> { V(T::A) }",
            "enum E<T: ::probe::Tr + ::core::marker::Sized> { V(T::A) }",
            "enum E<T: ::probe::Tr + ?Sized> { V(T::A) }",
            "enum E<T: ::probe::Tr> where T: ::std::marker::Sized { V(T::A) }",
        ] {
            let ok = run(nested, "T::A", "<>", "E<u8>").unwrap();
            assert_eq!(ok, "<u8as::probe::Tr>::A", "{}", nested);
        }
    }

    #[test]
    fn the_same_bound_twice_is_one_bound() {
        for nested in &[
            "enum E<T: ::probe::Tr> where T: ::probe::Tr { V(T::A) }",
            "enum E<T: ::probe::Tr + ::probe::Tr> { V(T::A) }",
            "enum E<T: ::probe::Tr<Item = u8>> where T: ::probe::Tr { V(T::A) }",
        ] {
            let ok = run(nested, "T::A", "<>", "E<u8>").unwrap();
            assert_eq!(ok, "<u8as::probe::Tr>::A", "{}", nested);
        }
        // Different arguments are different traits.
        let err = run(
            "enum E<T: ::probe::Conv<u8> + ::probe::Conv<u16>> { V(T::Out) }",
            "T::Out",
            "<>",
            "E<u8>",
        )
        .unwrap_err();
        assert!(err.contains("several trait bounds"), "{}", err);
    }

    #[test]
    fn higher_ranked_bound_message() {
        for nested in &[
            "enum E<T: for<'a> ::probe::Lt<'a>> { V(T::A) }",
            "enum E<T> where for<'a> T: ::probe::Lt<'a> { V(T::A) }",
        ] {
            let err = run(nested, "T::A", "<>", "E<u8>").unwrap_err();
            assert!(err.contains("higher-ranked trait bound"), "{}", err);
            assert!(err.contains("`<T as Trait>::A`"), "{}", err);
        }
    }

    #[test]
    fn no_bound_message() {
        let err = run("enum E<T> { V(T::A) }", "T::A", "<>", "E<u8>").unwrap_err();
        assert!(err.contains("no trait bound"), "{}", err);
    }

    #[test]
    fn associated_constant_is_qualified_like_a_type() {
        let ok = run(
            "enum E<T: ::probe::Tr> { V([u8; T::LEN]) }",
            "[u8; T::LEN]",
            "<>",
            "E<u8>",
        )
        .unwrap();
        assert_eq!(ok, "[u8;<u8as::probe::Tr>::LEN]");
        let ok = run(
            "enum E<T: ::probe::Tr> { V([u8; T::LEN]) }",
            "[u8; T::LEN]",
            "<P: ::probe::Tr>",
            "E<P>",
        )
        .unwrap();
        assert_eq!(ok, "[u8;P::LEN]");
        let ok = run(
            "enum E { V([u8; Self::LEN]) }",
            "[u8; Self::LEN]",
            "<>",
            "E",
        )
        .unwrap();
        assert_eq!(ok, "[u8;<E>::LEN]");
        let err = run(
            "enum E<T: ::probe::A + ::probe::B> { V([u8; T::LEN]) }",
            "[u8; T::LEN]",
            "<>",
            "E<u8>",
        )
        .unwrap_err();
        assert!(err.contains("several trait bounds"), "{}", err);
    }
}
