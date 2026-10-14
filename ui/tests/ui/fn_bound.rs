// The associated type of an `Fn(..)` bound is declared by `FnOnce`, and `<_ as Fn(u8) -> R>`
// cannot name the trait: flat_enum reports it. (It used to panic.)
use flat_enum::announce::Announce;
use flat_enum::{flat, into_flat};

#[derive(Announce)]
#[announce(crate = ::flat_enum::announce)]
pub enum E<F: ::core::ops::Fn(u8) -> R, R> {
    V(F::Output, F),
}

#[into_flat(OuterFlat)]
pub enum Outer {
    #[flatten]
    E(E<fn(u8) -> u32, u32>),
}

#[flat(Outer)]
pub enum OuterFlat {}

fn main() {}
