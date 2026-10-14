use flat_enum::announce::Announce;
use flat_enum::{flat, into_flat, testing::Enum6};

#[derive(Announce)]
#[announce(crate = ::flat_enum::announce, allow_std_prelude = true)]
pub enum Enum1<A> {
    E1(A),
    E2(),
    E3(String),
}

#[allow(unused)]
#[derive(Announce)]
#[announce(crate = ::flat_enum::announce)]
pub enum Enum2<B> {
    E4(B),
    E5(),
}

#[allow(unused)]
#[into_flat(Enum3Flat<A, B>)]
pub enum Enum3<A, B> {
    #[flatten]
    MyEnum1(Enum1<A>),
    #[flatten]
    MyEnum2(Enum2<B>),
    E6,
}

#[flat(Enum3<A, B>)]
#[derive(Announce)]
#[announce(crate = ::flat_enum::announce, allow_std_prelude = true)]
pub enum Enum3Flat<A, B> {}

#[test]
fn test_enum3() {}

mod m1 {
    use flat_enum::announce::Announce;
    #[derive(Announce)]
    #[announce(crate = ::flat_enum::announce)]
    pub enum Enum4<'a, A, const N: usize> {
        E7(&'a str, [A; N]),
    }
}

pub mod m2 {
    use flat_enum::{flat, into_flat};
    #[into_flat(Enum5Flat<'a, A>)]
    pub enum Enum5<'a, A> {
        #[flatten]
        MyEnum3(super::Enum3Flat<A, A>),
        #[flatten]
        MyEnum4(super::m1::Enum4<'a, A, 3>),
        #[flatten]
        MyEnum6(super::Enum6<'a, A, 4>),
    }

    #[flat(Enum5<'a, A>)]
    pub enum Enum5Flat<'a, A> {}
}
