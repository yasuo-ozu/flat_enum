use flat_enum::{flat_enum, ToBeFlatten};
use test_flat_enum::Enum6;

#[derive(ToBeFlatten)]
pub enum Enum1<A> {
    E1(A),
    E2(),
    E3(String),
}

#[derive(ToBeFlatten)]
pub enum Enum2<B> {
    E4(B),
    E5(),
}

#[flat_enum]
#[derive(ToBeFlatten)]
pub enum Enum3<A, B> {
    #[flatten]
    MyEnum1(Enum1<A>),
    #[flatten]
    MyEnum2(Enum2<B>),
    E6,
}

#[test]
fn test_enum3() {}

mod m1 {
    use flat_enum::ToBeFlatten;
    #[derive(ToBeFlatten)]
    pub enum Enum4<'a, const N: usize, A> {
        E7(&'a str, [A; N]),
    }
}

pub mod m2 {
    use flat_enum::flat_enum;
    #[flat_enum]
    pub enum Enum5<'a, A> {
        #[flatten]
        MyEnum3(super::Enum3<A, A>),
        #[flatten]
        MyEnum4(super::m1::Enum4<'a, 3, A>),
        #[flatten]
        MyEnum6(super::Enum6<'a, 4, A>),
    }
}
