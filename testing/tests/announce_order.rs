//! The flat enum is announced after `#[flat]` filled in its variants, with `#[announce]` written
//! below `#[flat]` or above it, next to other attributes.
use flat_enum::announce::announce;
use flat_enum::{flat, into_flat, IntoFlat};

#[announce(crate = ::flat_enum::announce)]
#[derive(Debug, PartialEq)]
pub enum InnerA {
    A(u8),
}

#[announce(crate = ::flat_enum::announce)]
#[derive(Debug, PartialEq)]
pub enum InnerB {
    B(u8),
}

#[into_flat(BelowFlat)]
pub enum Below {
    #[flatten]
    Inner(InnerA),
}

#[flat(Below)]
#[announce(crate = ::flat_enum::announce)]
#[derive(Debug, PartialEq)]
pub enum BelowFlat {}

#[into_flat(AboveFlat)]
pub enum Above {
    #[flatten]
    Inner(InnerB),
}

#[announce(crate = ::flat_enum::announce)]
#[flat(Above)]
#[derive(Debug, PartialEq)]
pub enum AboveFlat {}

#[into_flat(TopFlat)]
#[derive(Debug, PartialEq)]
pub enum Top {
    #[flatten]
    Below(BelowFlat),
    #[flatten]
    Above(AboveFlat),
    C(u8),
}

#[flat(Top)]
#[derive(Debug, PartialEq)]
pub enum TopFlat {}

#[test]
fn announce_above_and_below_flat() {
    assert_eq!(Top::Below(BelowFlat::A(1)).into_flat(), TopFlat::A(1));
    assert_eq!(Top::from_flat(TopFlat::A(2)), Top::Below(BelowFlat::A(2)));
    assert_eq!(Top::Above(AboveFlat::B(3)).into_flat(), TopFlat::B(3));
    assert_eq!(Top::from_flat(TopFlat::B(4)), Top::Above(AboveFlat::B(4)));
    assert_eq!(Top::C(5).into_flat(), TopFlat::C(5));
}
