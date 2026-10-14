# flat_enum crate [![Latest Version]][crates.io] [![Documentation]][docs.rs] [![GitHub Actions]][actions]

[Latest Version]: https://img.shields.io/crates/v/flat_enum.svg
[crates.io]: https://crates.io/crates/flat_enum
[Documentation]: https://img.shields.io/docsrs/flat_enum
[docs.rs]: https://docs.rs/flat_enum/latest/flat_enum/
[GitHub Actions]: https://github.com/yasuo-ozu/flat_enum/actions/workflows/rust.yml/badge.svg
[actions]: https://github.com/yasuo-ozu/flat_enum/actions/workflows/rust.yml

This crate expands nesting enums. See the example:

```
# use flat_enum::{flat, into_flat};
#[derive(flat_enum::announce::Announce)]
#[announce(crate = ::flat_enum::announce, allow_std_prelude = true)]
pub enum Enum1<A> {
    E1(A),
    E2(),
    E3(String),
}

#[into_flat(Enum2Flat<A>)]
pub enum Enum2<A> {
    #[flatten]
    Enum1(Enum1<A>),
    E4,
}

#[flat(Enum2<A>)]
pub enum Enum2Flat<A> {}
```

In macro invocation, the `Enum2Flat` expands into something like:

```
pub enum Enum2Flat<A> {
    E1(A),
    E2(),
    E3(String),
    E4,
}
```

In this example, `Enum1` and `Enum2` are not required to be defined in the same crate. But `Enum2` and `Enum2Flat` should be defined in the same context (module).

## Announcing the enums

An enum that `#[flatten]` takes is announced with the [announce](https://docs.rs/announce) crate:
`#[derive(Announce)]` or `#[announce]`. Nothing else is needed. flat_enum re-exports the crate as
`flat_enum::announce`, so the enum's crate does not need its own dependency on it; the
`crate = ..` option says where to find it.

A flat enum is announced the same way, so that another enum can flatten it. The order matters:
`#[flat(..)]` has to be the first attribute, above `#[derive(Announce)]` and `#[announce(..)]`,
because it fills in the variants before the enum is announced.

```
# use flat_enum::{flat, into_flat};
# #[derive(flat_enum::announce::Announce)]
# #[announce(crate = ::flat_enum::announce)]
# pub enum Enum1 {
#     E1(u8),
# }
# #[into_flat(Enum2Flat)]
# pub enum Enum2 {
#     #[flatten]
#     Enum1(Enum1),
# }
#[flat(Enum2)]
#[derive(flat_enum::announce::Announce)]
#[announce(crate = ::flat_enum::announce)]
pub enum Enum2Flat {}
```

The fields of the enum are copied to the flat enum, so their types have to mean the same
in the crate that flattens the enum. announce checks that where the enum is defined, and
reports a path that would not resolve there:

- write `crate::Local` or `::other_crate::Type` instead of a path that depends on a `use`;
- add `allow_std_prelude = true` for `String`, `Vec<T>`, `Option<T>` and the like;
- or give announce a [`repeater`](https://docs.rs/announce/latest/announce/attr.announce.html#type-context)
  for the types of the defining crate.

The bounds of the generic parameters, in the parameter list and in the `where` clause, are
checked the same way: the traits have to be absolute paths, `::core::ops::Fn` and
`crate::Tr` rather than `Fn` and `Tr` (only `Sized` and `?Sized` may be bare).

The generic parameters of the enum (lifetimes, types and constants, `Self` included) are
replaced by the arguments of the `#[flatten]` field type: `E<'a, u8, 4>` flattens
`V(&'a [T; N])` to `V(&'a [u8; 4])`. A lifetime that a field type binds itself
(`for<'b> fn(&'b u8)`) is not a parameter; it is renamed when an argument or the flat enum
uses the same name.

An associated type of a parameter can be written `T::Assoc` in a field type. In the flat enum it
becomes:

- `P::Assoc` when the argument for `T` is a generic parameter `P` of the flat enum
  (`#[flatten] V(E<P>)`). The bounds of `P` resolve it, whichever trait declares `Assoc`, a
  supertrait included (the flat enum has the bounds of `T` on `P`, or `E<P>` would not be
  well-formed). If several bounds of `P` declare `Assoc`, rustc reports it as ambiguous
  (E0221); write `<T as Trait>::Assoc` in the definition then;
- `<Arg as Bound>::Assoc` for any other argument `Arg`, when `T` has exactly one trait bound, in
  the parameter list or in the `where` clause (`Sized`, `?Sized` and a bound written twice do not
  count). `Bound` itself has to declare `Assoc`, not one of its supertraits: with
  `trait Sub: Super` and `T: Sub`, a `T::Assoc` of `Super` ends in rustc's error E0576 at the
  flat enum, which says that `Assoc` is not found in trait `Sub`. Write `<T as Super>::Assoc`
  in the definition then;
- an error from flat_enum otherwise, when `T` has no bound, several, a higher-ranked one
  (`for<'a> Tr<'a>`) or one with parenthesized arguments (`Fn(u8) -> R`, whose `Output` belongs
  to `FnOnce`). Write `<T as Trait>::Assoc` then.

An associated constant (`T::LEN` in an array length) is resolved the same way.

The generic parameters of the structured enum (`#[into_flat]`) and of the flat enum
(`#[flat]`) have the same names, as `Enum2<A>` and `Enum2Flat<A>` above: the field types are
written with the names of the structured enum and copied to the flat enum. rustc reports
``cannot find type `A` in this scope`` otherwise.

The docs of the variants of the structured enum are kept in the flat enum. The variants of a
flattened enum keep their docs only if that enum is announced with `docs = true`. Only docs are
copied from the variants and the fields of a flattened enum: other attributes there, such as
`#[serde(skip)]`, belong to a derive of that enum, which the flat enum does not have.

## Troubleshooting

- `cannot find macro `E` in this scope` (also "`E` is in scope, but it is an enum, not a
  macro"), where `E` is the type of a `#[flatten]` field: `E` is not announced. Add
  `#[derive(flat_enum::announce::Announce)]` and `#[announce(crate = ::flat_enum::announce)]`
  to it. If `E` is announced, look for the first error: announce rejects an enum it cannot
  announce (for example a relative type, see above) and does not define the macro then, which
  makes this error follow.
- `cannot find attribute `announce` in this scope` or "non-exhaustive patterns" (E0004) on a flat
  enum: `#[flat(..)]` has to be the first attribute of the flat enum. A `#[derive(..)]` written
  above it, `Announce` or any other (`Debug` gives E0004), and other attributes are applied to
  the enum before `#[flat(..)]` fills in the variants. Put `#[flat(..)]` first, and the derives
  and the other attributes below it.

# Motivation

## Memory compaction

In Rust's enum representation on memory, we have `std::mem::Disctiminant` value in addition to the field values of each variants. If two enums are nesting, it should have two discriminants on memory. The compiler's optimization algorithm does not do such work.

This crate gives a way to generate flattened enum automatically to deal with the problem.

## Syntax sugar

When using a value of nested enum types in match-like expression, the matchers are easily to become complex. The flattened enum solves that.

# Changes

## 0.2.0

`FlatTarget` is removed. The enums that `#[flatten]` takes are announced with
[announce](https://docs.rs/announce) directly, and the trait `Leak` that the derive
implemented for every field of the enum is gone with it: the field types are read from the
announced definition. `IntoFlat` and `Flat` are not `unsafe` traits any more: nothing relies
on them for memory safety (the conversions are exhaustive matches on the variant names), so
`unsafe impl` of them has to become `impl`.

| before | now |
| --- | --- |
| `#[derive(FlatTarget)]` | `#[derive(flat_enum::announce::Announce)]` and `#[announce(crate = ::flat_enum::announce)]`, or `#[flat_enum::announce::announce(crate = ::flat_enum::announce)]` |
| `#[flat_enum(path)]` | `#[announce(crate = path::announce)]`, with `path` where `flat_enum` is, as before |
| `unsafe trait FlatTarget` | none: the enum is announced (it implements `announce::Announce`) |
| `Leak<N, Params>` (hidden) | none: the types come from the announced definition |
| `unsafe trait IntoFlat`, `unsafe trait Flat` | `trait IntoFlat`, `trait Flat` |
| `String`, `Vec<T>`, ... in a field | `allow_std_prelude = true` in `#[announce(..)]`, also on a flat enum that is announced |
| a field type that depends on a `use` of the defining module | `crate::path::Type`, or announce's `repeater` option |
| a bound `T: Tr` that depends on a `use` | `T: crate::Tr`, an absolute path (also in the `where` clause) |
| `T::Assoc` in a field | `T::Assoc` when the argument is a parameter of the flat enum, or `T` has one trait bound that declares `Assoc` itself; `<T as Trait>::Assoc` otherwise |
| a macro type in a field, `m!(T)` | rejected unless the enum has a `repeater` (see below) |

The derive checked nothing about the shape of the enum, so there is no `flat_enum::check`.
`#[into_flat]` and `#[flat]` still report an enum that cannot be flattened where they are used.

Behavior change: a type in a field that is a macro call (`m!(T)`) used to be copied as it
was written, with a `T` inside the call left alone. announce rejects it now, unless the
flattened enum has a `repeater`; then the type is replaced by a type of the repeater that
takes the arguments of the `#[flatten]` field type, so `T` is replaced as well.

`flat_enum::testing::Enum6` (the `testing` feature) is declared as `Enum6<'a, A, const N: usize>`
now, not `Enum6<'a, const N: usize, A>`, because a const parameter has to follow the type
parameters before Rust 1.59 and the crate builds with 1.57. Its argument lists change the same
way (`Enum6<'a, A, 4>`).

The [announce](https://docs.rs/announce) crate is re-exported as `flat_enum::announce`, so it is
part of the public interface: a move to announce 0.2 will be a breaking release of flat_enum as
well. flat_enum 0.2 needs announce 0.1.1 or later.
