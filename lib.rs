pub use flat_enum_macro::{flat, into_flat, FlatTarget};

/// Marker trait implemented with `#[derive(FlatTarget)]`.
///
/// Describes that the enum is used with `#[flat]` attribute in #[flat_enum] macro.
/// For details, see [`flat_enum`]
pub unsafe trait FlatTarget {}

/// Leak `N`th type used in enum variants.
#[doc(hidden)]
pub unsafe trait Leak<const N: usize, EnumTypeParams>: FlatTarget {
    type Ty;
}

/// Implemented with [`flat_enum`].
pub unsafe trait IntoFlat {
    type Flat: Flat<Structured = Self>;
    fn into_flat(self) -> Self::Flat;
    fn from_flat(_: Self::Flat) -> Self;
}

pub unsafe trait Flat {
    type Structured: IntoFlat<Flat = Self>;
}
