//! Named fields. Broken in the macro_rules baseline (missing commas in
//! patterns, wrong argument order of the type lookup); fixed by the announce port.
use flat_enum::announce::Announce;
use flat_enum::{flat, into_flat, IntoFlat};

#[derive(Announce, Debug, PartialEq)]
#[announce(crate = ::flat_enum::announce)]
pub enum N1<A> {
    X { a: A, b: u8 },
    Y(A),
}

#[derive(Debug, PartialEq)]
#[into_flat(N2Flat<A>)]
pub enum N2<A> {
    #[flatten]
    N1(N1<A>),
    W {
        c: u8,
        d: String,
    },
    Z,
}

#[flat(N2<A>)]
#[derive(Debug, PartialEq)]
pub enum N2Flat<A> {}

#[test]
fn named_roundtrip() {
    let f = N2::N1(N1::X { a: 1u32, b: 2 }).into_flat();
    assert_eq!(f, N2Flat::X { a: 1, b: 2 });
    assert_eq!(N2::from_flat(f), N2::N1(N1::X { a: 1, b: 2 }));
    let f = N2::<u32>::W {
        c: 3,
        d: "d".into(),
    }
    .into_flat();
    assert_eq!(
        f,
        N2Flat::W {
            c: 3,
            d: "d".into()
        }
    );
    assert_eq!(N2::from_flat(N2Flat::<u32>::Z), N2::Z);
}
