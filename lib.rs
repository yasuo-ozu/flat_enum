#![doc = include_str!("README.md")]

/// This attribute macro implements trait [`Flat`] on the speficied enum. The definition body
/// should be empty, thus it is generated from corresponding structured enum automatically.
///
/// ```
/// # use flat_enum::{flat, into_flat, FlatTarget};
/// # #[derive(FlatTarget)]
/// # pub enum Enum1<A> {
/// #     E1(A),
/// #     E2(),
/// #     E3(String),
/// # }
/// #[into_flat(Enum2Flat<A>)]
/// pub enum Enum2<A> {
///     #[flatten]
///     Enum1(Enum1<A>),
///     E4,
/// }
///
/// #[flat(Enum2<A>)]
/// pub enum Enum2Flat<A> {}
/// ```
///
/// ## Arguments
///
/// Specify the coresponding structured enum (which has [`IntoFlat`] trait implementation)
/// as the first arguments.
/// The flat enum should be exist in the same context (same module, function, or block context).
///
/// You can specify the `flat_enum` crate using atmark syntax like
/// `#[flat(Enum2<A> @ ::flat_enum)]`.
pub use flat_enum_macro::flat;

/// This attribute macro implements trait [`IntoFlat`] on the speficied enum.
///
/// ```
/// # use flat_enum::{flat, into_flat, FlatTarget};
/// # #[derive(FlatTarget)]
/// # pub enum Enum1<A> {
/// #     E1(A),
/// #     E2(),
/// #     E3(String),
/// # }
/// #[into_flat(Enum2Flat<A>)]
/// pub enum Enum2<A> {
///     #[flatten]
///     Enum1(Enum1<A>),
///     E4,
/// }
///
/// #[flat(Enum2<A>)]
/// pub enum Enum2Flat<A> {}
/// ```
///
/// ## Arguments
///
/// Specify the flat enum (which has [`Flat`] trait implementation implemented with [`flat`]
/// macro) as the first arguments.
/// The flat enum should be exist in the same context (same module, function, or block context).
///
/// You can specify the `flat_enum` crate using atmark syntax like
/// `#[into_flat(Enum2Flat<A> @ ::flat_enum)]`.
///
/// ## `#[flatten]` attribute
///
/// Due to the effect of `#[into_flat]` macro, variants defined with `#[flatten]`
/// attribute is expanded in the corresponding flat enum. The variant should have tuple-like
/// fields, and just one field with type, which has [`FlatTarget`] trait implementation defined
/// with `#[derive(FlatTarget)]`.
pub use flat_enum_macro::into_flat;

/// Implements trait [`FlatTarget`] on the specified enum. This trait is required to be
/// used as the field type of nesting enum variant augmented with `#[flatten]` attribute
/// in `#[into_flat]` enum.
pub use flat_enum_macro::FlatTarget;

/// Marker trait implemented with `#[derive(FlatTarget)]`.
pub unsafe trait FlatTarget {}

/// Leak `N`th type used in enum variants.
#[doc(hidden)]
pub unsafe trait Leak<const N: usize, EnumTypeParams>: FlatTarget {
    type Ty;
}

/// See [`into_flat`]
pub unsafe trait IntoFlat {
    type Flat: Flat<Structured = Self>;
    fn into_flat(self) -> Self::Flat;
    fn from_flat(_: Self::Flat) -> Self;
}

/// See [`flat`]
pub unsafe trait Flat {
    type Structured: IntoFlat<Flat = Self>;
}

#[cfg(feature = "testing")]
pub mod testing {
    use super::FlatTarget;
    #[derive(FlatTarget)]
    pub enum Enum6<'a, const N: usize, A> {
        E8(&'a [A; N]),
    }
}
