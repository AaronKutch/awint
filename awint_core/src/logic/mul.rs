use awint_internals::*;
use const_fn::const_fn;

use crate::Bits;

// TODO optimize for high leading zero and trailing zero cases

/// # Multiplication
impl Bits {
    /// Assigns `cin + (self * rhs)` to `self` and returns the overflow
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn digit_cin_mul_(&mut self, cin: Digit, rhs: Digit) -> Digit {
        let mut carry = cin;
        unsafe_for_each_mut!(
            self,
            x,
            {
                let tmp = widen_mul_add(*x, rhs, carry);
                *x = tmp.0;
                carry = tmp.1;
            },
            false
        );
        let oflow = if self.extra() == 0 {
            carry
        } else {
            (self.last() >> self.extra()) | (carry << (BITS - self.extra()))
        };
        self.clear_unused_bits();
        oflow
    }

    /// Add-assigns `lhs * rhs` to `self` and returns if overflow happened
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn digit_mul_add_(&mut self, lhs: &Self, rhs: Digit) -> Option<bool> {
        let mut mul_carry = 0;
        let mut add_carry = 0;
        unsafe_binop_for_each_mut!(
            self,
            lhs,
            self_x,
            lhs_x,
            {
                let tmp0 = widen_mul_add(lhs_x, rhs, mul_carry);
                mul_carry = tmp0.1;
                let tmp1 = widen_add(*self_x, tmp0.0, add_carry);
                add_carry = tmp1.1;
                *self_x = tmp1.0;
            },
            true
        );
        Some((mul_carry != 0) || (add_carry != 0))
    }

    /// Multiplies `lhs` by `rhs` and add-assigns the product to `self`. Three
    /// operands eliminates the need for an allocating temporary.
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn mul_add_(&mut self, lhs: &Self, rhs: &Self) -> Option<()> {
        if self.bw() != lhs.bw() || self.bw() != rhs.bw() {
            return None
        }
        unsafe {
            const_for!(lhs_i in {0..self.total_digits()} {
                // carry from the short multiplication
                let mut carry0 = 0;
                let mut carry1 = 0;
                const_for!(rhs_i in {0..(self.total_digits() - lhs_i)} {
                    let tmp0 =
                        widen_mul_add(lhs.get_unchecked(lhs_i), rhs.get_unchecked(rhs_i), carry0);
                    carry0 = tmp0.1;
                    let tmp1 = widen_add(self.get_unchecked(lhs_i + rhs_i), tmp0.0, carry1);
                    carry1 = tmp1.1;
                    *self.get_unchecked_mut(lhs_i + rhs_i) = tmp1.0;
                });
            });
        }
        self.clear_unused_bits();
        Some(())
    }

    /// Multiply-assigns `self` by `rhs`. `pad` is a scratchpad that will be
    /// mutated arbitrarily.
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn mul_(&mut self, rhs: &Self, pad: &mut Self) -> Option<()> {
        if self.bw() != rhs.bw() || self.bw() != pad.bw() {
            return None
        }
        pad.zero_();
        unsafe {
            const_for!(self_i in {0..self.total_digits()} {
                // carry from the short multiplication
                let mut carry0 = 0;
                let mut carry1 = 0;
                const_for!(rhs_i in {0..(self.total_digits() - self_i)} {
                    let tmp0 =
                        widen_mul_add(self.get_unchecked(self_i), rhs.get_unchecked(rhs_i), carry0);
                    carry0 = tmp0.1;
                    let tmp1 = widen_add(pad.get_unchecked(self_i + rhs_i), tmp0.0, carry1);
                    carry1 = tmp1.1;
                    *pad.get_unchecked_mut(self_i + rhs_i) = tmp1.0;
                });
            });
        }
        pad.clear_unused_bits();
        self.copy_(pad).unwrap();
        Some(())
    }

    /// Arbitrarily-unsigned-multiplies `lhs` by `rhs` and add-assigns the
    /// product to `self`. This function is equivalent to:
    /// ```
    /// use awint::awi::*;
    ///
    /// fn arb_umul_(add: &mut Bits, lhs: &Bits, rhs: &Bits) {
    ///     let mut resized_lhs = Awi::zero(add.nzbw());
    ///     // Note that this function is specified as unsigned,
    ///     // because we use `zero_resize_`
    ///     resized_lhs.zero_resize_(lhs);
    ///     let mut resized_rhs = Awi::zero(add.nzbw());
    ///     resized_rhs.zero_resize_(rhs);
    ///     add.mul_add_(&resized_lhs, &resized_rhs).unwrap();
    /// }
    /// ```
    /// except that it avoids allocation and is more efficient overall
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn arb_umul_add_(&mut self, lhs: &Self, rhs: &Self) {
        // first, we swap references so that `x0.bw() <= x1.bw()`
        let (x0, x1) = if lhs.bw() <= rhs.bw() {
            (lhs, rhs)
        } else {
            (rhs, lhs)
        };
        let x0_upper_bound = if self.total_digits() < x0.total_digits() {
            self.total_digits()
        } else {
            x0.total_digits()
        };
        // Safety: all the `get_unchecked_` are in bounds, since `x0_i < x0_upper_bound
        // < x0.total_digits()` and there are independent checks every loop for the
        // `x1_i` and `self_i` cases.
        unsafe {
            const_for!(x0_i in {0..x0_upper_bound} {
                // carry from the short multiplication
                let mut carry0 = 0;
                let mut carry1 = 0;
                let mut x1_i = 0;
                let mut self_i = x0_i;
                loop {
                    if x1_i >= x1.total_digits() || self_i >= self.total_digits() {
                        break
                    }
                    let tmp0 =
                        widen_mul_add(x0.get_unchecked(x0_i), x1.get_unchecked(x1_i), carry0);
                    carry0 = tmp0.1;
                    let tmp1 = widen_add(self.get_unchecked(self_i), tmp0.0, carry1);
                    carry1 = tmp1.1;
                    *self.get_unchecked_mut(self_i) = tmp1.0;
                    x1_i += 1;
                    self_i += 1;
                }
                // handle the last short multiplication carry if `self` continues
                if self_i < self.total_digits() {
                    let tmp = widen_add(self.get_unchecked(self_i), carry0, carry1);
                    *self.get_unchecked_mut(self_i) = tmp.0;
                    carry1 = tmp.1;
                    self_i += 1;
                    // handle arbitrarily many addition carries
                    while self_i < self.total_digits() && carry1 != 0 {
                        let tmp = widen_add(self.get_unchecked(self_i), carry1, 0);
                        *self.get_unchecked_mut(self_i) = tmp.0;
                        carry1 = tmp.1;
                        self_i += 1;
                    }
                }
            });
        }
        self.clear_unused_bits();
    }

    /// Arbitrarily-signed-multiplies `lhs` by `rhs` and add-assigns the product
    /// to `self`. Has the same behavior as [Bits::arb_umul_add_] except that is
    /// interprets the arguments as signed. `lhs` and `rhs` are marked
    /// mutable but their values are not changed by this function.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn arb_imul_add_(&mut self, lhs: &mut Self, rhs: &mut Self) {
        let lhs_msb = lhs.msb();
        let rhs_msb = rhs.msb();
        lhs.neg_(lhs_msb);
        rhs.neg_(rhs_msb);
        self.neg_(lhs_msb != rhs_msb);
        self.arb_umul_add_(lhs, rhs);
        lhs.neg_(lhs_msb);
        rhs.neg_(rhs_msb);
        self.neg_(lhs_msb != rhs_msb);
    }
}
