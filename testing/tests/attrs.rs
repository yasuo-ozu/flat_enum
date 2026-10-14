//! Only the docs of a field are copied to the flat enum. Another attribute on it belongs to a
//! derive of the nested enum (`#[serde(skip)]`), which the flat enum does not have.
use flat_enum::announce::Announce;
use flat_enum::{flat, into_flat, IntoFlat};
use test_flat_enum_helper::Helper;

#[derive(Announce, Helper, Debug, PartialEq)]
#[announce(crate = ::flat_enum::announce, docs = true)]
pub enum Inner {
    /// A tuple variant.
    V(#[helper(skip)] u8),
    W {
        /// A named field.
        #[helper(rename = "y")]
        x: u8,
    },
}

#[into_flat(OuterFlat)]
#[derive(Debug, PartialEq)]
pub enum Outer {
    #[flatten]
    Inner(Inner),
}

#[flat(Outer)]
#[derive(Debug, PartialEq)]
pub enum OuterFlat {}

#[test]
fn field_attributes_are_not_copied() {
    assert_eq!(Outer::Inner(Inner::V(1)).into_flat(), OuterFlat::V(1));
    assert_eq!(
        Outer::from_flat(OuterFlat::W { x: 2 }),
        Outer::Inner(Inner::W { x: 2 })
    );
}
