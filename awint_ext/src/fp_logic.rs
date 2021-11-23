use core::{
    borrow::BorrowMut,
    cmp::{max, min},
};

use awint_core::Bits;

use crate::FP;

fn itousize(i: isize) -> Option<usize> {
    usize::try_from(i).ok()
}

/// TODO replace by std lib abs_diff when it stabilizes
fn abs_diff(x: isize, y: isize) -> usize {
    if x < y {
        y.wrapping_sub(x) as usize
    } else {
        x.wrapping_sub(y) as usize
    }
}

/// These functions are associated to avoid name clashes
impl<B: BorrowMut<Bits>> FP<B> {
    /// One-assigns `this`. Returns `None` if a positive one value is not
    /// representable.
    pub fn one_assign(this: &mut Self) -> Option<()> {
        // if fp is negative, one can certainly not be represented
        let fp = itousize(this.fp())?;
        // if `this.signed() && fp == this.bw()`, trying to set the one would set the
        // sign bit
        if fp > this.bw().wrapping_sub(this.signed() as usize) {
            None
        } else {
            this.const_as_mut().zero_assign();
            this.const_as_mut().usize_or_assign(1, fp);
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
    pub fn rel_sb(this: &Self) -> (isize, isize) {
        // cannot overflow because of the invariants
        let lo = this.fp().wrapping_neg();
        // the msb position is one less than the bitwidth
        (lo, this.ibw().wrapping_sub(1).wrapping_add(lo))
    }

    /// The same as [FP::truncate_assign] except it always intreprets arguments
    /// as unsigned
    pub fn utruncate_assign(this: &mut Self, rhs: &Self) {
        this.zero_assign();
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
        let diff = abs_diff(lbb.0, rbb.0);
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
    /// `this.neg_assign(rhs.msb() && this.signed())`.
    pub fn truncate_assign(this: &mut Self, rhs: &mut Self) {
        let mut b = rhs.is_negative();
        // reinterpret as unsigned to avoid imin overflow
        rhs.const_as_mut().neg_assign(b);
        FP::utruncate_assign(this, rhs);
        rhs.const_as_mut().neg_assign(b);
        b &= this.signed();
        this.const_as_mut().neg_assign(b);
    }

    /// The same as [FP::otruncate_assign] except it always intreprets arguments
    /// as unsigned
    fn outruncate_assign(this: &mut Self, rhs: &Self) -> (bool, bool) {
        this.zero_assign();
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
        let diff = abs_diff(lbb.0, rbb.0);
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
    /// [FP::truncate_assign], except that a tuple of booleans is returned. The
    /// first indicates if the least significant numerical bit was truncated,
    /// and the second indicates if the most significant numerical bit was
    /// truncated. Additionally, if `this.is_negative() != rhs.is_negative()`,
    /// the second overflow is set.
    ///
    /// What this means is that if transitive truncations return no overflow,
    /// then numerical value is preserved. If only `FP::otruncate_assign(...).0`
    /// is true, then less significant numerical values were changed and only
    /// some kind of truncation rounding has occured to the numerical value. If
    /// `FP::otruncate_assign(...).1` is true, then the numerical value could be
    /// dramatically changed.
    pub fn otruncate_assign(this: &mut Self, rhs: &mut Self) -> (bool, bool) {
        let mut b = rhs.is_negative();
        // reinterpret as unsigned to avoid imin overflow
        rhs.const_as_mut().neg_assign(b);
        let o = FP::outruncate_assign(this, rhs);
        rhs.const_as_mut().neg_assign(b);
        // imin works correctly
        b &= this.signed();
        this.const_as_mut().neg_assign(b);
        (o.0, o.1 || (this.is_negative() != rhs.is_negative()))
    }
}
