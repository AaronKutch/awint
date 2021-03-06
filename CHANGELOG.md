# Changelog

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
- Added the portable `Bits::u8_slice_assign` and `Bits::to_u8_slice` functions.
- The hidden functions `Bits::as_bytes_full_width_nonportable` and its mutable counterpart were
  added for situations where direct byte slice references are needed, but note that these need a lot
  of special handling to make them portable.

## [0.2.0] - 2021-08-20
### Fixes
- Fixed that version 0.1 was broken by Rustc 1.59.0-nightly.

### Changes
- Updated to 2021 edition
- `neg_assign` now takes a boolean that conditionally activates it
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
- Added `lut_set`, `neg_add_assign`, `mul_assign`, `arb_umul_add_assign`, and `arb_imul_add_assign`
  to `Bits`
- Added `from_bytes_general` and `from_str_general` to `ExtAwi`
