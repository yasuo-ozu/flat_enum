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
`crate = ..` option says where to find it. A flat enum is announced the same way, with
`#[flat(..)]` written above `#[derive(Announce)]` (it fills in the variants first).

The fields of the enum are copied to the flat enum, so their types have to mean the same
in the crate that flattens the enum. announce checks that where the enum is defined, and
reports a path that would not resolve there:

- write `crate::Local` or `::other_crate::Type` instead of a path that depends on a `use`;
- add `allow_std_prelude = true` for `String`, `Vec<T>`, `Option<T>` and the like;
- or give announce a [`repeater`](https://docs.rs/announce/latest/announce/attr.announce.html#type-context)
  for the types of the defining crate.

The generic parameters of the enum (lifetimes, types and constants, `Self` included) are
replaced by the arguments of the `#[flatten]` field type: `E<'a, u8, 4>` flattens
`V(&'a [T; N])` to `V(&'a [u8; 4])`.

# Motivation

## Memory compaction

In Rust's enum representation on memory, we have `std::mem::Disctiminant` value in addition to the field values of each variants. If two enums are nesting, it should have two discriminants on memory. The compiler's optimization algorithm does not do such work.

This crate gives a way to generate flattened enum automatically to deal with the problem.

## Syntax sugar

When using a value of nested enum types in match-like expression, the matchers are easily to become complex. The flattened enum solves that.

# Changes

`FlatTarget` is removed. The enums that `#[flatten]` takes are announced with
[announce](https://docs.rs/announce) directly, and the trait `Leak` that the derive
implemented for every field of the enum is gone with it: the field types are read from the
announced definition. The generated `Flat` and `IntoFlat` implementations are the same.

| before | now |
| --- | --- |
| `#[derive(FlatTarget)]` | `#[derive(flat_enum::announce::Announce)]` and `#[announce(crate = ::flat_enum::announce)]`, or `#[flat_enum::announce::announce(crate = ::flat_enum::announce)]` |
| `#[flat_enum(path)]` | `#[announce(crate = path::announce)]`, with `path` where `flat_enum` is, as before |
| `unsafe trait FlatTarget` | none: the enum is announced (it implements `announce::Announce`) |
| `Leak<N, Params>` (hidden) | none: the types come from the announced definition |
| `flat_enum::__private::announce` | `flat_enum::announce` |
| `String`, `Vec<T>`, ... in a field | `allow_std_prelude = true` in `#[announce(..)]`, also on a flat enum that is announced |
| a field type that depends on a `use` of the defining module | `crate::path::Type`, or announce's `repeater` option |

The derive checked nothing about the shape of the enum, so there is no `flat_enum::check`.
`#[into_flat]` and `#[flat]` still report an enum that cannot be flattened where they are used.

Not supported any more: a type in the field of the flattened enum that is only a macro call
mentioning a generic parameter (`m!(T)`); it is copied as it is, and `T` is not replaced.
