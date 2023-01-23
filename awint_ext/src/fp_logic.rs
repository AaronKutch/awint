use alloc::{string::String, vec::Vec};
use core::{
    borrow::BorrowMut,
    cmp::{max, min},
    num::NonZeroUsize,
};

use awint_core::Bits;

use crate::{
    awint_internals::{bits_upper_bound, SerdeError, SerdeError::*},
    ExtAwi, FP,
};

// TODO there are variations of algorithms that can eliminate all the cases
// where we take `rhs`s by mutable reference

fn itousize(i: isize) -> Option<usize> {
    usize::try_from(i).ok()
}

/// These functions are associated to avoid name clashes.
///
/// Note: Adding new functions to `FP` is a WIP
// TODO
impl<B: BorrowMut<Bits>> FP<B> {
    /// One-assigns `this`. Returns `None` if a positive one value is not
    /// representable.
    #[must_use]
    pub fn one_(this: &mut Self) -> Option<()> {
        // if fp is negative, one can certainly not be represented
        let fp = itousize(this.fp())?;
        // if `this.signed() && fp == this.bw()`, trying to set the one would set the
        // sign bit
        if fp > this.bw().wrapping_sub(this.signed() as usize) {
            None
        } else {
            this.const_as_mut().zero_();
            this.const_as_mut().usize_or_(1, fp);
            Some(())
        }
    }

    /// Relative significant bit positions, determines the bit positions
    /// (inclusive) of the least and most significant bits relative to the
    /// fixed point
    ///
    /// Note: because the msb position is one less than the bitwidth, the
    /// bitwidth is equal to the difference in the bounds _plus one_
    #[inline]
    #[must_use]
    pub fn rel_sb(this: &Self) -> (isize, isize) {
        // cannot overflow because of the invariants
        let lo = this.fp().wrapping_neg();
        // the msb position is one less than the bitwidth
        (lo, this.ibw().wrapping_sub(1).wrapping_add(lo))
    }

    /// The same as [FP::truncate_] except it always intreprets arguments
    /// as unsigned
    pub fn utruncate_<C: BorrowMut<Bits>>(this: &mut Self, rhs: &FP<C>) {
        this.zero_();
        let lbb = FP::rel_sb(this);
        let rbb = FP::rel_sb(rhs);

        // find overlap
        let lo = max(lbb.0, rbb.0);
        let hi = min(lbb.1, rbb.1);
        if hi < lo {
            // does not overlap
            return
        }
        let width = hi.wrapping_sub(lo).wrapping_add(1) as usize;
        let diff = lbb.0.abs_diff(rbb.0);
        // the fielding will start from 0 in one argument and end at `diff` in the other
        let (to, from) = if lbb.0 < rbb.0 { (diff, 0) } else { (0, diff) };
        this.const_as_mut()
            .field(to, rhs.const_as_ref(), from, width)
            .unwrap();
    }

    /// Truncate-assigns `rhs` to `this`. For the unsigned case, logically what
    /// this does is make `this` and `rhs` into concatenations with infinite
    /// zeros on both ends, aligns the fixed points, and copies from `rhs`
    /// to `this`. For the case of `rhs.signed()`, the absolute value of
    /// `rhs` is used for truncation to `this` followed by
    /// `this.neg_(rhs.msb() && this.signed())`.
    pub fn truncate_<C: BorrowMut<Bits>>(this: &mut Self, rhs: &mut FP<C>) {
        let mut b = rhs.is_negative();
        // reinterpret as unsigned to avoid imin overflow
        rhs.const_as_mut().neg_(b);
        FP::utruncate_(this, rhs);
        rhs.const_as_mut().neg_(b);
        b &= this.signed();
        this.const_as_mut().neg_(b);
    }

    /// The same as [FP::otruncate_] except it always intreprets arguments
    /// as unsigned
    #[must_use = "use `utruncate_` if you do not need the overflow booleans"]
    pub fn outruncate_<C: BorrowMut<Bits>>(this: &mut Self, rhs: &FP<C>) -> (bool, bool) {
        this.zero_();
        if rhs.is_zero() {
            return (false, false)
        }
        let lbb = FP::rel_sb(this);
        let rbb = FP::rel_sb(rhs);

        // find overlap
        let lo = max(lbb.0, rbb.0);
        let hi = min(lbb.1, rbb.1);
        if hi < lo {
            // does not overlap
            return (true, true)
        }
        let width = hi.wrapping_sub(lo).wrapping_add(1) as usize;
        let diff = lbb.0.abs_diff(rbb.0);
        let (to, from) = if lbb.0 < rbb.0 { (diff, 0) } else { (0, diff) };
        this.const_as_mut()
            .field(to, rhs.const_as_ref(), from, width)
            .unwrap();
        // when testing if a less significant numerical bit is cut off, we need to be
        // aware that it can be cut off from above even if overlap happens, for
        // example:
        //
        // 1.0
        //  .yyy
        // _____
        //  .000
        //
        // The `1` is the least significant numerical bit, but will get truncated by
        // being above the rel_msb.

        // note overflow cannot happen because of the `rhs.is_zero()` early return and
        // invariants
        let mut lsnb = rhs.const_as_ref().tz() as isize;
        lsnb = lsnb.wrapping_add(rbb.0);
        let mut msnb = rhs
            .bw()
            .wrapping_sub(rhs.const_as_ref().lz())
            .wrapping_sub(1) as isize;
        msnb = msnb.wrapping_add(rbb.0);
        (
            (lsnb < lbb.0) || (lsnb > lbb.1),
            (msnb < lbb.0) || (msnb > lbb.1),
        )
    }

    /// Overflow-truncate-assigns `rhs` to `this`. The same as
    /// [FP::truncate_], except that a tuple of booleans is returned. The
    /// first indicates if the least significant numerical bit was truncated,
    /// and the second indicates if the most significant numerical bit was
    /// truncated. Additionally, if `this.is_negative() != rhs.is_negative()`,
    /// the second overflow is set.
    ///
    /// What this means is that if transitive truncations return no overflow,
    /// then numerical value is preserved. If only `FP::otruncate_(...).0`
    /// is true, then less significant numerical values were changed and only
    /// some kind of truncation rounding has occured to the numerical value. If
    /// `FP::otruncate_(...).1` is true, then the numerical value could be
    /// dramatically changed.
    #[must_use = "use `truncate_` if you do not need the overflow booleans"]
    pub fn otruncate_<C: BorrowMut<Bits>>(this: &mut Self, rhs: &mut FP<C>) -> (bool, bool) {
        let mut b = rhs.is_negative();
        // reinterpret as unsigned to avoid imin overflow
        rhs.const_as_mut().neg_(b);
        let o = FP::outruncate_(this, rhs);
        rhs.const_as_mut().neg_(b);
        // imin works correctly
        b &= this.signed();
        this.const_as_mut().neg_(b);
        (o.0, o.1 || (this.is_negative() != rhs.is_negative()))
    }

    /// Floating-assigns `rhs` to `this`. This modifies the `fp` of `this` to
    /// retain as much significant numerical precision as possible. If
    /// `this.signed()`, the msnb (most significant numerical bit) is moved to
    /// the second msb of `this`. Otherwise, the msnb is moved to the msb of
    /// `this`. If `rhs.is_negative()` and `this` is not signed, the absolute
    /// value of `rhs` is used. If `rhs.is_zero()`, `this` and its `fp` are
    /// zeroed. Returns `None` if the fixed point invariant would be
    /// violated.
    pub fn floating_<C: BorrowMut<Bits>>(this: &mut Self, rhs: &mut FP<C>) -> Option<()> {
        let b = rhs.is_negative();
        rhs.neg_(b);
        let rhs_lz = rhs.lz();
        if rhs_lz == rhs.bw() {
            // efficient zero
            this.zero_();
            // do this since we will also do this in triop situations
            this.set_fp(0).unwrap();
        } else {
            let msnb_add1 = rhs.bw().wrapping_sub(rhs_lz);
            let this_sig_w = this.bw().wrapping_sub(this.signed() as usize);
            let (to, from, width) = if msnb_add1 > this_sig_w {
                (0, msnb_add1.wrapping_sub(this_sig_w), this_sig_w)
            } else {
                (this_sig_w.wrapping_sub(msnb_add1), 0, msnb_add1)
            };
            let rhs_exp = (msnb_add1.wrapping_sub(1) as isize).wrapping_sub(rhs.fp());
            let neg_this = b && this.signed();
            if neg_this && (this.bw() == 1) {
                // corner case: negative powers of two can be represented with one signed bit
                if this.set_fp(rhs_exp.wrapping_neg()).is_none() {
                    rhs.neg_(b);
                    return None
                }
                this.umax_();
            } else {
                if this
                    .set_fp((this_sig_w as isize).wrapping_sub(1).wrapping_sub(rhs_exp))
                    .is_none()
                {
                    rhs.neg_(b);
                    return None
                }
                this.zero_();
                this.field(to, rhs, from, width).unwrap();
                this.neg_(neg_this);
            }
        }
        rhs.neg_(b);
        Some(())
    }

    /// Creates a tuple of `Vec<u8>`s representing the integer and fraction
    /// parts `this` (sign indicators, prefixes, points, and postfixes not
    /// included). This function performs allocation. This is the inverse of
    /// [ExtAwi::from_bytes_general] and extends the abilities of
    /// [ExtAwi::bits_to_vec_radix]. Signedness and fixed point position
    /// information is taken from `this`. `min_integer_chars` specifies the
    /// minimum number of chars in the integer part, inserting leading '0's if
    /// there are not enough chars. `min_fraction_chars` works likewise for the
    /// fraction part, inserting trailing '0's.
    ///
    /// ```
    /// use awint::awi::*;
    /// // note: a user may want to define their own helper functions to do
    /// // this in one step and combine the output into one string using
    /// // the notation they prefer.
    ///
    /// // This creates a fixed point value of -42.1234_i32f16
    /// // (`ExtAwi::from_str` will be able to parse this format in the future
    /// // after more changes to `awint` are made).
    /// let awi = ExtAwi::from_str_general(Some(true), "42", "1234", 0, 10, bw(32), 16).unwrap();
    /// let fp_awi = FP::new(true, awi, 16).unwrap();
    /// assert_eq!(
    ///     // note: in many situations users will want at least 1 zero for
    ///     // both parts so that zero parts result in "0" strings and not "",
    ///     // so `min_..._chars` will be 1. See also
    ///     // `FPType::unique_min_fraction_digits`.
    ///     FP::to_str_general(&fp_awi, 10, false, 1, 1),
    ///     Ok(("42".to_owned(), "1234".to_owned()))
    /// );
    /// ```
    ///
    /// # Errors
    ///
    /// This can only return an error if `radix` is not in the range 2..=36 or
    /// if resource exhaustion occurs.
    pub fn to_vec_general(
        this: &Self,
        radix: u8,
        upper: bool,
        min_integer_chars: usize,
        min_fraction_chars: usize,
    ) -> Result<(Vec<u8>, Vec<u8>), SerdeError> {
        if radix < 2 || radix > 36 {
            return Err(InvalidRadix)
        }
        // I was originally going to include b'-', but it causes insertion performance
        // problems here, and users have to remove it anyway in the usage cases where a
        // prefix is added (we want "-0x123" and not "0x-123")

        let is_zero = this.is_zero();
        let is_negative = this.is_negative();
        let mut unsigned = ExtAwi::zero(this.nzbw());
        unsigned.copy_(this).unwrap();
        // reinterpret as unsigned for `imin`
        unsigned.neg_(is_negative);
        // safe because of invariants
        let tot_lz = unsigned.lz() as isize;

        // the order of these `||` is important to avoid overflow
        let integer_part_zero =
            is_zero || (this.fp() > this.ibw()) || (tot_lz > (this.ibw() - this.fp()));
        let mut integer_part = if integer_part_zero {
            alloc::vec![b'0'; min_integer_chars]
        } else {
            let from = max(this.fp(), 0) as usize;
            // no overflow because of `integer_part_zero` checks
            let bits = this.ibw().wrapping_sub(tot_lz);
            let integer_bits = bits.wrapping_sub(this.fp()) as usize;
            let field_bits = bits.wrapping_sub(from as isize) as usize;
            // if the fixed point mandates more trailing zeroes in the integer part
            let extra_zeros = if this.fp() < 0 {
                this.fp().unsigned_abs()
            } else {
                0
            };
            match NonZeroUsize::new(integer_bits) {
                Some(integer_bits) => {
                    let mut tmp = ExtAwi::zero(integer_bits);
                    tmp.field(extra_zeros, &unsigned, from, field_bits).unwrap();
                    // note: we do not unwrap here in case of resource exhaustion
                    ExtAwi::bits_to_vec_radix(&tmp, false, radix, upper, min_integer_chars)?
                }
                None => alloc::vec![b'0'; min_integer_chars],
            }
        };

        let tot_tz = unsigned.tz() as isize;
        // order is important again
        let fraction_part_zero = is_zero || (this.fp() <= 0) || (tot_tz >= this.fp());
        let mut fraction_part = if fraction_part_zero {
            alloc::vec![b'0'; min_fraction_chars]
        } else {
            let unique_digits = this
                .fp_ty()
                .unique_min_fraction_digits(usize::from(radix))
                .unwrap();
            let calc_digits = max(unique_digits, min_fraction_chars);
            let multiplier_bits = bits_upper_bound(calc_digits, radix)?;
            // avoid needing some calculation by dropping zero bits that have no impact
            let calc_fp = this.fp().wrapping_sub(tot_tz) as usize;
            let field_bits = min(this.fp(), this.ibw()).wrapping_sub(tot_tz) as usize;
            let mut tmp = ExtAwi::zero(
                NonZeroUsize::new(multiplier_bits.checked_add(calc_fp).ok_or(Overflow)?).unwrap(),
            );
            tmp.field_from(&unsigned, tot_tz as usize, field_bits)
                .unwrap();
            for _ in 0..calc_digits {
                tmp.short_cin_mul(0, usize::from(radix));
            }
            let inc = if (tmp.get_digit(calc_fp.checked_sub(1).ok_or(Overflow)?) & 1) == 0 {
                // round down
                false
            } else if tmp.tz().checked_add(1).ok_or(Overflow)? == calc_fp {
                // round to even
                (tmp.get_digit(calc_fp) & 1) == 0
            } else {
                // round up
                true
            };
            tmp.lshr_(calc_fp).unwrap();
            tmp.inc_(inc);
            // note: we do not unwrap here in case of resource exhaustion
            let mut s = ExtAwi::bits_to_vec_radix(&tmp, false, radix, upper, calc_digits)?;
            // trim off zeroes
            while s.len() > min_fraction_chars {
                // s.len() > 0 so this cannot overflow
                if s[s.len().wrapping_sub(1)] == b'0' {
                    let _ = s.pop();
                } else {
                    break
                }
            }
            s
        };
        integer_part.shrink_to_fit();
        fraction_part.shrink_to_fit();
        Ok((integer_part, fraction_part))
    }

    /// Creates a tuple of `String`s representing the integer and fraction
    /// parts of `this`. This does the same thing as `[FP::to_vec_general]`
    /// but with `String`s.
    pub fn to_str_general(
        this: &Self,
        radix: u8,
        upper: bool,
        min_integer_chars: usize,
        min_fraction_chars: usize,
    ) -> Result<(String, String), SerdeError> {
        let (i, f) = FP::to_vec_general(this, radix, upper, min_integer_chars, min_fraction_chars)?;
        Ok((String::from_utf8(i).unwrap(), String::from_utf8(f).unwrap()))
    }
}
