#![doc = include_str!("README.md")]

/// This attribute macro implements trait [`Flat`] on the speficied enum. The definition body
/// should be empty, thus it is generated from corresponding structured enum automatically.
///
/// ```
/// # use flat_enum::{flat, into_flat};
/// # #[flat_enum::announce::announce(crate = ::flat_enum::announce, allow_std_prelude = true)]
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
///
/// ## Announcing the flat enum
///
/// To use the flat enum as the field of a `#[flatten]` variant, announce it with
/// `#[announce(..)]`, above or below `#[flat]`: the enum is announced after `#[flat]` has filled
/// in its variants.
///
/// ```
/// # use flat_enum::{flat, into_flat};
/// # #[flat_enum::announce::announce(crate = ::flat_enum::announce)]
/// # pub enum Enum1 {
/// #     E1(u8),
/// # }
/// # #[into_flat(Enum2Flat)]
/// # pub enum Enum2 {
/// #     #[flatten]
/// #     Enum1(Enum1),
/// # }
/// #[flat(Enum2)]
/// #[flat_enum::announce::announce(crate = ::flat_enum::announce)]
/// pub enum Enum2Flat {}
/// ```
pub use flat_enum_macro::flat;

/// This attribute macro implements trait [`IntoFlat`] on the speficied enum.
///
/// ```
/// # use flat_enum::{flat, into_flat};
/// # #[flat_enum::announce::announce(crate = ::flat_enum::announce, allow_std_prelude = true)]
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
/// fields, and just one field with type, which is an enum announced with
/// `#[announce]` (see the [`announce`] crate). The type may have
/// generic arguments, lifetimes and constants included.
pub use flat_enum_macro::into_flat;

/// The [`announce`] crate, which announces the enums that `#[flatten]` takes:
///
/// ```
/// #[flat_enum::announce::announce(crate = ::flat_enum::announce)]
/// pub enum Inner {
///     A(u8),
///     B,
/// }
/// ```
///
/// An enum that is flattened needs `#[announce]` and nothing else. The
/// types in its fields have to resolve in the calling crate, which announce checks where the enum is
/// defined: write `crate::..` and `::krate::..` paths, add `allow_std_prelude = true` for `String`,
/// `Vec<..>` and the like, or give announce a `repeater` for the types of the defining crate
/// (`#[announce(repeater = crate::Repeater)]`). The traits in the bounds of the generic
/// parameters are absolute paths as well (`T: crate::Tr`, `F: ::core::ops::Fn(u8)`).
///
/// `T::Assoc` in a field becomes `P::Assoc` when the argument for `T` is a generic parameter
/// `P` of the flat enum, and `<Arg as Bound>::Assoc` for another argument `Arg` when `T` has
/// exactly one trait bound. With a concrete argument `Bound` has to declare `Assoc` itself, not
/// a supertrait of it; and when `T` has no bound or several, a higher-ranked one or an
/// `Fn(..)` one, flat_enum reports an error. In those cases write `<T as Trait>::Assoc` in the
/// definition.
///
/// If a `#[flatten]` field type is not announced, rustc reports ``cannot find macro `E` ``;
/// add `#[announce]` to `E`.
pub use announce;

/// See [`into_flat`]
///
/// `#[into_flat]` is the only implementation that is meant to exist. The generated
/// implementation moves every field of a variant to the same field of the variant with the same
/// name, without conversion, so rustc checks it. `into_flat` and `from_flat` are inverses of
/// each other. Nothing relies on the layout of the two enums.
pub trait IntoFlat {
    type Flat: Flat<Structured = Self>;
    fn into_flat(self) -> Self::Flat;
    fn from_flat(_: Self::Flat) -> Self;
}

/// See [`flat`]
///
/// `#[flat]` is the only implementation that is meant to exist. `Structured` is the enum that
/// [`IntoFlat`] flattens into `Self`, as the bound already says.
pub trait Flat {
    type Structured: IntoFlat<Flat = Self>;
}

/// Implementation details used by the generated code. Not a stable interface.
#[doc(hidden)]
pub mod __private {
    pub use flat_enum_macro::{__flat_cb as flat_cb, __into_flat_cb as into_flat_cb};
}

#[cfg(feature = "testing")]
pub mod testing {
    macro_rules! emit_enum6 {
        () => {
            #[$crate::announce::announce(crate = $crate::announce)]
            pub enum Enum6<'a, A, const N: usize> {
                E8(&'a [A; N]),
            }
        };
    }

    emit_enum6!();
}
