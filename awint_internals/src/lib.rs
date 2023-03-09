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
// not const and tends to be longer
#![allow(clippy::manual_range_contains)]
#![allow(clippy::needless_range_loop)]

// TODO when const traits are stabilized, try introducing the `BitsInternals`
// trait again

mod macros;
mod serde_common;

use core::num::NonZeroUsize;

pub use serde_common::*;

/// The basic element of the internal slice in `Bits`. This should be a type
/// alias of the unsigned integer of the architecture's registers. On most
/// architectures, this is simply `usize`, however there are cases such as AVR
/// where the pointer size is 16 bits but the register size is 8 bits. If this
/// were not register size, it can incur excessive unrolling or underutilization
/// for every loop in the internals.
pub type Digit = u32;

/// Signed version of `Digit`
pub type IDigit = i32;

/// Bitwidth of a `Digit`
pub const BITS: usize = Digit::BITS as usize;

/// Maximum value of a `Digit`
pub const MAX: Digit = Digit::MAX;

/// Number of bytes in a `Digit`
pub const DIGIT_BYTES: usize = (Digit::BITS / u8::BITS) as usize;

/// Number of metadata digits
pub const METADATA_DIGITS: usize = if USIZE_BITS >= BITS {
    USIZE_BITS / BITS
} else {
    1
};

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

/// Returns the number of `usize` digits needed to represent `w`, including any
/// digit with unused bits
#[inline]
pub const fn regular_digits(w: NonZeroUsize) -> usize {
    digits(w).wrapping_add((extra(w) != 0) as usize)
}

/// Returns `regular_digits + 1` to account for the metadata digits
#[inline]
pub const fn raw_digits(w: usize) -> usize {
    digits_u(w)
        .wrapping_add((extra_u(w) != 0) as usize)
        .wrapping_add(METADATA_DIGITS)
}

/// Checks that the `BW` and `LEN` values are valid for an `InlAwi`.
///
/// # Panics
///
/// If `BW == 0`, `LEN < METADATA_DIGITS + 1`, or the bitwidth is outside the
/// range `(((LEN - METADATA_DIGITS - 1)*BITS) + 1)..=((LEN -
/// METADATA_DIGITS)*BITS)`
pub const fn assert_inlawi_invariants<const BW: usize, const LEN: usize>() {
    if BW == 0 {
        panic!("Tried to create an `InlAwi<BW, LEN>` with `BW == 0`")
    }
    if LEN < METADATA_DIGITS + 1 {
        panic!("Tried to create an `InlAwi<BW, LEN>` with `LEN < METADATA_DIGITS + 1`")
    }
    if BW <= ((LEN - METADATA_DIGITS - 1) * BITS) {
        panic!(
            "Tried to create an `InlAwi<BW, LEN>` with `BW <= Digit::BITS*(LEN - METADATA_DIGITS \
             - 1)`"
        )
    }
    if BW > ((LEN - METADATA_DIGITS) * BITS) {
        panic!(
            "Tried to create an `InlAwi<BW, LEN>` with `BW > Digit::BITS*(LEN - METADATA_DIGITS)`"
        )
    }
}

/// Checks that a raw slice for `InlAwi` construction is correct. Also runs
/// `assert_inlawi_invariants` to check the correctness of the `BW` and `LEN`
/// values.
///
/// # Panics
///
/// If `raw.len() != LEN`, the bitwidth digit is not equal to `BW`, `BW == 0`,
/// `LEN < METADATA_DIGITS + 1`, or the bitwidth is outside the range
/// `(((LEN - METADATA_DIGITS - 1)*BITS) + 1)..=((LEN - METADATA_DIGITS)*BITS)`
pub const fn assert_inlawi_invariants_slice<const BW: usize, const LEN: usize>(raw: &[Digit]) {
    assert_inlawi_invariants::<BW, LEN>();
    if raw.len() != LEN {
        panic!("`length of raw slice does not equal LEN")
    }
    let mut w = 0;
    const_for!(i in {(raw.len() - METADATA_DIGITS)..raw.len()} {
        w |= (raw[i] as usize) << ((i + METADATA_DIGITS - raw.len()) * BITS);
    });
    if w != BW {
        panic!("bitwidth digit does not equal BW")
    }
}

/// Computes x + y + z and returns the widened result as a tuple.
#[inline]
pub const fn widen_add(x: Digit, y: Digit, z: Digit) -> (Digit, Digit) {
    // TODO make sure this is adc on appropriate platforms and also works well on
    // RISCV
    let (sum, carry0) = x.overflowing_add(y);
    let (sum, carry1) = sum.overflowing_add(z);
    (sum, (carry0 as Digit) + (carry1 as Digit))
}

macro_rules! widen_mul_add_internal {
    ($x:ident, $y:ident, $z:ident; 128 => $other:block $($bits:expr, $uD:ident);*;) => {
        match BITS {
            $(
                $bits => {
                    let tmp = ($x as $uD).wrapping_mul($y as $uD).wrapping_add($z as $uD);
                    (tmp as Digit, tmp.wrapping_shr($bits) as Digit)
                }
            )*
            128 => $other
            _ => panic!("Unsupported pointer size"),
        }
    };
}

pub const fn widening_mul_add_u128(lhs: u128, rhs: u128, add: u128) -> (u128, u128) {
    //                       [rhs_hi]  [rhs_lo]
    //                       [lhs_hi]  [lhs_lo]
    //                     X___________________
    //                       [------tmp0------]
    //             [------tmp1------]
    //             [------tmp2------]
    //     [------tmp3------]
    //                       [-------add------]
    // +_______________________________________
    //                       [------sum0------]
    //     [------sum1------]

    let lhs_lo = lhs as u64;
    let rhs_lo = rhs as u64;
    let lhs_hi = (lhs.wrapping_shr(64)) as u64;
    let rhs_hi = (rhs.wrapping_shr(64)) as u64;
    let tmp0 = (lhs_lo as u128).wrapping_mul(rhs_lo as u128);
    let tmp1 = (lhs_lo as u128).wrapping_mul(rhs_hi as u128);
    let tmp2 = (lhs_hi as u128).wrapping_mul(rhs_lo as u128);
    let tmp3 = (lhs_hi as u128).wrapping_mul(rhs_hi as u128);
    // tmp1 and tmp2 straddle the boundary. We have to handle three carries
    let (sum0, carry0) = tmp0.overflowing_add(tmp1.wrapping_shl(64));
    let (sum0, carry1) = sum0.overflowing_add(tmp2.wrapping_shl(64));
    let (sum0, carry2) = sum0.overflowing_add(add);
    let sum1 = tmp3
        .wrapping_add(tmp1.wrapping_shr(64))
        .wrapping_add(tmp2.wrapping_shr(64))
        .wrapping_add(carry0 as u128)
        .wrapping_add(carry1 as u128)
        .wrapping_add(carry2 as u128);
    (sum0, sum1)
}

/// Computes (x * y) + z. This cannot overflow, because it returns the value
/// widened into a tuple, where the first element is the least significant part
/// of the integer and the second is the most significant.
#[inline]
pub const fn widen_mul_add(x: Digit, y: Digit, z: Digit) -> (Digit, Digit) {
    widen_mul_add_internal!(
        x, y, z;
        128 => {
            // Hopefully Rust has a built in `widening_mul` or LLVM recognizes this is one
            // big widening multiplication by the time things like 128 bit RISCV are a
            // thing.
            let tmp = widening_mul_add_u128(x as u128, y as u128, z as u128);
            (tmp.0 as Digit, tmp.1 as Digit)
        }
        8, u16;
        16, u32;
        32, u64;
        64, u128;
    )
}

macro_rules! dd_division_internal {
    ($duo:ident, $div:ident; $($bits:expr, $uD:ident);*;) => {
        match BITS {
            $(
                $bits => {
                    let duo = $duo.0 as $uD | (($duo.1 as $uD) << $bits);
                    let div = $div.0 as $uD | (($div.1 as $uD) << $bits);
                    let tmp0 = duo.wrapping_div(div);
                    let tmp1 = duo.wrapping_rem(div);
                    (
                        (
                            tmp0 as Digit,
                            (tmp0 >> $bits) as Digit,
                        ),
                        (
                            tmp1 as Digit,
                            (tmp1 >> $bits) as Digit,
                        )
                    )
                }
            )*
            _ => panic!("Unsupported digit size"),
        }
    };
}

/// Divides `duo` by `div` and returns the quotient and remainder.
///
/// # Panics
///
/// If `div == 0`, this function will panic.
#[inline]
pub const fn dd_division(
    duo: (Digit, Digit),
    div: (Digit, Digit),
) -> ((Digit, Digit), (Digit, Digit)) {
    dd_division_internal!(
        duo, div;
        8, u16;
        16, u32;
        32, u64;
        64, u128;
        // TODO fix this for 128 bits
    )
}

/// Location for an item in the source code
#[derive(Debug, Clone, Copy)]
pub struct Location {
    pub file: &'static str,
    pub line: u32,
    pub col: u32,
}
