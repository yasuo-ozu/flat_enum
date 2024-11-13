use flat_enum::{flat,into_flat, FlatTarget};

#[into_flat(EnumAFlat<Q>)]
pub enum EnumA<Q> {
    E1(Q),
    E2(Q, Q),
    E3(Vec<Q>, Vec<u8>),
}

#[flat(EnumA<Q>)]
#[derive(FlatTarget)]
pub enum EnumAFlat<Q> {}

#[into_flat(EnumBFlat<Q>)]
 pub enum EnumB<Q> {
     #[flatten]
     EnumA(EnumAFlat<Q>),
     E4(Q, Q, Q),
     E5(Q),
 }

#[flat(EnumB<Q>)]
#[derive(FlatTarget)]
pub enum EnumBFlat<Q> {}
