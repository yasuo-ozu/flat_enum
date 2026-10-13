//! The field types of a nested enum are read from its announced definition; its generic
//! parameters are replaced by the arguments of the `#[flatten]` field type. Each enum below
//! would break a replacement that is naive in one way, and the round trips check the types.
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
    let nested = ArrFlat::<u8>::Nested {
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

// Types of the defining crate through announce's `repeater`: `Local` is only relative here.
mod inner {
    #[derive(Debug, PartialEq)]
    pub struct Local(pub u8);
}
use inner::Local;

flat_enum::announce::define_repeater!(pub Repeater);

#[derive(Announce, Debug, PartialEq)]
#[announce(crate = ::flat_enum::announce, repeater = crate::Repeater)]
pub enum Repeated<T> {
    R(Local, Vec<T>),
}

#[into_flat(RepeatedOuterFlat)]
#[derive(Debug, PartialEq)]
pub enum RepeatedOuter {
    #[flatten]
    Repeated(Repeated<u16>),
}

#[flat(RepeatedOuter)]
#[derive(Debug, PartialEq)]
pub enum RepeatedOuterFlat {}

#[test]
fn repeater() {
    let v = RepeatedOuter::Repeated(Repeated::R(Local(1), vec![2]));
    assert_eq!(v.into_flat(), RepeatedOuterFlat::R(Local(1), vec![2]));
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
