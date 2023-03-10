use crate::{Digit, BITS};

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
            _ => panic!("Unsupported digit size"),
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

type U256 = (u128, u128);

/// Computes the quotient and remainder of `duo` divided by `div` and returns
/// them as a tuple.
pub const fn dd_division_u256(duo: U256, div: U256) -> (U256, U256) {
    // uses the trifecta algorithm from https://crates.io/crates/triple_arena

    const fn lz_u256(x: U256) -> usize {
        let res = x.1.leading_zeros() as usize;
        if res == 128 {
            128 + (x.0.leading_zeros() as usize)
        } else {
            res
        }
    }

    const fn is_zero_u256(x: U256) -> bool {
        (x.0 == 0) && (x.1 == 0)
    }

    const fn add_u256(lhs: U256, rhs: U256) -> U256 {
        let (res0, o) = lhs.0.overflowing_add(rhs.0);
        let res1 = lhs.1.wrapping_add(rhs.1).wrapping_add(o as u128);
        (res0, res1)
    }

    const fn sub_u256(lhs: U256, rhs: U256) -> U256 {
        let (res0, o) = lhs.0.overflowing_sub(rhs.0);
        let res1 = lhs.1.wrapping_sub(rhs.1).wrapping_sub(o as u128);
        (res0, res1)
    }

    const fn ge_u256(lhs: U256, rhs: U256) -> bool {
        if lhs.1 > rhs.1 {
            true
        } else if lhs.1 == rhs.1 {
            lhs.0 >= rhs.0
        } else {
            false
        }
    }

    const fn lt_u256(lhs: U256, rhs: U256) -> bool {
        if lhs.1 < rhs.1 {
            true
        } else if lhs.1 == rhs.1 {
            lhs.0 < rhs.0
        } else {
            false
        }
    }

    const fn or_u256(lhs: U256, rhs: U256) -> U256 {
        (lhs.0 | rhs.0, lhs.1 | rhs.1)
    }

    const fn shl_u256(x: U256, s: usize) -> U256 {
        if s == 0 {
            x
        } else if s < 128 {
            (x.0 << s, (x.0 >> (128 - s)) | (x.1 << s))
        } else {
            (0, x.1 << (s - 128))
        }
    }

    const fn shr_u256(x: U256, s: usize) -> U256 {
        if s == 0 {
            x
        } else if s < 128 {
            ((x.0 >> s) | (x.1 << (128 - s)), x.1 >> s)
        } else {
            (x.1 >> (s - 128), 0)
        }
    }

    const fn half_division(lhs: u128, rhs: u128) -> (u128, u128) {
        (lhs / rhs, lhs % rhs)
    }

    const fn u256_mul_u128(lhs: U256, rhs: u128) -> U256 {
        let (lo, carry) = widening_mul_add_u128(lhs.0, rhs, 0);
        let (hi, _) = widening_mul_add_u128(lhs.1, rhs, carry);
        (lo, hi)
    }

    let one_u256: U256 = (1, 0);
    let zero_u256: U256 = (0, 0);

    // the number of bits in a $uX
    let n = 128;

    if is_zero_u256(div) {
        unreachable!();
    }

    let div_lz = lz_u256(div);
    let mut duo_lz = lz_u256(duo);

    // quotient is 0 or 1 branch
    if div_lz <= duo_lz {
        if ge_u256(duo, div) {
            return (one_u256, sub_u256(duo, div))
        } else {
            return (zero_u256, duo)
        }
    }

    // smaller division branch
    if duo_lz >= n {
        let (quo, rem) = half_division(duo.0, div.0);
        return ((quo, 0), (rem, 0))
    }

    // short division branch
    if div_lz >= (128 + 64) {
        let duo_hi = duo.1;
        let div_0 = div.0 as u64 as u128;
        let (quo_hi, rem_3) = half_division(duo_hi, div_0);

        let duo_mid = ((duo.0 >> 64) as u64 as u128) | (rem_3 << 64);
        let (quo_1, rem_2) = half_division(duo_mid, div_0);

        let duo_lo = (duo.0 as u64 as u128) | (rem_2 << 64);
        let (quo_0, rem_1) = half_division(duo_lo, div_0);

        return (
            or_u256(or_u256((quo_0, 0), (quo_1 << 64, quo_1 >> 64)), (0, quo_hi)),
            (rem_1, 0),
        )
    }

    let lz_diff = div_lz - duo_lz;

    if lz_diff < 64 {
        // Two possibility division algorithm

        let shift = n - duo_lz;
        let duo_sig_n = shr_u256(duo, shift).0;
        let div_sig_n = shr_u256(div, shift).0;
        let quo = half_division(duo_sig_n, div_sig_n).0;

        let div_lo = div.0;
        let div_hi = div.1;
        let (tmp_lo, carry) = widening_mul_add_u128(quo, div_lo, 0);
        let (tmp_hi, overflow) = widening_mul_add_u128(quo, div_hi, carry);
        let tmp = (tmp_lo, tmp_hi);
        if (overflow != 0) || lt_u256(duo, tmp) {
            return ((quo - 1, 0), sub_u256(add_u256(duo, div), tmp))
        } else {
            return ((quo, 0), sub_u256(duo, tmp))
        }
    }

    // Undersubtracting long division algorithm.

    let mut duo = duo;
    let mut quo = zero_u256;

    let div_extra = (128 + 64) - div_lz;

    let div_sig_n_h = shr_u256(div, div_extra).0 as u64;

    let div_sig_n_h_add1 = (div_sig_n_h as u128) + 1;

    loop {
        let duo_extra = n - duo_lz;

        let duo_sig_n = shr_u256(duo, duo_extra).0;

        if div_extra <= duo_extra {
            // Undersubtracting long division step
            let quo_part_lo = half_division(duo_sig_n, div_sig_n_h_add1).0;
            let extra_shl = duo_extra - div_extra;

            quo = add_u256(quo, shl_u256((quo_part_lo, 0), extra_shl));

            duo = sub_u256(duo, shl_u256(u256_mul_u128(div, quo_part_lo), extra_shl));
        } else {
            // Two possibility algorithm
            let shift = n - duo_lz;
            let duo_sig_n = shr_u256(duo, shift).0;
            let div_sig_n = shr_u256(div, shift).0;
            let quo_part = half_division(duo_sig_n, div_sig_n).0;
            let div_lo = div.0;
            let div_hi = div.1;

            let (tmp_lo, carry) = widening_mul_add_u128(quo_part, div_lo, 0);
            // The undersubtracting long division algorithm has already run once, so
            // overflow beyond `$uD` bits is not possible here
            let (tmp_hi, _) = widening_mul_add_u128(quo_part, div_hi, carry);
            let tmp = (tmp_lo, tmp_hi);

            if lt_u256(duo, tmp) {
                return (
                    add_u256(quo, (quo_part - 1, 0)),
                    sub_u256(add_u256(duo, div), tmp),
                )
            } else {
                return (add_u256(quo, (quo_part, 0)), sub_u256(duo, tmp))
            }
        }

        duo_lz = lz_u256(duo);

        if div_lz <= duo_lz {
            // quotient can have 0 or 1 added to it
            if ge_u256(duo, div) {
                return (add_u256(quo, one_u256), sub_u256(duo, div))
            } else {
                return (quo, duo)
            }
        }

        // This can only happen if `div_sd < n` (because of previous "quo = 0 or 1"
        // branches), but it is not worth it to unroll further.
        if n <= duo_lz {
            // simple division and addition
            let tmp = half_division(duo.0, div.0);
            return (add_u256(quo, (tmp.0, 0)), (tmp.1, 0))
        }
    }
}

macro_rules! dd_division_internal {
    ($duo:ident, $div:ident; 128 => $other:block $($bits:expr, $uD:ident);*;) => {
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
            128 => $other
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
        128 => {
            let (quo, rem) = dd_division_u256(
                (duo.0 as u128, duo.1 as u128),
                (div.0 as u128, div.1 as u128)
            );
            ((quo.0 as Digit, quo.1 as Digit), (rem.0 as Digit, rem.1 as Digit))
        }
        8, u16;
        16, u32;
        32, u64;
        64, u128;
    )
}
