use flat_enum::{flat_enum, FlatTarget};
#[derive(FlatTarget)]
pub enum Enum1<A> {
    E1(A),
    E2(),
    E3(String),
}

#[flat_enum]
#[derive(FlatTarget)]
pub enum Enum2<A> {
    #[flatten]
    Enum1(Enum1<A>),
    E4,
}

#[flat_enum]
pub enum Enum3<A> {
    #[flatten]
    Enum2(Enum2<A>),
}
