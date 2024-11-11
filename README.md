# flat_enum crate [![Latest Version]][crates.io] [![Documentation]][docs.rs] [![GitHub Actions]][actions]

[Latest Version]: https://img.shields.io/crates/v/flat_enum.svg
[crates.io]: https://crates.io/crates/flat_enum
[Documentation]: https://img.shields.io/docsrs/flat_enum
[docs.rs]: https://docs.rs/flat_enum/latest/flat_enum/
[GitHub Actions]: https://github.com/yasuo-ozu/flat_enum/actions/workflows/rust.yml/badge.svg
[actions]: https://github.com/yasuo-ozu/flat_enum/actions/workflows/rust.yml

This crate expands nesting enums. See the example:

```
# use flat_enum::{flat, into_flat, FlatTarget};
#[derive(FlatTarget)]
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

# Motivation

## Memory compaction

In Rust's enum representation on memory, we have `std::mem::Disctiminant` value in addition to the field values of each variants. If two enums are nesting, it should have two discriminants on memory. The compiler's optimization algorithm does not do such work.

This crate gives a way to generate flattened enum automatically to deal with the problem.

## Syntax sugar

When using a value of nested enum types in match-like expression, the matchers are easily to become complex. The flattened enum solves that.
