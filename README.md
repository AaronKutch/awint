# Arbitrary Width Integers

This system of crates together forms a kind of big-integer library with separated storage and
functional structs, manually controlled bitwidth, and bitwidth dependent operations. Instead of one
struct that has all of the allocation and functional capabilities, there are 3 storage types which
manage allocation: `InlAwi`, `ExtAwi`, and `Awi`. There is a common `Bits` reference type that
manages fixed width arithmetical functionality. Most operations on `Bits` are const and have no
allocations. `Bits` backed by `InlAwi` can perform big-integer arithmetic both at compile time and
in a `no-std` runtime without any allocator at all. `Bits` backed by `ExtAwi` can use dynamic
bitwidths at runtime. `Awi` has capacity and cheap bitwidth resizing. If a function is written
purely in terms of `Bits`, then any mix of `InlAwi`s, `ExtAwi`s, and `Awi`s can be used as arguments
to that function with the help of their `Deref<Target = Bits>` impls.

A generic `FP` struct for fixed point numbers is also included, adding more functions for it is
currently a WIP. In the future, `Awi` should also be able to have automatic resizing functions like
in traditional bigint libraries.

`Bits` and `InlAwi` are provided by the `awint_core` crate. `ExtAwi`, `Awi`, and `FP` are provided
by the `awint_ext` crate. The reason for this split is to provide maximum flexibility to `no-std`
and `no-alloc` use cases. `ExtAwi` is not within `awint_core` under a feature flag, because if a
no-`alloc` project depended on both `awint_core` and `awint_macros` (which requires `ExtAwi`), the
flag would be activated for the common compilation of `awint_core`.
The `awint_macros` crate is a proc-macro crate with several construction utilities.
The `awint_dag` crate supplies a way to use `awint` types as a DSL (Domain Specific Language) for
combinational logic.
The `awint` crate compiles these interfaces together and enables or disables different parts of the
system depending on these feature flags:

- "const_support" turns on nightly features that are needed for many functions to be `const`
- "alloc" turns on parts that require an allocator
- "std" turns on parts that require std
- "dag" turns on `awint_dag`
- "try_support" turns on some features required for `dag::Option` and `dag::Result` to fully work
- "debug" turns on some developer functions
- "rand_support" turns on a dependency to `rand_core` without its default features
- "serde_support" turns on a dependency to `serde` without its default features
- "zeroize_support" turns on a dependency to `zeroize` without its default features

Note: By default, "std" and "try_support" is turned on, use `default-features = false` and select
specific features to be more specific.

NOTE: As of Rust 1.70, if you try to use "const_support" with the macros you may get strange
"erroneous constant used" and "deref_mut" errors unless you add all of
```
#![feature(const_trait_impl)]
#![feature(const_mut_refs)]
#![feature(const_option)]
```
to _all_ of the crate roots where you use the macros in `const` contexts.

NOTE: As of some versions of Rust starting around 1.70, "const_support" is unfortunately broken on
nightly (see https://github.com/AaronKutch/awint/issues/19).

## Planned Features

These are currently unimplemented because of other developments and improvements that are being
prioritized. Please open an issue or PR if you would like these implemented faster.

- We need a macro for optimizing 2 input, 1 output functions to our inplace style functions. The
  base inplace assignment functions can have virtual counterparts (e.g. `x.add_(y)` would have the
  alternative `z = x.add(y)` or `z = x + y`) and the macro optimizes storage creation and routing.
- Add some missing functions to the mimicking primitives in `awint_dag`
- There are many things more to be done with `awint_dag`
- Add more functions to `FP`
- Some kind of matching macro
- Add traditional big-integer library functions to `Awi`
- Add a `const` Karatsuba algorithm to multiplication if possible, or add a `fast_mul` function to
   `awint_ext`
- Better string serialization and deserialization performance. Most basic numerical functions are
   well optimized, but the serialization performance is currently very bad compared to what is
   possible.
- Add custom allocator parameter to `ExtAwi`
- Certain formatting and serialization trait impls need more work.
- Make "const_support" compile on stable. Almost every unstable feature used by these crates is some
  kind of `const` feature, and will hopefully be stabilized soon.
