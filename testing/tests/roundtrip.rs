use flat_enum::{flat, into_flat, FlatTarget, IntoFlat};
#[derive(FlatTarget, Debug, PartialEq)]
pub enum Enum1<A> {
    E1(A),
    E2(),
    E3(String),
}
#[derive(FlatTarget, Debug, PartialEq)]
pub enum Enum2<B> {
    E4(B),
    E5,
}
#[derive(Debug, PartialEq)]
#[into_flat(Enum3Flat<A, B>)]
pub enum Enum3<A, B> {
    #[flatten]
    MyEnum1(Enum1<A>),
    #[flatten]
    MyEnum2(Enum2<B>),
    E6,
    E7(u8, u8),
}
#[flat(Enum3<A, B>)]
#[derive(FlatTarget, Debug, PartialEq)]
pub enum Enum3Flat<A, B> {}
#[test]
fn roundtrip() {
    let v: Enum3<u32, i8> = Enum3::MyEnum1(Enum1::E3("x".into()));
    let f = v.into_flat();
    assert_eq!(f, Enum3Flat::E3("x".into()));
    assert_eq!(Enum3::from_flat(f), Enum3::MyEnum1(Enum1::E3("x".into())));
    for f in [
        Enum3Flat::<u32, i8>::E1(7),
        Enum3Flat::E2(),
        Enum3Flat::E4(-1),
        Enum3Flat::E5,
        Enum3Flat::E6,
        Enum3Flat::E7(1, 2),
    ] {
        let s = format!("{:?}", f);
        let back = Enum3::from_flat(f);
        assert_eq!(format!("{:?}", back.into_flat()), s);
    }
    assert_eq!(
        Enum3::<u32, i8>::MyEnum2(Enum2::E5).into_flat(),
        Enum3Flat::E5
    );
}
