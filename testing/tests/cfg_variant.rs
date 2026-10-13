//! `#[into_flat]` works from the cfg-evaluated enum: a cfg'd-out variant (plain or
//! `#[flatten]`) is not part of the `IntoFlat` impl.
use flat_enum::{flat, into_flat, FlatTarget, IntoFlat};

#[derive(FlatTarget, Debug, PartialEq)]
pub enum Inner {
    I1(u8),
    I2,
}

#[derive(FlatTarget, Debug, PartialEq)]
pub enum Gone {
    G1,
}

#[into_flat(OuterFlat)]
#[derive(Debug, PartialEq)]
pub enum Outer {
    #[flatten]
    Inner(Inner),
    #[cfg(any())]
    Missing(NoSuchType),
    #[cfg(any())]
    #[flatten]
    Gone(Gone),
    O1(u16),
}

#[flat(Outer)]
#[derive(Debug, PartialEq)]
pub enum OuterFlat {}

#[test]
fn cfg_variant_roundtrip() {
    let o = Outer::Inner(Inner::I1(3));
    let f = o.into_flat();
    assert_eq!(f, OuterFlat::I1(3));
    assert_eq!(Outer::from_flat(f), Outer::Inner(Inner::I1(3)));
    assert_eq!(Outer::O1(7).into_flat(), OuterFlat::O1(7));
    assert_eq!(Outer::from_flat(OuterFlat::I2), Outer::Inner(Inner::I2));
}
