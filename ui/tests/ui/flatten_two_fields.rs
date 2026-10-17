// `#[flatten]` needs a variant with a single unnamed field. The error is reported once: the
// variant is treated as a normal one, so `Outer!` and `OuterFlat` are still emitted.
use flat_enum::announce::announce;
use flat_enum::{flat, into_flat};

#[announce(crate = ::flat_enum::announce)]
pub enum E {
    V(u8),
}

#[into_flat(OuterFlat)]
pub enum Outer {
    #[flatten]
    X(E, u8),
}

#[flat(Outer)]
pub enum OuterFlat {}

fn main() {
    let _ = OuterFlat::X(E::V(0), 0);
}
