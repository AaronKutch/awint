//! This crate contains common developer utilities for crates within the `awint`
//! system, such as macros that needed a separate crate because
//! `#[macro_export]` unconditionally causes macros to be publicly accessible.
//! In rare circumstances, someone might want to use the items here for new
//! storage types or highly optimized routines, but most users should never have
//! to interact with this. Be aware that safety requirements can change over
//! time, check `bits.rs` under `awint_core`.
//!
//! There is a hidden reexport of this crate for `awint_core`, `awint_ext`, and
//! `awint`.

#![no_std]
// TODO
#![cfg_attr(feature = "const_support", feature(const_slice_from_raw_parts_mut))]
#![cfg_attr(feature = "const_support", feature(const_mut_refs))]
// not const and tends to be longer
#![allow(clippy::manual_range_contains)]
#![allow(clippy::needless_range_loop)]

// TODO when const traits are stabilized, try introducing the `BitsInternals`
// trait again

mod macros;
mod raw_bits;
mod serde_common;
mod widening;

use core::num::NonZeroUsize;

pub use raw_bits::{CustomDst, RawBits, RawStackBits};
pub use serde_common::*;
pub use widening::{dd_division, widen_add, widen_mul_add, widening_mul_add_u128};

// If more than one flag is active it will cause an error because two `Digits`
// are defined. However, we have this one duplication check in case of trying to
// use `--all-features`.
#[cfg(all(feature = "u8_digits", feature = "u64_digits"))]
compile_error!(
    "Attempted to activate multiple `*_digits` features at the same time. This is likely because \
     `--all-features` was used, which does not work for `awint`."
);

/// The basic element of the internal slice in `Bits`. This should be a type
/// alias of the unsigned integer of the architecture's registers. On most
/// architectures, this is simply `usize`, however there are cases such as AVR
/// where the pointer size is 16 bits but the register size is 8 bits. If this
/// were not register size, it can incur excessive unrolling or underutilization
/// for every loop in the internals.
#[cfg(not(any(
    feature = "u8_digits",
    feature = "u16_digits",
    feature = "u32_digits",
    feature = "u64_digits",
    feature = "u128_digits",
    target_arch = "avr",
)))]
pub type Digit = usize;
#[cfg(any(feature = "u8_digits", target_arch = "avr"))]
pub type Digit = u8;
#[cfg(feature = "u16_digits")]
pub type Digit = u16;
#[cfg(feature = "u32_digits")]
pub type Digit = u32;
#[cfg(feature = "u64_digits")]
pub type Digit = u64;
#[cfg(feature = "u128_digits")]
pub type Digit = u128;

/// Signed version of `Digit`
#[cfg(not(any(
    feature = "u8_digits",
    feature = "u16_digits",
    feature = "u32_digits",
    feature = "u64_digits",
    feature = "u128_digits",
    target_arch = "avr",
)))]
pub type IDigit = isize;
#[cfg(any(feature = "u8_digits", target_arch = "avr"))]
pub type IDigit = i8;
#[cfg(feature = "u16_digits")]
pub type IDigit = i16;
#[cfg(feature = "u32_digits")]
pub type IDigit = i32;
#[cfg(feature = "u64_digits")]
pub type IDigit = i64;
#[cfg(feature = "u128_digits")]
pub type IDigit = i128;

/// Bitwidth of a `Digit`
pub const BITS: usize = Digit::BITS as usize;

/// Maximum value of a `Digit`
pub const MAX: Digit = Digit::MAX;

/// Number of bytes in a `Digit`
pub const DIGIT_BYTES: usize = (Digit::BITS / u8::BITS) as usize;

/// Number of bits in a `usize`
pub const USIZE_BITS: usize = usize::BITS as usize;

/// Subset of `awint::awi`
pub mod awi {
    // everything except for `char`, `str`, `f32`, and `f64`
    pub use core::{
        assert, assert_eq, assert_ne,
        primitive::{bool, i128, i16, i32, i64, i8, isize, u128, u16, u32, u64, u8, usize},
    };

    pub use Option::{self, None, Some};
    pub use Result::{self, Err, Ok};

    pub use crate::bw;
}

/// Utility free function for converting a `usize` to a `NonZeroUsize`. This is
/// mainly intended for usage with literals, and shouldn't be used for fallible
/// conversions.
///
/// # Panics
///
/// If `w == 0`, this function will panic.
#[inline]
#[track_caller]
#[must_use]
pub const fn bw(w: usize) -> NonZeroUsize {
    match NonZeroUsize::new(w) {
        None => {
            panic!("tried to construct an invalid bitwidth of 0 using the `awint::bw` function")
        }
        Some(w) => w,
    }
}

/// Returns the number of extra bits given `w`
#[inline]
pub const fn extra_u(w: usize) -> usize {
    w & (BITS - 1)
}

/// Returns the number of _whole_ digits (not including a digit with unused
/// bits) given `w`
#[inline]
pub const fn digits_u(w: usize) -> usize {
    w.wrapping_shr(BITS.trailing_zeros())
}

/// Returns the number of extra bits given `w`
#[inline]
pub const fn extra(w: NonZeroUsize) -> usize {
    extra_u(w.get())
}

/// Returns the number of _whole_ digits (not including a digit with unused
/// bits) given `w`
#[inline]
pub const fn digits(w: NonZeroUsize) -> usize {
    digits_u(w.get())
}

/// Returns the number of `Digit`s needed to represent `w`, including any
/// digit with unused bits
#[inline]
pub const fn total_digits(w: NonZeroUsize) -> NonZeroUsize {
    // Safety: if `digits(w)` is zero, `extra(w)` must be nonzero
    unsafe { NonZeroUsize::new_unchecked(digits(w).wrapping_add((extra(w) != 0) as usize)) }
}

/// Location for an item in the source code. This is essentially a
/// `std::panic::Location<'static>` with all public fields.
#[derive(Debug, Clone, Copy)]
pub struct Location {
    pub file: &'static str,
    pub line: u32,
    pub col: u32,
}
