use flat_enum::{flat, into_flat, testing::Enum6, FlatTarget};

#[derive(FlatTarget)]
pub enum Enum1<A> {
    E1(A),
    E2(),
    E3(String),
}

#[allow(unused)]
#[derive(FlatTarget)]
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
#[derive(FlatTarget)]
pub enum Enum3Flat<A, B> {}

#[test]
fn test_enum3() {}

mod m1 {
    use flat_enum::FlatTarget;
    #[derive(FlatTarget)]
    pub enum Enum4<'a, const N: usize, A> {
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
        MyEnum4(super::m1::Enum4<'a, 3, A>),
        #[flatten]
        MyEnum6(super::Enum6<'a, 4, A>),
    }

    #[flat(Enum5<'a, A>)]
    pub enum Enum5Flat<'a, A> {}
}
