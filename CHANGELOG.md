# Changelog

## TODO
### Additions
- Added more `awi` to `dag` `From` impls

### Changes
- Overhauled the epoch system for `awint_dag` and various related things

## [0.12.0] - 2023-08-29
### Crate
- bumped MSRV to 1.70.0
- `triple_arena` 0.12

### Fixes
- Replaced some usages of transmutation with fat pointer `as` casts to prevent a technical subtlety
  that could lead to UB (even if it wouldn't occur with `-Zrandomize-layout` in practice)
- Finally found a "custom" DST workaround to store the bitwidth inline and derive the slice length
  from that instead of using the metadata hack. Note that this currently requires
  `-Zmiri-tree-borrows` for Miri to accept it.

### Additions
- Added `Bits::total_cmp`
- Added `OrdBits`
- Added `Error` impl for `EvalError`

## [0.11.0] - 2023-06-04
### Crate
- Updated "zeroize_support" to use `zeroize` 1.6
- Updated to `triple_arena` 0.9

### Changes
- Made "const_support" not a default feature flag

## [0.10.0] - 2023-03-11
### Fixes
- Fixed that the overflow check in `chars_upper_bound` and `bits_upper_bound` was completely broken
- Greatly improved the efficiency of `awint` on restricted architectures such as AVR
- Fixed that `inlawi!` could cause double the necessary stack usage on all platforms
- Macro constants are now compiled down to `&'static [u8]` or `&'static Bits`
- `awint` should now theoretically work with 128 bit architectures like riscv128

### Additions
- Added `Digit`, a type alias for the underlying storage element for `Bits`. Also added various
  primitive related functions for it.
- Added feature flags to control `Digit`
- Added the `bits` macro for creating `&'static Bits` constants easily
- Enabled `const` PartialEq

### Changes
- Replaced `usize` with `Digit` where applicable. This does not immediately change things for common
  architectures, but `Digit` can be different from `usize` now.
- Renamed `short_` functions to `digit_` functions
- Added missing `_` suffix to `digit_cin_mul_`
- `Digit` has a minimum guaranteed maximum value of `u8::MAX` rather than `u16::MAX`
- `const_as_ref` and `const_as_mut` removed from `InlAwi` and `ExtAwi` (although it still exists as
  a hidden function on `Bits` for macro purposes)
- Many changes to hidden and unstable items

## [0.9.0] - 2023-02-28
### Fixes
- Added a limiter to `FP::to_vec_general` and downstream string formatting to prevent easy resource
  exhaustion problems. Note that `max_ufp` is set to 4096 for default formatters.
- Fixed that '_'s in the fraction of `ExtAwi::from_bytes_general` could cause incorrect results.
- Fixed that `ExtAwi::from_str` could allow effectively empty integers like "_u8"

### Additions
- Added `FP::floating_`
- Added IEEE-754 related items for `FP`
- Added fixed point support to `ExtAwi::from_str` to quickly leverage `ExtAwi::from_str_general` 

## [0.8.0] - 2023-01-17
### Crate
- MSRV 1.66

### Additions
- Added `zeroize` support
- Added mimicking `Option` and `Result` types and `try_support` to support using them with `?`
- Added mimicking assertions and made `awint_dag` actually respect invalid bitwidths and values

### Changes
- Refactored the `awi`, `dag`, and `prelude` modules
- Renamed all `*_assign` functions to `*_` functions
- Renamed `funnel` to `funnel_`
- Renamed `rand_assign_using` to `rand_`
- Renamed `Node` and `Dag` to `OpNode` and `OpDag`
- Updated `triple_arena` version, which changes some things in `awint_dag`
- Many improvements to `awint_dag`

## [0.7.0] - 2022-08-22
### Additions
- Added `#[must_use]` where applicable
- Added `Bits::mux_`
- First workable version of `awint_dag`

### Changes
- Renamed `Bits::lut` to `Bits::lut_`
- Added a second generic to some `FP` functions that allows different `Borrow<Bits>` types to work
  together

## [0.6.0] - 2022-07-20
### Fixes
- Fixed that the infallibility of some macros was being calculated wrong. A few macros now return
  `Option`s to prevent hidden panics and other macros have become infallible.
- Fixed `const_support` for Rust 1.64

### Additions
- Added missing `from_...` and `From<...>` impls to `InlAwi` and `ExtAwi`

## [0.5.0] - 2022-06-09
### Fixes
- Fixed that `to_u8_slice` on big endian platforms did not zero bytes beyond `self.bw()`. There was
  a blind spot in the testing that has been fixed.
- Fixed error E0716 in many cases for the macros.

### Changes
- Overhaul of the macros. Uses proper token tree parsing that fixes many long standing issues.
  Nested macros and complex inner expressions with brackets, commas, and semicolons not belonging to
  the outside macro are now possible. Trailing commas and semicolons are allowed.
- Note: in order for some expressions to remain const, you need to add
  `#![feature(const_trait_impl)]` to your crate root, or else you will run into strange
  `erroneous constant used` and `deref_mut` errors.
- Note: certain reference patterns of a form like `fn(awi_ref: &mut Bits) {cc!(1u8; awi_ref)}` are
  broken by the workaround for E0716. This can be fixed by making the reference mutable
  `fn(mut awi_ref: &mut Bits) {...}`.
- Note: the old specified initialization macros such as `extawi_[init]!(...)` can be replaced by
  `extawi!([init]: ...)`. The old initialization macros also had a feature where a single literal
  with no suffix could be interpreted as a bitwidth (e.x. `inlawi_zero!(64)`), but this
  functionality has been removed and instead fillers should be used (e.x. `inlawi!(zero: ..64)`).
- Implemented `Copy` for `FP<B>` if `B: Copy`

### Additions
- Added more specializations of `Bits::field` and used them to improve macro performance
- Added `Bits::sig` as a shorthand for `x.bw() - x.lz()`
- Added direct `InlAwi::from_X` functions

## [0.4.0] - 2022-04-07
### Fixes
- Fixed a stacked borrows violation in the permutation functions. CI now runs the latest Miri with
  `-Zmiri-strict-provenance` to prevent issues like this from being introduced in the future.
- Fixed the macros in cases where the build architecture pointer size and target architecture
  pointer size is different. The CI successfully runs the full test suite on
  `mips-unknown-linux-gnu` which is a 32 bit big endian architecture.

### Changes
- A few hidden functions were added and removed, and you may need to import `Bits` in more cases
  because of the changes to macros.

## [0.3.0] - 2022-03-01
### Fixes
- Fixed that string deserialization functions with radix higher than 10 could accept chars that they
  shouldn't
- Had to remove `Bits::as_bytes` and `Bits::as_bytes_mut` because they were fundamentally broken on
  big endian architectures. Fixed all affected functions (except `Hash` related ones) so that they
  are actually consistent across architectures.

### Additions
- Added the portable `Bits::u8_slice_` and `Bits::to_u8_slice` functions.
- The hidden functions `Bits::as_bytes_full_width_nonportable` and its mutable counterpart were
  added for situations where direct byte slice references are needed, but note that these need a lot
  of special handling to make them portable.

## [0.2.0] - 2021-08-20
### Fixes
- Fixed that version 0.1 was broken by Rustc 1.59.0-nightly.

### Changes
- Updated to 2021 edition
- `neg_` now takes a boolean that conditionally activates it
- Some string conversion functions no longer include a sign indicator in their output, and accept
  empty strings as 0
- Renamed all `_triop` functions to `_assign` functions. The short divisions are differentiated by
  `_inplace_`.
- Removed standard ops that hid panics.

### Additions
- Implemented `Deref` for `InlAwi` and `ExtAwi`. `const_as_ref` and `const_as_mut` can be elided in
  many circumstances now.
- Added generic `FP` struct for fixed-point arithmetic and `FPType`
- Added `const_nzbw` and `const_bw` to `InlAwi`
- Added `lut_set`, `neg_add_`, `mul_`, `arb_umul_add_`, and `arb_imul_add_`
  to `Bits`
- Added `from_bytes_general` and `from_str_general` to `ExtAwi`
