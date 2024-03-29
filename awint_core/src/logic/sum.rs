use awint_internals::*;
use const_fn::const_fn;

use crate::Bits;

/// # Summation
impl Bits {
    /// Increment-assigns `self` with a carry-in `cin` and returns the carry-out
    /// bit. If `cin == true` then one is added to `self`, otherwise nothing
    /// happens. `false` is always returned unless `self.is_umax()`.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn inc_(&mut self, cin: bool) -> bool {
        if !cin {
            return false
        }
        unsafe_for_each_mut!(
            self,
            x,
            {0..(self.total_digits() - 1)}
            {
                match x.overflowing_add(1) {
                    (v, false) => {
                        *x = v;
                        return false;
                    }
                    // if the bits were relatively random, this should rarely happen
                    (v, true) => {
                        *x = v;
                    }
                }
            },
            false
        );
        let (last, oflow) = self.last().overflowing_add(1);
        if self.extra() == 0 {
            *self.last_mut() = last;
            oflow
        } else {
            let mask = MAX << self.extra();
            let oflow = (last & mask) != 0;
            *self.last_mut() = last & (!mask);
            oflow
        }
    }

    /// Decrement-assigns `self` with a carry-in `cin` and returns the carry-out
    /// bit. If `cin == false` then one is subtracted from `self`, otherwise
    /// nothing happens. `true` is always returned unless `self.is_zero()`.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn dec_(&mut self, cin: bool) -> bool {
        if cin {
            return true
        }
        unsafe_for_each_mut!(
            self,
            x,
            {0..(self.total_digits() - 1)}
            {
                match x.overflowing_sub(1) {
                    (v, false) => {
                        *x = v;
                        return true
                    }
                    (v, true) => {
                        *x = v;
                    }
                }
            },
            true
        );
        if self.extra() == 0 {
            let (last, oflow) = self.last().overflowing_add(!0);
            *self.last_mut() = last;
            oflow
        } else {
            let mask = MAX << self.extra();
            let last = self.last().wrapping_add(!mask);
            *self.last_mut() = last & (!mask);
            (last & mask) != 0
        }
    }

    /// Negate-assigns `self` if `neg` is true. Note that signed minimum values
    /// will overflow.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn neg_(&mut self, neg: bool) {
        if neg {
            self.not_();
            // note: do not return overflow from the increment because it only happens if
            // `self.is_zero()`, not `self.is_imin()` which will certainly lead
            // to accidents
            self.inc_(true);
        }
    }

    /// Absolute-value-assigns `self`. Note that signed minimum values will
    /// overflow, unless `self` is interpreted as unsigned after a call to this
    /// function.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn abs_(&mut self) {
        self.neg_(self.msb());
    }

    /// Add-assigns by `rhs`
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn add_(&mut self, rhs: &Self) -> Option<()> {
        let mut carry = 0;
        unsafe_binop_for_each_mut!(
            self,
            rhs,
            x,
            y,
            {
                let tmp = widen_add(*x, y, carry);
                *x = tmp.0;
                carry = tmp.1;
            },
            true
        )
    }

    /// Subtract-assigns by `rhs`
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn sub_(&mut self, rhs: &Self) -> Option<()> {
        let mut carry = 1;
        unsafe_binop_for_each_mut!(
            self,
            rhs,
            x,
            y,
            {
                let tmp = widen_add(*x, !y, carry);
                *x = tmp.0;
                carry = tmp.1;
            },
            true
        )
    }

    /// Reverse-subtract-assigns by `rhs`. Sets `self` to `(-self) + rhs`.
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn rsb_(&mut self, rhs: &Self) -> Option<()> {
        let mut carry = 1;
        unsafe_binop_for_each_mut!(
            self,
            rhs,
            x,
            y,
            {
                let tmp = widen_add(!*x, y, carry);
                *x = tmp.0;
                carry = tmp.1;
            },
            true
        )
    }

    /// Negate-add-assigns by `rhs`. Negates conditionally on `neg`.
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn neg_add_(&mut self, neg: bool, rhs: &Self) -> Option<()> {
        if neg {
            self.sub_(rhs)
        } else {
            self.add_(rhs)
        }
    }

    /// A general summation with carry-in `cin` and two inputs `lhs` and `rhs`.
    /// `self` is set to the sum. The unsigned overflow (equivalent to the
    /// carry-out bit) and the signed overflow is returned as a tuple. `None` is
    /// returned if any bitwidths do not match. If subtraction is desired,
    /// one of the operands can be negated.
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn cin_sum_(&mut self, cin: bool, lhs: &Self, rhs: &Self) -> Option<(bool, bool)> {
        if self.bw() != lhs.bw() || self.bw() != rhs.bw() {
            return None
        }
        let mut carry = cin as Digit;
        unsafe {
            const_for!(i in {0..(self.total_digits() - 1)} {
                let tmp = widen_add(lhs.get_unchecked(i), rhs.get_unchecked(i), carry);
                *self.get_unchecked_mut(i) = tmp.0;
                carry = tmp.1;
            });
        }
        let tmp = widen_add(lhs.last(), rhs.last(), carry);
        let extra = self.extra();
        Some(if extra == 0 {
            let lhs_sign = (lhs.last() as IDigit) < 0;
            let rhs_sign = (rhs.last() as IDigit) < 0;
            let output_sign = (tmp.0 as IDigit) < 0;
            *self.last_mut() = tmp.0;
            (
                tmp.1 != 0,
                // Signed overflow only happens if the two input signs are the same and the output
                // sign is different
                (lhs_sign == rhs_sign) && (output_sign != lhs_sign),
            )
        } else {
            let lhs_sign = (lhs.last() & (1 << (extra - 1))) != 0;
            let rhs_sign = (rhs.last() & (1 << (extra - 1))) != 0;
            let output_sign = (tmp.0 & (1 << (extra - 1))) != 0;
            let mask = MAX << extra;
            // handle clearing of unused bits
            *self.last_mut() = tmp.0 & (!mask);
            (
                (tmp.0 & mask) != 0,
                (lhs_sign == rhs_sign) && (output_sign != lhs_sign),
            )
        })
    }
}
