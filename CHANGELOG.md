# Changelog

## [0.3.0] - 2022-03-01
### Fixes
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
