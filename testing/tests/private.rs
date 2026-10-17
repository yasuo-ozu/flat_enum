//! Non-`pub` definitions: announce emits a local macro_rules + `<vis> use`,
//! produced several expansion levels deep (derive -> #[announce] -> capture).
#![allow(dead_code)]
use flat_enum::announce::announce;
use flat_enum::{flat, into_flat, IntoFlat};

#[announce(crate = ::flat_enum::announce)]
#[derive(Debug, PartialEq)]
enum P1 {
    A(u8),
    B,
}

#[announce(crate = ::flat_enum::announce)]
#[derive(Debug, PartialEq)]
pub(crate) enum P2 {
    C,
}

#[derive(Debug, PartialEq)]
#[into_flat(PFlat)]
enum PS {
    #[flatten]
    X(P1),
    #[flatten]
    Y(P2),
    D,
}

#[flat(PS)]
#[derive(Debug, PartialEq)]
enum PFlat {}

mod sub {
    use flat_enum::{flat, into_flat};
    #[derive(Debug, PartialEq)]
    #[into_flat(QFlat)]
    pub(crate) enum Q {
        #[flatten]
        Z(super::P2),
        E,
    }
    #[flat(Q)]
    #[derive(Debug, PartialEq)]
    pub(crate) enum QFlat {}
}

#[test]
fn private_roundtrip() {
    assert_eq!(PS::X(P1::A(3)).into_flat(), PFlat::A(3));
    assert_eq!(PS::from_flat(PFlat::C), PS::Y(P2::C));
    assert_eq!(sub::Q::Z(P2::C).into_flat(), sub::QFlat::C);
}
