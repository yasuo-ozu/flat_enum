// The type of a `#[flatten]` field must be an announced enum. The error is reported once, by
// `#[into_flat]`: `#[flat]` emits the flat enum without the variants of the struct, but with the
// others.
use flat_enum::announce::Announce;
use flat_enum::{flat, into_flat};

#[derive(Announce)]
#[announce(crate = ::flat_enum::announce)]
pub struct NotEnum {
    pub x: u8,
}

#[derive(Announce)]
#[announce(crate = ::flat_enum::announce)]
pub enum E {
    V(u8),
}

#[into_flat(OuterFlat)]
pub enum Outer {
    A(u8),
    #[flatten]
    E(E),
    #[flatten]
    N(NotEnum),
}

#[flat(Outer)]
pub enum OuterFlat {}

fn main() {
    let _ = (OuterFlat::A(0), OuterFlat::V(0));
}
