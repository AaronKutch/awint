use awint_internals::*;

use crate::Bits;

/// # Miscellanious
impl Bits {
    /// Returns the least significant bit
    #[inline]
    pub const fn lsb(&self) -> bool {
        (self.first() & 1) != 0
    }

    /// Returns the most significant bit
    #[inline]
    pub const fn msb(&self) -> bool {
        if self.extra() == 0 {
            (self.last() as isize) < 0
        } else {
            (self.last() & (1 << (self.extra() - 1))) != 0
        }
    }

    /// Returns the number of leading zero bits
    pub const fn lz(&self) -> usize {
        const_for!(i in {0..self.len()}.rev() {
            let x = unsafe{self.get_unchecked(i)};
            if x != 0 {
                return ((self.len() - 1 - i) * BITS) + (x.leading_zeros() as usize) - self.unused();
            }
        });
        (self.len() * BITS) - self.unused()
    }

    /// Returns the number of trailing zero bits
    pub const fn tz(&self) -> usize {
        const_for!(i in {0..self.len()} {
            let x = unsafe{self.get_unchecked(i)};
            if x != 0 {
                return (i * BITS) + (x.trailing_zeros() as usize);
            }
        });
        (self.len() * BITS) - self.unused()
    }

    /// Returns the number of set ones
    pub const fn count_ones(&self) -> usize {
        let mut ones = 0;
        const_for!(i in {0..self.len()} {
            let x = unsafe{self.get_unchecked(i)};
            ones += x.count_ones() as usize;
        });
        ones
    }
}
