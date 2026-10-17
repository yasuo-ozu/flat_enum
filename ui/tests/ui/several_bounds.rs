use flat_enum::announce::announce;
use flat_enum::{flat, into_flat};

pub trait A {
    type X;
}
pub trait B {}
impl A for u8 {
    type X = u16;
}
impl B for u8 {}

#[announce(crate = ::flat_enum::announce)]
pub enum E<T: crate::A + crate::B> {
    V(T::X),
}

#[into_flat(OuterFlat)]
pub enum Outer {
    #[flatten]
    E(E<u8>),
}

#[flat(Outer)]
pub enum OuterFlat {}

fn main() {}
