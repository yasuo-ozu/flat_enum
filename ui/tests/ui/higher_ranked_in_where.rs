// The same with the bound in the `where` clause.
use flat_enum::announce::Announce;
use flat_enum::{flat, into_flat};

pub trait Tr {
    type A;
}
impl Tr for u8 {
    type A = u16;
}

#[derive(Announce)]
#[announce(crate = ::flat_enum::announce)]
pub enum E<T>
where
    for<'a> T: crate::Tr,
{
    V(T::A),
}

#[into_flat(OuterFlat)]
pub enum Outer {
    #[flatten]
    E(E<u8>),
}

#[flat(Outer)]
pub enum OuterFlat {}

fn main() {}
