pub use flat_enum_macro::{flat_enum, ToBeFlatten};

/// Marker trait implemented with `#[derive(ToBeFlatten)]`.
///
/// Describes that the enum is used with `#[flatten]` attribute in #[flat_enum] macro.
/// For details, see [`flat_enum`]
pub unsafe trait ToBeFlatten {}

/// Leak `N`th type used in enum variants.
#[doc(hidden)]
pub unsafe trait Leak<const N: usize, EnumTypeParams>: ToBeFlatten {
    type Ty;
}
