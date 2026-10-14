//! The field types of a nested enum are read from its announced definition; its generic
//! parameters are replaced by the arguments of the `#[flatten]` field type. Each enum below
//! would break a replacement that is naive in one way, and the round trips check the types.
#![allow(clippy::type_complexity)]

use flat_enum::announce::Announce;
use flat_enum::{flat, into_flat, IntoFlat};

// Parameters used in the other order: a replacement must not be replaced again.
#[derive(Announce, Debug, PartialEq)]
#[announce(crate = ::flat_enum::announce)]
pub enum Pair<A, B> {
    P(A, B),
    Q { b: B },
}

#[into_flat(SwappedFlat<A, B>)]
#[derive(Debug, PartialEq)]
pub enum Swapped<A, B> {
    #[flatten]
    Pair(Pair<B, A>),
}

#[flat(Swapped<A, B>)]
#[derive(Debug, PartialEq)]
pub enum SwappedFlat<A, B> {}

#[test]
fn swapped_parameters() {
    let f = Swapped::<u8, &str>::Pair(Pair::P("x", 1)).into_flat();
    assert_eq!(f, SwappedFlat::P("x", 1));
    assert_eq!(
        Swapped::<u8, &str>::from_flat(SwappedFlat::Q { b: 2 }),
        Swapped::Pair(Pair::Q { b: 2 })
    );
}

// An argument that is itself a type with arguments.
#[into_flat(TwiceFlat<A>)]
#[derive(Debug, PartialEq)]
pub enum Twice<A> {
    #[flatten]
    Pair(Pair<Pair<A, A>, u8>),
}

#[flat(Twice<A>)]
#[derive(Debug, PartialEq)]
pub enum TwiceFlat<A> {}

#[test]
fn argument_with_arguments() {
    let p = Pair::P(1u16, 2u16);
    let f = Twice::Pair(Pair::P(p, 3)).into_flat();
    assert_eq!(f, TwiceFlat::P(Pair::P(1, 2), 3));
    assert_eq!(Twice::from_flat(f), Twice::Pair(Pair::P(Pair::P(1, 2), 3)));
}

// Lifetimes, constants (a literal and a block), arrays and `Self`.
#[derive(Announce, Debug, PartialEq)]
#[announce(crate = ::flat_enum::announce)]
pub enum Arr<'a, T, const N: usize> {
    Slice(&'a [T; N]),
    Nested {
        rest: ::std::boxed::Box<Self>,
        len: [u8; N],
    },
    Empty,
}

#[into_flat(ArrFlat<'a, T>)]
#[derive(Debug, PartialEq)]
pub enum ArrOuter<'a, T> {
    #[flatten]
    Three(Arr<'a, T, 3>),
}

#[flat(ArrOuter<'a, T>)]
#[derive(Debug, PartialEq)]
pub enum ArrFlat<'a, T> {}

#[into_flat(ArrBlockFlat)]
#[derive(Debug, PartialEq)]
pub enum ArrBlock {
    #[flatten]
    Two(Arr<'static, u16, { 1 + 1 }>),
}

#[flat(ArrBlock)]
#[derive(Debug, PartialEq)]
pub enum ArrBlockFlat {}

#[test]
fn constant_block() {
    static X: [u16; 2] = [5, 6];
    assert_eq!(
        ArrBlock::Two(Arr::Slice(&X)).into_flat(),
        ArrBlockFlat::Slice(&X)
    );
    let nested = ArrBlockFlat::Nested {
        rest: Box::new(Arr::Empty),
        len: [1, 2],
    };
    assert_eq!(
        ArrBlock::from_flat(nested),
        ArrBlock::Two(Arr::Nested {
            rest: Box::new(Arr::Empty),
            len: [1, 2]
        })
    );
}

#[test]
fn lifetimes_constants_and_self() {
    let x = [1u8, 2, 3];
    let f = ArrOuter::Three(Arr::Slice(&x)).into_flat();
    assert_eq!(f, ArrFlat::Slice(&x));
    let nested: ArrFlat<'_, u8> = ArrFlat::Nested {
        rest: Box::new(Arr::Empty),
        len: [0; 3],
    };
    assert_eq!(
        ArrOuter::from_flat(nested),
        ArrOuter::Three(Arr::Nested {
            rest: Box::new(Arr::Empty),
            len: [0; 3]
        })
    );
}

// An associated type of a parameter, written with the trait.
#[derive(Announce)]
#[announce(crate = ::flat_enum::announce)]
pub enum Assoc<I: ::core::iter::Iterator> {
    Item(<I as ::core::iter::Iterator>::Item),
    Done,
}

#[into_flat(AssocOuterFlat)]
pub enum AssocOuter {
    #[flatten]
    Assoc(Assoc<::std::vec::IntoIter<u8>>),
}

#[flat(AssocOuter)]
pub enum AssocOuterFlat {}

#[test]
fn associated_type() {
    match AssocOuter::Assoc(Assoc::Item(7)).into_flat() {
        AssocOuterFlat::Item(n) => assert_eq!(n, 7u8),
        AssocOuterFlat::Done => unreachable!(),
    }
}

// Defaults of the parameters.
#[derive(Announce, Debug, PartialEq)]
#[announce(crate = ::flat_enum::announce)]
pub enum Defaults<A = u8, B = ::std::vec::Vec<A>> {
    D(A, B),
}

#[into_flat(DefaultsOuterFlat)]
#[derive(Debug, PartialEq)]
pub enum DefaultsOuter {
    #[flatten]
    Defaults(Defaults),
}

#[flat(DefaultsOuter)]
#[derive(Debug, PartialEq)]
pub enum DefaultsOuterFlat {}

#[test]
fn defaulted_parameters() {
    assert_eq!(
        DefaultsOuter::Defaults(Defaults::D(1, vec![2])).into_flat(),
        DefaultsOuterFlat::D(1, vec![2])
    );
}

// `#[announce]` on the enum instead of the derive.
#[flat_enum::announce::announce(crate = ::flat_enum::announce)]
#[derive(Debug, PartialEq)]
pub enum Attr {
    A(u8),
}

#[into_flat(AttrOuterFlat)]
#[derive(Debug, PartialEq)]
pub enum AttrOuter {
    #[flatten]
    Attr(Attr),
}

#[flat(AttrOuter)]
#[derive(Debug, PartialEq)]
pub enum AttrOuterFlat {}

#[test]
fn attribute_announce() {
    assert_eq!(AttrOuter::Attr(Attr::A(1)).into_flat(), AttrOuterFlat::A(1));
}

// An associated type of a parameter, written `T::Assoc`: with one trait bound on `T` it is
// `<Arg as Bound>::Assoc`, whether the bound is in the parameter list or in the `where` clause.
pub trait Tr {
    type A;
}
impl Tr for u8 {
    type A = u16;
}

pub trait Conv<X> {
    type Out;
}
impl Conv<u8> for u16 {
    type Out = u32;
}

#[derive(Announce)]
#[announce(crate = ::flat_enum::announce)]
pub enum Shorthand<T: crate::Tr, U>
where
    U: crate::Conv<u8>,
{
    Direct(T::A),
    Wrapped(::core::option::Option<T::A>, U::Out),
}

#[into_flat(ShorthandOuterFlat)]
pub enum ShorthandOuter {
    #[flatten]
    Shorthand(Shorthand<u8, u16>),
}

#[flat(ShorthandOuter)]
pub enum ShorthandOuterFlat {}

#[test]
fn associated_type_shorthand() {
    match ShorthandOuter::Shorthand(Shorthand::Direct(7)).into_flat() {
        ShorthandOuterFlat::Direct(n) => assert_eq!(n, 7u16),
        ShorthandOuterFlat::Wrapped(..) => unreachable!(),
    }
    match ShorthandOuter::Shorthand(Shorthand::Wrapped(Some(1), 2)).into_flat() {
        ShorthandOuterFlat::Wrapped(a, b) => assert_eq!((a, b), (Some(1u16), 2u32)),
        ShorthandOuterFlat::Direct(_) => unreachable!(),
    }
}

// A bound with associated type constraints, and a generic bound.
#[derive(Announce)]
#[announce(crate = ::flat_enum::announce)]
pub enum Constrained<I: ::core::iter::Iterator<Item = u8>, X, T: crate::Conv<X>> {
    Item(I::Item),
    Out(T::Out, ::core::marker::PhantomData<X>),
}

#[into_flat(ConstrainedOuterFlat)]
pub enum ConstrainedOuter {
    #[flatten]
    Constrained(Constrained<::std::vec::IntoIter<u8>, u8, u16>),
}

#[flat(ConstrainedOuter)]
pub enum ConstrainedOuterFlat {}

#[test]
fn constrained_and_generic_bounds() {
    let item = ConstrainedOuter::Constrained(Constrained::Item(1));
    assert!(matches!(item.into_flat(), ConstrainedOuterFlat::Item(1u8)));
    let out = ConstrainedOuter::Constrained(Constrained::Out(2, Default::default()));
    assert!(matches!(
        out.into_flat(),
        ConstrainedOuterFlat::Out(2u32, _)
    ));
}

// A lifetime bound by `for<..>` is not the parameter of the same name, and is renamed when
// the argument mentions a lifetime with its name.
#[derive(Announce)]
#[announce(crate = ::flat_enum::announce)]
pub enum Higher<'a> {
    Fn(for<'b> fn(&'b u8, &'a u8) -> &'b u8),
    Dyn(::std::boxed::Box<dyn for<'b> ::core::ops::Fn(&'b u8, &'a u8) -> &'b u8 + 'a>),
    Nested(for<'b, 'c> fn(for<'d> fn(&'b u8, &'c u8, &'d u8, &'a u8))),
}

fn first<'x>(a: &'x u8, _: &u8) -> &'x u8 {
    a
}

#[into_flat(HigherCapturedFlat<'b>)]
pub enum HigherCaptured<'b> {
    #[flatten]
    Higher(Higher<'b>),
    Other(&'b u8),
}

#[flat(HigherCaptured<'b>)]
pub enum HigherCapturedFlat<'b> {}

#[test]
fn bound_lifetime_is_not_captured() {
    let (a, b) = (1u8, 2u8);
    let f = HigherCaptured::Higher(Higher::Fn(first)).into_flat();
    match f {
        HigherCapturedFlat::Fn(f) => assert_eq!(*f(&a, &b), 1),
        _ => unreachable!(),
    }
    let g: Box<dyn for<'x> Fn(&'x u8, &u8) -> &'x u8> = Box::new(first);
    let f = HigherCaptured::Higher(Higher::Dyn(g)).into_flat();
    match f {
        HigherCapturedFlat::Dyn(f) => assert_eq!(*f(&a, &b), 1),
        _ => unreachable!(),
    }
    let back = HigherCaptured::from_flat(HigherCapturedFlat::Fn(first));
    assert!(matches!(back, HigherCaptured::Higher(Higher::Fn(_))));
}

// The flat enum's own lifetimes are not shadowed by a binder either.
#[into_flat(HigherStaticFlat<'b>)]
pub enum HigherStatic<'b> {
    #[flatten]
    Higher(Higher<'static>),
    Other(&'b u8),
}

#[flat(HigherStatic<'b>)]
pub enum HigherStaticFlat<'b> {}

#[test]
fn bound_lifetime_does_not_shadow_the_flat_enum() {
    let f = HigherStatic::Higher(Higher::Fn(first)).into_flat();
    assert!(matches!(f, HigherStaticFlat::Fn(_)));
    assert!(matches!(
        HigherStatic::Other(&1).into_flat(),
        HigherStaticFlat::Other(&1)
    ));
}

// `T::Assoc` where `Assoc` is declared by a supertrait of the bound of `T`. The trait that
// declares it is not known, so a type of the flat enum (`P`, with its own bounds) is written
// as it is: `P::Assoc`.
pub trait Super {
    type Assoc;
}
pub trait Sub: Super {}
impl Super for u8 {
    type Assoc = u16;
}
impl Sub for u8 {}

#[derive(Announce)]
#[announce(crate = ::flat_enum::announce)]
pub enum FromSuper<T: crate::Sub> {
    V(T::Assoc),
    W,
}

#[into_flat(SuperOuterFlat<P>)]
pub enum SuperOuter<P: crate::Sub> {
    #[flatten]
    FromSuper(FromSuper<P>),
}

#[flat(SuperOuter<P>)]
pub enum SuperOuterFlat<P: crate::Sub> {}

#[test]
fn associated_type_of_a_supertrait_through_a_parameter() {
    match SuperOuter::<u8>::FromSuper(FromSuper::V(7)).into_flat() {
        SuperOuterFlat::V(n) => assert_eq!(n, 7u16),
        SuperOuterFlat::W => unreachable!(),
    }
    assert!(matches!(
        SuperOuter::<u8>::from_flat(SuperOuterFlat::V(1)),
        SuperOuter::FromSuper(FromSuper::V(1))
    ));
}

// With a concrete type, the supertrait is named in the definition.
#[derive(Announce)]
#[announce(crate = ::flat_enum::announce)]
pub enum FromSuperExplicit<T: crate::Sub> {
    V(<T as crate::Super>::Assoc),
}

#[into_flat(SuperExplicitOuterFlat)]
pub enum SuperExplicitOuter {
    #[flatten]
    FromSuperExplicit(FromSuperExplicit<u8>),
}

#[flat(SuperExplicitOuter)]
pub enum SuperExplicitOuterFlat {}

#[test]
fn associated_type_of_a_supertrait_written_with_the_trait() {
    let SuperExplicitOuterFlat::V(n) =
        SuperExplicitOuter::FromSuperExplicit(FromSuperExplicit::V(8)).into_flat();
    assert_eq!(n, 8u16);
}

// `Sized` and `?Sized` are not trait bounds that `T::Assoc` could belong to, and a bound
// written in the parameter list and again in the `where` clause is one bound.
#[derive(Announce)]
#[announce(crate = ::flat_enum::announce)]
pub enum SizedBounds<
    T: Sized + crate::Tr,
    U: crate::Tr,
    V: crate::Tr + crate::Tr,
    W: ?Sized + crate::Tr,
> where
    T: crate::Tr,
    U: ::core::marker::Sized,
    V: crate::Tr,
{
    S(T::A, U::A, V::A, W::A),
}

#[into_flat(SizedBoundsOuterFlat)]
pub enum SizedBoundsOuter {
    #[flatten]
    SizedBounds(SizedBounds<u8, u8, u8, u8>),
}

#[flat(SizedBoundsOuter)]
pub enum SizedBoundsOuterFlat {}

#[test]
fn sized_and_repeated_bounds_are_ignored() {
    let SizedBoundsOuterFlat::S(a, b, c, d) =
        SizedBoundsOuter::SizedBounds(SizedBounds::S(1, 2, 3, 4)).into_flat();
    let (a, b, c, d): (u16, u16, u16, u16) = (a, b, c, d);
    assert_eq!((a, b, c, d), (1, 2, 3, 4));
}

// A bound of the `Fn` family has no `<_ as Trait>` form, but a parameter of the flat enum
// brings its own bound along.
#[derive(Announce)]
#[announce(crate = ::flat_enum::announce)]
pub enum Callable<F: ::core::ops::Fn(u8) -> R, R> {
    Call(F),
    Out(F::Output),
}

#[into_flat(CallableOuterFlat<F, R>)]
pub enum CallableOuter<F: ::core::ops::Fn(u8) -> R, R> {
    #[flatten]
    Callable(Callable<F, R>),
}

#[flat(CallableOuter<F, R>)]
pub enum CallableOuterFlat<F: ::core::ops::Fn(u8) -> R, R> {}

#[test]
fn fn_bound_through_a_parameter() {
    type Outer = CallableOuter<fn(u8) -> u32, u32>;
    match Outer::Callable(Callable::Out(5)).into_flat() {
        CallableOuterFlat::Out(n) => assert_eq!(n, 5u32),
        CallableOuterFlat::Call(_) => unreachable!(),
    }
    let back = Outer::from_flat(CallableOuterFlat::Call(|n| n.into()));
    match back {
        CallableOuter::Callable(Callable::Call(f)) => assert_eq!(f(4), 4),
        _ => unreachable!(),
    }
}
