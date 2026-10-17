//! Types of the defining crate through announce's `repeater`. `define_repeater!` needs
//! Rust 1.59, so this test is behind the `repeater` feature (on by default).
use flat_enum::announce::announce;
use flat_enum::{flat, into_flat, IntoFlat};

// Types of the defining crate through announce's `repeater`: `Local` is only relative here.
mod inner {
    #[derive(Debug, PartialEq)]
    pub struct Local(pub u8);
}
use inner::Local;

flat_enum::announce::define_repeater!(pub Repeater);

#[announce(crate = ::flat_enum::announce, repeater = crate::Repeater)]
#[derive(Debug, PartialEq)]
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
