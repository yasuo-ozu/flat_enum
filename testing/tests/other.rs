use flat_enum::announce::Announce;
use flat_enum::{flat, into_flat};

#[into_flat(EnumAFlat<Q>)]
pub enum EnumA<Q> {
    E1(Q),
    E2(Q, Q),
    E3(Vec<Q>, Vec<u8>),
}

#[flat(EnumA<Q>)]
#[derive(Announce)]
#[announce(crate = ::flat_enum::announce, allow_std_prelude = true)]
pub enum EnumAFlat<Q> {}

#[into_flat(EnumBFlat<Q>)]
pub enum EnumB<Q> {
    #[flatten]
    EnumA(EnumAFlat<Q>),
    E4(Q, Q, Q),
    E5(Q),
}

#[flat(EnumB<Q>)]
#[derive(Announce)]
#[announce(crate = ::flat_enum::announce, allow_std_prelude = true)]
pub enum EnumBFlat<Q> {}
