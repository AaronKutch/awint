//! This crate contains common utilities for crates within the `awint` system.
//! Some of these are highly unsafe macros that were only placed here because
//! `#[macro_export]` unconditionally causes macros to be publicly accessible.
//! To prevent them from being accessible from intended user-facing crates, this
//! `_internals` crate was made. The safety requirements of these macros may
//! change over time, so this crate should never be used outside of this system.

#![feature(const_panic)]
#![no_std]
// not const and tends to be longer
#![allow(clippy::manual_range_contains)]

mod macros;
mod serde_common;

use core::num::NonZeroUsize;

pub use serde_common::*;

/// Maximum bitwidth of an inline `Awi`
pub const BITS: usize = usize::BITS as usize;

/// Maximum value of an inline `Awi`
pub const MAX: usize = usize::MAX;

/// Utility free function for converting a `usize` to a `NonZeroUsize`. This is
/// mainly intended for usage with literals, and shouldn't be used for fallible
/// conversions.
///
/// # Panics
///
/// If `bw == 0`, this function will panic.
#[inline]
#[track_caller]
pub const fn bw(bw: usize) -> NonZeroUsize {
    match NonZeroUsize::new(bw) {
        None => {
            panic!("Tried to construct an invalid bitwidth of 0 using the `awint::bw` function")
        }
        Some(bw) => bw,
    }
}

/// Returns the number of extra bits given `bw`
#[inline]
pub const fn extra_u(bw: usize) -> usize {
    bw & (BITS - 1)
}

/// Returns the number of _whole_ digits (not including a digit with unused
/// bits) given `bw`
#[inline]
pub const fn digits_u(bw: usize) -> usize {
    bw.wrapping_shr(BITS.trailing_zeros())
}

/// Returns the number of extra bits given `bw`
#[inline]
pub const fn extra(bw: NonZeroUsize) -> usize {
    extra_u(bw.get())
}

/// Returns the number of _whole_ digits (not including a digit with unused
/// bits) given `bw`
#[inline]
pub const fn digits(bw: NonZeroUsize) -> usize {
    digits_u(bw.get())
}

/// Returns the number of `usize` digits needed to represent `bw`, including any
/// digit with unused bits
#[inline]
pub const fn regular_digits(bw: NonZeroUsize) -> usize {
    digits(bw).wrapping_add((extra(bw) != 0) as usize)
}

/// Returns `regular_digits + 1` to account for the bitwidth digit
#[inline]
pub fn raw_digits(bw: usize) -> usize {
    digits_u(bw)
        .wrapping_add((extra_u(bw) != 0) as usize)
        .wrapping_add(1)
}

/// Checks that the `BW` and `LEN` values are valid for an `InlAwi`.
///
/// # Panics
///
/// If `BW == 0`, `LEN < 2`, or the bitwidth is outside the range
/// `(((LEN - 2)*BITS) + 1)..=((LEN - 1)*BITS)`
pub const fn assert_inlawi_invariants<const BW: usize, const LEN: usize>() {
    if BW == 0 {
        panic!("Tried to create an `InlAwi<BW, LEN>` with `BW == 0`")
    }
    if LEN < 2 {
        panic!("Tried to create an `InlAwi<BW, LEN>` with `LEN < 2`")
    }
    if BW <= ((LEN - 2) * BITS) {
        panic!("Tried to create an `InlAwi<BW, LEN>` with `BW <= BITS*(LEN - 2)`")
    }
    if BW > ((LEN - 1) * BITS) {
        panic!("Tried to create an `InlAwi<BW, LEN>` with `BW > BITS*(LEN - 1)`")
    }
}

/// Checks that a raw slice for `InlAwi` construction is correct. Assumes that
/// `assert_inlawi_invariants` has already been run to check the correctness of
/// the `BW` and `LEN` values.
///
/// # Panics
///
/// If `raw.len() != LEN`, the bitwidth digit is zero, or the bitwidth is
/// outside the range `(((LEN - 2)*BITS) + 1)..=((LEN - 1)*BITS)`
pub const fn assert_inlawi_invariants_slice<const BW: usize, const LEN: usize>(raw: &[usize]) {
    if raw.len() != LEN {
        panic!("`length of raw slice does not equal LEN")
    }
    let bw = raw[raw.len() - 1];
    if bw != BW {
        panic!("bitwidth digit does not equal BW")
    }
}

/// Alternate check for `InlAwi` invariants
///
/// # Panics
///
/// If the bitwidth is outside the range `(((LEN - 2)*BITS) + 1)..=((LEN -
/// 1)*BITS)`
pub const fn assert_inlawi_invariants_2<const BW: usize, const LEN: usize>() {
    if BW == 0 {
        panic!("Tried to create an InlAwi with zero bitwidth")
    }
}

/// Computes x + y + z and returns the widened result as a tuple.
#[inline]
pub const fn widen_add(x: usize, y: usize, z: usize) -> (usize, usize) {
    // TODO make sure this is adc on appropriate platforms and also works well on
    // RISCV
    let (sum, carry0) = x.overflowing_add(y);
    let (sum, carry1) = sum.overflowing_add(z);
    (sum, (carry0 as usize) + (carry1 as usize))
}

macro_rules! widen_mul_add_internal {
    ($x:ident, $y:ident, $z:ident; 128 => $other:block $($bits:expr, $uD:ident);*;) => {
        match BITS {
            $(
                $bits => {
                    let tmp = ($x as $uD).wrapping_mul($y as $uD).wrapping_add($z as $uD);
                    (tmp as usize, tmp.wrapping_shr($bits) as usize)
                }
            )*
            128 => $other
            _ => panic!("Unsupported pointer size"),
        }
    };
}

/// Computes (x * y) + z. This cannot overflow, because it returns the value
/// widened into a tuple, where the first element is the least significant part
/// of the integer and the second is the most significant.
#[inline]
pub const fn widen_mul_add(x: usize, y: usize, z: usize) -> (usize, usize) {
    widen_mul_add_internal!(
        x, y, z;
        128 => {
            // Hopefully Rust has a built in `widening_mul` or LLVM recognizes this is one
            // big widening multiplication by the time things like 128 bit RISCV are a
            // thing. TODO verify this
            let lhs_lo = x as u64;
            let rhs_lo = y as u64;
            let lhs_hi = (x.wrapping_shr(64)) as u64;
            let rhs_hi = (y.wrapping_shr(64)) as u64;
            let tmp0 = (lhs_lo as u128).wrapping_mul(rhs_lo as u128);
            let tmp1 = (lhs_lo as u128).wrapping_mul(rhs_hi as u128);
            let tmp2 = (lhs_hi as u128).wrapping_mul(rhs_lo as u128);
            let tmp3 = (lhs_hi as u128).wrapping_mul(rhs_hi as u128);
            // tmp1 and tmp2 straddle the boundary. We have to handle three carries
            let (mul0, carry0) = tmp0.overflowing_add(tmp1.wrapping_shl(64));
            let (mul0, carry1) = mul0.overflowing_add(tmp2.wrapping_shl(64));
            let (mul0, carry2) = mul0.overflowing_add(z as u128);
            let mul1 = tmp3
                .wrapping_add(tmp1.wrapping_shr(64))
                .wrapping_add(tmp2.wrapping_shr(64))
                .wrapping_add(carry0 as u128)
                .wrapping_add(carry1 as u128)
                .wrapping_add(carry2 as u128);
            (mul0 as usize, mul1 as usize)
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
                            tmp0 as usize,
                            (tmp0 >> $bits) as usize,
                        ),
                        (
                            tmp1 as usize,
                            (tmp1 >> $bits) as usize,
                        )
                    )
                }
            )*
            _ => panic!("Unsupported pointer size"),
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
    duo: (usize, usize),
    div: (usize, usize),
) -> ((usize, usize), (usize, usize)) {
    dd_division_internal!(
        duo, div;
        8, u16;
        16, u32;
        32, u64;
        64, u128;
        // TODO fix this for 128 bits
    )
}
