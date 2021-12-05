# Changelog

## [0.2.0] - date TODO
### Fixes
- Fixed that version 0.1 was broken by Rustc 1.59.0-nightly.

### Changes
- Updated to 2021 edition
- `neg_assign` now takes a boolean that conditionally activates it
- Some string conversion functions no longer include a sign indicator in their output, and accept
  empty strings as 0
- Renamed all `_triop` functions to `_assign` functions. The short divisions are differentiated by
  `_inplace_`.

### Additions
- Implemented `Deref` for `InlAwi` and `ExtAwi`
- Added generic `FP` struct for fixed-point arithmetic and `FPType`
- Added `const_nzbw` and `const_bw` to `InlAwi`
- Added `lut_set`, `neg_add_assign`, `mul_assign`, `arb_umul_add_assign`, and `arb_imul_add_assign`
  to `Bits`
- Added `from_bytes_general` and `from_str_general` to `ExtAwi`
