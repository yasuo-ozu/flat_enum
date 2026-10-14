use flat_enum::announce::Announce;
use flat_enum::{flat, into_flat};
#[derive(Announce)]
#[announce(crate = ::flat_enum::announce, allow_std_prelude = true)]
pub enum Enum1<A> {
    E1(A),
    E2(),
    E3(String),
}

#[into_flat(Enum2Flat<A>)]
pub enum Enum2<A> {
    #[flatten]
    Enum1(Enum1<A>),
    E4,
}

#[flat(Enum2<A>)]
pub enum Enum2Flat<A> {}
