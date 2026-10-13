//! `Leak` indices by (variant, field) position: the same type spelled
//! `crate::Foo` twice (announce rewrites it to `$crate::Foo` in the payload) and a repeated
//! type. Also a second announcement of a `FlatTarget` enum: the derive's
//! `emit_item = false` implies `marker = false`, so the `Marker` impls do not conflict.
use flat_enum::{flat, into_flat, FlatTarget, IntoFlat};

#[derive(Debug, PartialEq)]
pub struct Foo(pub u8);

#[flat_enum::__private::announce::announce(crate = flat_enum::__private::announce, macro_name = L1Announced)]
#[derive(FlatTarget, Debug, PartialEq)]
pub enum L1 {
    A(crate::Foo),
    B { x: crate::Foo, y: u8, z: u8 },
}

#[derive(Debug, PartialEq)]
#[into_flat(L2Flat)]
pub enum L2 {
    #[flatten]
    L1(L1),
    C(u8),
}

#[flat(L2)]
#[derive(Debug, PartialEq)]
pub enum L2Flat {}

#[test]
fn leak_index_by_position() {
    let f = L2::L1(L1::B {
        x: Foo(1),
        y: 2,
        z: 3,
    })
    .into_flat();
    assert_eq!(
        f,
        L2Flat::B {
            x: Foo(1),
            y: 2,
            z: 3
        }
    );
    assert_eq!(L2::from_flat(L2Flat::A(Foo(4))), L2::L1(L1::A(Foo(4))));
    assert_eq!(L2::C(5).into_flat(), L2Flat::C(5));
}
