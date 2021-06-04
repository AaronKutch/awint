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

`Bits` and `InlAwi` are provided by the `awint_core` crate.
`ExtAwi` is provided by the `awint_ext` crate. The reason for this split is to provide maximum
flexibility to `no-std` and `no-alloc` use cases. `ExtAwi` is not within `awint_core` under a
feature flag, because if a no-`alloc` project depended on both `awint_core` and `awint_macros`
(which requires `ExtAwi`), the flag would be activated for the common compilation of `awint_core`.
The `awint_macros` crate is a proc-macro crate with several utilities to construct `InlAwi`s.
The `awint_dag` crate is a WIP.
The `awint` crate compiles these interfaces together and enables or disables different parts of the
system depending on these feature flags:

- "alloc" turns on parts that require an allocator
- "std" turns on parts that require std. This is turned on by default, although currently nothing
  requires `std`.
- "rand_support" turns on a dependency to `rand_core` without its default features
- "serde_support" turns on a dependency to `serde` without its default features

## Planned Features

These are currently unimplemented because of other developments and improvements that are being
prioritized. Please open an issue or PR if you would like these implemented faster.

- A higher level `Awi` wrapper around `ExtAwi` with more traditional big-integer library functions
   such as a dynamic sign and automatically resizing bitwidth. This higher level wrapper keeps track
   of leading zeros and ones to speed up operations on very large bitwidth integers with small
   numerical value.
- Add a `const` Karatsuba algorithm to multiplication if possible, or add a `fast_mul` free function
  to `awint_ext`
- Add custom allocator parameter to `ExtAwi`
- Do something about the `Display` impls. The `Debug` impls are probably final, but the `Display`
  impl needs more functionality. Some of the serialization trait impls also need work.
