//! The flat enum has the generic parameters of its declaration: constants, bounds and the
//! `where` clause are kept.
use flat_enum::announce::Announce;
use flat_enum::{flat, into_flat, IntoFlat};

#[derive(Announce)]
#[announce(crate = ::flat_enum::announce)]
pub enum Inner<T, const N: usize> {
    V([T; N]),
}

#[into_flat(ConstFlat<M>)]
pub enum Const<const M: usize> {
    #[flatten]
    X(Inner<u8, M>),
    Y,
}

#[flat(Const<M>)]
pub enum ConstFlat<const M: usize> {}

#[test]
fn const_parameter() {
    let f = Const::<2>::X(Inner::V([1, 2])).into_flat();
    assert!(matches!(f, ConstFlat::<2>::V([1, 2])));
    assert!(matches!(Const::<2>::from_flat(ConstFlat::Y), Const::Y));
}

mod m {
    pub trait Tr {}
    impl Tr for u8 {}
    pub struct L;
}

#[derive(Announce)]
#[announce(crate = ::flat_enum::announce)]
pub enum Plain<T> {
    P(T),
}

#[into_flat(BoundFlat<T>)]
pub enum Bound<T: m::Tr>
where
    T: Clone,
{
    #[flatten]
    X(Plain<T>),
    Y(m::L),
}

#[flat(Bound<T>)]
pub enum BoundFlat<T: m::Tr>
where
    T: Clone, {}

#[test]
fn bounds_and_where_clause() {
    assert!(matches!(
        Bound::<u8>::Y(m::L).into_flat(),
        BoundFlat::Y(m::L)
    ));
    assert!(matches!(
        Bound::<u8>::X(Plain::P(3)).into_flat(),
        BoundFlat::P(3)
    ));
}
