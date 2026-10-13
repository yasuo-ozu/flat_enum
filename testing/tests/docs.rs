//! The docs of the variants are kept in the flat enum, so `#![deny(missing_docs)]` is satisfied.
#![deny(missing_docs)]

use flat_enum::announce::Announce;
use flat_enum::{flat, into_flat, IntoFlat};

/// Announced, with docs on its variants.
#[derive(Announce)]
#[announce(crate = ::flat_enum::announce, docs = true)]
pub enum Inner {
    /// An `Inner` variant.
    I(u8),
}

/// The structured enum.
#[into_flat(OuterFlat)]
pub enum Outer {
    /// Flattened.
    #[flatten]
    Inner(Inner),
    /// A unit variant.
    Q,
    /// A tuple variant.
    R(u8),
}

/// The flat enum.
#[flat(Outer)]
pub enum OuterFlat {}

#[test]
fn documented_variants() {
    assert!(matches!(Outer::Q.into_flat(), OuterFlat::Q));
    assert!(matches!(Outer::R(1).into_flat(), OuterFlat::R(1)));
    assert!(matches!(
        Outer::Inner(Inner::I(2)).into_flat(),
        OuterFlat::I(2)
    ));
}
