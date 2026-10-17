// `Assoc` is declared by a supertrait of the only bound of `T`. flat_enum cannot know that,
// writes `<u8 as Sub>::Assoc`, and rustc reports it. With a parameter of the flat enum for
// the argument it works (see testing/tests/subst.rs); with a concrete type, write
// `<T as Super>::Assoc` in the definition.
use flat_enum::announce::announce;
use flat_enum::{flat, into_flat};

pub trait Super {
    type Assoc;
}
pub trait Sub: Super {}
impl Super for u8 {
    type Assoc = u16;
}
impl Sub for u8 {}

#[announce(crate = ::flat_enum::announce)]
pub enum E<T: crate::Sub> {
    V(T::Assoc),
}

#[into_flat(OuterFlat)]
pub enum Outer {
    #[flatten]
    E(E<u8>),
}

#[flat(Outer)]
pub enum OuterFlat {}

fn main() {}
