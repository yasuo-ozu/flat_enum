// The argument of `#[into_flat]` names the flat enum, which is defined in the same module.
// The error is reported once, and the last segment is used so that `#[flat]` still works.
use flat_enum::{flat, into_flat};

#[into_flat(self::OuterFlat)]
pub enum Outer {
    A(u8),
}

#[flat(Outer)]
pub enum OuterFlat {}

fn main() {
    let _ = OuterFlat::A(0);
}
