# Arbitrary Width Integers

This system of crates together forms a kind of big-integer library with separated storage and
functional structs, manually controlled bitwidth, and bitwidth dependent operations. Instead of one
struct that has all of the allocation and functional capabilities, there are two storage types which
manage allocation, `InlAwi` and `ExtAwi`, and a common `Bits` reference type that manages
arithmetical functionality. Most operations on `Bits` are const and have no allocations. `Bits`
backed by `InlAwi` can perform big-integer arithmetic both at compile time and in a `no-std` runtime
without any allocator at all. `Bits` backed by `ExtAwi` can use dynamic bitwidths at runtime. If a
function is written purely in terms of `Bits`, then any mix of `InlAwi`s and `ExtAwi`s can be used
as arguments to that function.

A generic `FP` struct for fixed point numbers is also included, adding more functions for it is
currently a WIP.

`Bits` and `InlAwi` are provided by the `awint_core` crate.
`ExtAwi` and `FP` is provided by the `awint_ext` crate. The reason for this split is to provide
maximum flexibility to `no-std` and `no-alloc` use cases. `ExtAwi` is not within `awint_core` under
a feature flag, because if a no-`alloc` project depended on both `awint_core` and `awint_macros`
(which requires `ExtAwi`), the flag would be activated for the common compilation of `awint_core`.
The `awint_macros` crate is a proc-macro crate with several construction utilities.
The `awint_dag` crate is a WIP.
The `awint` crate compiles these interfaces together and enables or disables different parts of the
system depending on these feature flags:

- "const_support" turns on nightly features that are needed for many functions to be `const`
- "alloc" turns on parts that require an allocator
- "std" turns on parts that require std
- "rand_support" turns on a dependency to `rand_core` without its default features
- "serde_support" turns on a dependency to `serde` without its default features

Note: By default, "const_support" and "std" are turned on, use `default-features = false` and
select specific features to avoid requiring nightly.

NOTE: As of Rust 1.64, if you try to use "const_support" with the macros you may get strange
`erroneous constant used` and `deref_mut` errors unless you add all of
```
#![feature(const_trait_impl)]
#![feature(const_mut_refs)]
#![feature(const_option)]
```
to _all_ of the crate roots where you use the macros in `const` contexts.

## Planned Features

These are currently unimplemented because of other developments and improvements that are being
prioritized. Please open an issue or PR if you would like these implemented faster.

- We need some kind of macro for handling fallible points, instead of `unwrap` everywhere or `?`
  operators that make it difficult to determine panic positions, have a macro find locations of `?`
  operators and do stuff from there.
- We need a macro for simpler syntax. The base `_assign` functions can have virtual counterparts
  (e.g. `x.add_assign(y)` would have the alternative `z = x.add(y)` or `z = x + y`) and the macro
  optimizes storage creation and routing.
- A hybrid stack/heap allocated type like what `smallvec` does
- A higher level `Awi` wrapper around `ExtAwi` with more traditional big-integer library functions
   such as a dynamic sign and automatically resizing bitwidth. This higher level wrapper keeps track
   of leading zeros and ones to speed up operations on very large bitwidth integers with small
   numerical value.
- Add a `const` Karatsuba algorithm to multiplication if possible, or add a `fast_mul` function to
   `awint_ext`
- Better string serialization and deserialization performance. Most basic numerical functions are
   well optimized, but the serialization performance is currently very bad compared to what is
   possible.
- Add custom allocator parameter to `ExtAwi`
- Certain formatting and serialization trait impls need more work.
- Make "const_support" compile on stable. Almost every unstable feature used by these crates is some
  kind of `const` feature, and will hopefully be stabilized soon.
