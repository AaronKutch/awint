use awint_internals::*;
use const_fn::const_fn;

use crate::Bits;

/// # Division
///
/// These operations are not inplace unlike many other functions in this crate,
/// because extra mutable space is needed in order to avoid allocation.
///
/// Note that signed divisions can overflow when `duo.is_imin()` and
/// `div.is_umax()` (negative one in signed interpretation). The overflow
/// results in `quo.is_imin()` and `rem.is_zero()`.
///
/// Note about terminology: we like short three letter shorthands, but run into
/// a problem where the first three letters of "divide", "dividend", and
/// "divisor" all clash with each other. Additionally, the standard Rust
/// terminology for a function returning a quotient is things such as
/// `i64::wrapping_div`, which should have been named `i64::wrapping_quo`
/// instead. Here, we choose to type out "divide" in full whenever the operation
/// involves both quotients and remainders. We don't use "num" or "den", because
/// it may cause confusion later if an `awint` crate gains rational number
/// capabilities. We use "quo" for quotient and "rem" for remainder. We use
/// "div" for divisor. That still leaves a name clash with dividend, so we
/// choose to use the shorthand "duo". This originates from the fact that for
/// inplace division operations (which this crate does not have for performance
/// purposes and avoiding allocation), the dividend is often subtracted from in
/// the internal algorithms until it becomes the remainder, so that it serves
/// two purposes.
impl Bits {
    /// Unsigned-divides `self` by `div`, sets `self` to the quotient, and
    /// returns the remainder. Returns `None` if `div == 0`.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn short_udivide_inplace_assign(&mut self, div: usize) -> Option<usize> {
        if div == 0 {
            return None
        }
        // Note: we cannot have a carry-in because the quotient could otherwise
        // overflow; `rem` needs to start as 0 and it would cause signature problems
        // anyway.
        let mut rem = 0;
        const_for!(i in {0..self.len()}.rev() {
            // Safety: we checked that `self.bw() == duo.bw()`
            let y = unsafe {self.get_unchecked(i)};
            let tmp = dd_division((y, rem), (div, 0));
            rem = tmp.1.0;
            *unsafe {self.get_unchecked_mut(i)} = tmp.0.0;
        });
        Some(rem)
    }

    // Unsigned-divides `duo` by `div`, sets `self` to the quotient, and
    // returns the remainder. Returns `None` if `self.bw() != duo.bw()` or
    // `div == 0`.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn short_udivide_assign(&mut self, duo: &Self, div: usize) -> Option<usize> {
        if div == 0 || self.bw() != duo.bw() {
            return None
        }
        let mut rem = 0;
        const_for!(i in {0..self.len()}.rev() {
            // Safety: we checked that `self.bw() == duo.bw()`
            let y = unsafe {duo.get_unchecked(i)};
            let tmp = dd_division((y, rem), (div, 0));
            rem = tmp.1.0;
            *unsafe {self.get_unchecked_mut(i)} = tmp.0.0;
        });
        Some(rem)
    }

    /// This function is factored out to keep code size  of the division
    /// functions from exploding
    ///
    /// # Safety
    ///
    /// Assumptions: The bitwidths all match, `div.lz() > duo.lz()`, `div.lz() -
    /// duo.lz() < BITS`, and there are is at least `BITS * 2` bits worth of
    /// significant bits in `duo`.
    #[const_fn(cfg(feature = "const_support"))]
    pub(crate) const unsafe fn two_possibility_algorithm(
        quo: &mut Self,
        rem: &mut Self,
        duo: &Self,
        div: &Self,
    ) {
        debug_assert!(div.lz() > duo.lz());
        debug_assert!((div.lz() - duo.lz()) < BITS);
        debug_assert!((duo.bw() - duo.lz()) >= (BITS * 2));
        let i = duo.bw() - duo.lz() - (BITS * 2);
        let duo_sig_dd = duo.get_double_digit(i);
        let div_sig_dd = div.get_double_digit(i);
        // Because `lz_diff < BITS`, the quotient will fit in one `usize`
        let mut small_quo: usize = dd_division(duo_sig_dd, div_sig_dd).0 .0;
        // using `rem` as a temporary
        rem.copy_assign(div).unwrap();
        let uof = rem.short_cin_mul(0, small_quo);
        rem.rsb_assign(duo).unwrap();
        if (uof != 0) || rem.msb() {
            rem.add_assign(div).unwrap();
            small_quo -= 1;
        }
        quo.usize_assign(small_quo);
    }

    /// Unsigned-divides `duo` by `div` and assigns the quotient to `quo` and
    /// remainder to `rem`. Returns `None` if any bitwidths are not equal or
    /// `div.is_zero()`.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn udivide(quo: &mut Self, rem: &mut Self, duo: &Self, div: &Self) -> Option<()> {
        // prevent any potential problems with the assumptions that many subroutines
        // make
        quo.assert_cleared_unused_bits();
        rem.assert_cleared_unused_bits();
        duo.assert_cleared_unused_bits();
        div.assert_cleared_unused_bits();
        let bw = quo.bw();
        if div.is_zero() || bw != rem.bw() || bw != duo.bw() || bw != div.bw() {
            return None
        }
        // This is a version of the "trifecta" division algorithm adapted for bigints.
        // See https://github.com/AaronKutch/specialized-div-rem for better documentation.

        let len = quo.len();
        let mut duo_lz = duo.lz();
        let div_lz = div.lz();

        // quotient is 0 or 1 branch
        if div_lz <= duo_lz {
            if duo.uge(div).unwrap() {
                quo.uone_assign();
                rem.copy_assign(duo).unwrap();
                rem.sub_assign(div).unwrap();
            } else {
                quo.zero_assign();
                rem.copy_assign(duo).unwrap();
            }
            return Some(())
        }

        // small division branch
        if (bw - duo_lz) <= BITS {
            let tmp_duo = duo.to_usize();
            let tmp_div = div.to_usize();
            quo.usize_assign(tmp_duo.wrapping_div(tmp_div));
            rem.usize_assign(tmp_duo.wrapping_rem(tmp_div));
            return Some(())
        }

        // double digit division branch. This is needed or else some branches below
        // cannot rely on there being at least two digits of significant bits.
        if (bw - duo_lz) <= BITS * 2 {
            unsafe {
                let tmp = dd_division(
                    (duo.first(), duo.get_unchecked(1)),
                    (div.first(), div.get_unchecked(1)),
                );
                // using `usize_assign` to make sure other digits are zeroed
                quo.usize_assign(tmp.0 .0);
                *quo.get_unchecked_mut(1) = tmp.0 .1;
                rem.usize_assign(tmp.1 .0);
                *rem.get_unchecked_mut(1) = tmp.1 .1;
            }
            return Some(())
        }

        // TODO optimize for `trailing_zeros`. This function needs optimization in
        // general such as using subslices more aggressively, need internal functions
        // that handle differing lengths.

        // short division branch
        if bw - div_lz <= BITS {
            let tmp = quo.short_udivide_assign(duo, div.to_usize()).unwrap();
            rem.usize_assign(tmp);
            return Some(())
        }

        // Two possibility division algorithm branch
        let lz_diff = div_lz - duo_lz;
        if lz_diff < BITS {
            // Safety: we checked for bitwidth equality, the quotient is 0 or 1 branch makes
            // sure `div.lz() > duo.lz()`, we just checked that `lz_diff < BITS`, and the
            // double digit division branch makes sure there are at least `BITS*2`
            // significant bits
            unsafe {
                Bits::two_possibility_algorithm(quo, rem, duo, div);
            }
            return Some(())
        }

        // We make a slight deviation from the original trifecta algorithm: instead of
        // finding the quotient partial by dividing the `BITS*2` msbs of `duo` by the
        // `BITS` msbs of `div`, we use `BITS*2 - 1` msbs of `duo` and `BITS` msbs of
        // `div`. This is because the original partial could require `BITS + 1`
        // significant bits (consider (2^(BITS*2) - 1) / (2^(BITS - 1) + 1)). This is
        // possible to handle, but causes more problems than it is worth as seen from an
        // earlier implementation in the `apint` crate.

        let div_extra = bw - div_lz - BITS;
        let div_sig_d = div.get_digit(div_extra);
        let div_sig_d_add1 = widen_add(div_sig_d, 1, 0);
        quo.zero_assign();
        // use `rem` as "duo" from now on
        rem.copy_assign(duo).unwrap();
        loop {
            let duo_extra = bw - duo_lz - (BITS * 2) + 1;
            // using `<` instead of `<=` because of the change to `duo_extra`
            if div_extra < duo_extra {
                // Undersubtracting long division step

                // `get_dd_unchecked` will not work, e.x. bw = 192 and duo_lz = 0, it will
                // attempt to access an imaginary zero bit beyond the bitwidth
                let duo_sig_dd = unsafe {
                    let digits = digits_u(duo_extra);
                    let bits = extra_u(duo_extra);
                    if bits == 0 {
                        (rem.get_unchecked(digits), rem.get_unchecked(digits + 1))
                    } else {
                        let mid = rem.get_unchecked(digits + 1);
                        let last = if digits + 2 == len {
                            0
                        } else {
                            rem.get_unchecked(digits + 2)
                        };
                        (
                            (rem.get_unchecked(digits) >> bits) | (mid << (BITS - bits)),
                            (mid >> bits) | (last << (BITS - bits)),
                        )
                    }
                };
                let quo_part = dd_division(duo_sig_dd, div_sig_d_add1).0 .0;
                let extra_shl = duo_extra - div_extra;
                let shl_bits = extra_u(extra_shl);
                let shl_digits = digits_u(extra_shl);

                // Addition of `quo_part << extra_shl` to the quotient.
                let (carry, next) = unsafe {
                    if shl_bits == 0 {
                        let tmp = widen_add(quo.get_unchecked(shl_digits), quo_part, 0);
                        *quo.get_unchecked_mut(shl_digits) = tmp.0;
                        (tmp.1 != 0, shl_digits + 1)
                    } else {
                        let tmp0 =
                            widen_add(quo.get_unchecked(shl_digits), quo_part << shl_bits, 0);
                        *quo.get_unchecked_mut(shl_digits) = tmp0.0;
                        let tmp1 = widen_add(
                            quo.get_unchecked(shl_digits + 1),
                            quo_part >> (BITS - shl_bits),
                            tmp0.1,
                        );
                        *quo.get_unchecked_mut(shl_digits + 1) = tmp1.0;
                        (tmp1.1 != 0, shl_digits + 2)
                    }
                };
                unsafe {
                    subdigits_mut!(quo, { next..len }, subquo, {
                        subquo.inc_assign(carry);
                    });
                }

                // Subtraction of `(div * quo_part) << extra_shl` from duo. Requires three
                // different carries.

                // carry for bits that wrap across digit boundaries when `<< shl_bits` is
                // applied
                let mut wrap_carry = 0;
                // the multiplication carry
                let mut mul_carry = 0;
                // subtraction carry starting with the two's complement increment
                let mut add_carry = 1;
                unsafe {
                    if shl_bits == 0 {
                        // subdigit shift unneeded
                        const_for!(i in {shl_digits..len} {
                            let tmp1 = widen_mul_add(
                                div.get_unchecked(i - shl_digits),
                                quo_part,
                                mul_carry
                            );
                            mul_carry = tmp1.1;
                            // notice that `tmp1` is inverted for subtraction
                            let tmp2 = widen_add(!tmp1.0, rem.get_unchecked(i), add_carry);
                            add_carry = tmp2.1;
                            *rem.get_unchecked_mut(i) = tmp2.0;
                        });
                    } else {
                        const_for!(i in {shl_digits..len} {
                            let tmp0 = wrap_carry | (div.get_unchecked(i - shl_digits) << shl_bits);
                            wrap_carry = div.get_unchecked(i - shl_digits) >> (BITS - shl_bits);
                            let tmp1 = widen_mul_add(tmp0, quo_part, mul_carry);
                            mul_carry = tmp1.1;
                            let tmp2 = widen_add(!tmp1.0, rem.get_unchecked(i), add_carry);
                            add_carry = tmp2.1;
                            *rem.get_unchecked_mut(i) = tmp2.0;
                        });
                    }
                }
            } else {
                // Two possibility algorithm
                let i = bw - duo_lz - (BITS * 2);
                let duo_sig_dd = rem.get_double_digit(i);
                let div_sig_dd = div.get_double_digit(i);
                // Because `lz_diff < BITS`, the quotient will fit in one `usize`
                let mut small_quo: usize = dd_division(duo_sig_dd, div_sig_dd).0 .0;
                // subtract `div*small_quo` from `rem` inplace
                let mut mul_carry = 0;
                let mut add_carry = 1;
                unsafe {
                    const_for!(i in {0..len} {
                        let tmp0 = widen_mul_add(div.get_unchecked(i), small_quo, mul_carry);
                        mul_carry = tmp0.1;
                        let tmp1 = widen_add(!tmp0.0, rem.get_unchecked(i), add_carry);
                        add_carry = tmp1.1;
                        *rem.get_unchecked_mut(i) = tmp1.0;
                    });
                }
                if rem.msb() {
                    rem.add_assign(div).unwrap();
                    small_quo -= 1;
                }
                // add `quo_add` to `quo`
                let tmp = widen_add(quo.first(), small_quo, 0);
                *quo.first_mut() = tmp.0;
                unsafe {
                    subdigits_mut!(quo, { 1..len }, subquo, {
                        subquo.inc_assign(tmp.1 != 0);
                    });
                }
                return Some(())
            }

            duo_lz = rem.lz();

            if div_lz <= duo_lz {
                // quotient can have 0 or 1 added to it
                if div_lz == duo_lz && div.ule(rem).unwrap() {
                    quo.inc_assign(true);
                    rem.sub_assign(div);
                }
                return Some(())
            }

            // duo fits in two digits. Only possible if `div` fits into two digits, but it
            // is not worth it to unroll further
            if (bw - duo_lz) <= (BITS * 2) {
                unsafe {
                    let tmp = dd_division(
                        (rem.first(), rem.get_unchecked(1)),
                        (div.first(), div.get_unchecked(1)),
                    );
                    // using `usize_assign` to make sure other digits are zeroed
                    let tmp0 = widen_add(quo.first(), tmp.0 .0, 0);

                    *quo.first_mut() = tmp0.0;
                    // tmp.0.1 is zero, just handle the carry now
                    subdigits_mut!(quo, { 1..len }, subquo, {
                        subquo.inc_assign(tmp0.1 != 0);
                    });
                    rem.usize_assign(tmp.1 .0);
                    *rem.get_unchecked_mut(1) = tmp.1 .1;
                }
                return Some(())
            }
        }
    }

    /// Signed-divides `duo` by `div` and assigns the quotient to `quo` and
    /// remainder to `rem`. Returns `None` if any bitwidths are not equal or
    /// `div.is_zero()`. `duo` and `div` are marked mutable but their values are
    /// not changed by this function. They are mutable in order to prevent
    /// internal complications.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn idivide(
        quo: &mut Self,
        rem: &mut Self,
        duo: &mut Self,
        div: &mut Self,
    ) -> Option<()> {
        let bw = quo.bw();
        if div.is_zero() || bw != rem.bw() || bw != duo.bw() || bw != div.bw() {
            return None
        }
        let duo_msb = duo.msb();
        let div_msb = div.msb();
        duo.neg_assign(duo_msb);
        div.neg_assign(div_msb);
        Bits::udivide(quo, rem, duo, div).unwrap();
        duo.neg_assign(duo_msb);
        rem.neg_assign(duo_msb);
        div.neg_assign(div_msb);
        quo.neg_assign(duo_msb != div_msb);
        Some(())
    }
}
