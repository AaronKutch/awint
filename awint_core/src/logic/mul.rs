use awint_internals::*;

use crate::Bits;

// TODO optimize for high leading zero and trailing zero cases

/// # Multiplication
impl Bits {
    /// Assigns `cin + (self * rhs)` to `self` and returns the overflow
    pub const fn short_cin_mul(&mut self, cin: usize, rhs: usize) -> usize {
        let mut carry = cin;
        for_each_mut!(
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
    pub const fn short_mul_add_triop(&mut self, lhs: &Self, rhs: usize) -> Option<bool> {
        let mut mul_carry = 0;
        let mut add_carry = 0;
        binop_for_each_mut!(
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
    pub const fn mul_add_triop(&mut self, lhs: &Self, rhs: &Self) -> Option<()> {
        if self.bw() != lhs.bw() || self.bw() != rhs.bw() {
            return None
        }
        unsafe {
            const_for!(lhs_i in {0..self.len()} {
                // carry from the short multiplication
                let mut carry0 = 0;
                let mut carry1 = 0;
                const_for!(rhs_i in {0..(self.len() - lhs_i)} {
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
}
