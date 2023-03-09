use core::{ops::Range, ptr};

use awint_internals::*;
use const_fn::const_fn;

use crate::Bits;

/// # Bitwise
impl Bits {
    /// Zero-assigns. Same as the Unsigned-minimum-value. All bits are set to 0.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn zero_(&mut self) {
        unsafe { self.digit_set(false, 0..self.len(), false) }
    }

    /// Unsigned-maximum-value-assigns. All bits are set to 1.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn umax_(&mut self) {
        unsafe { self.digit_set(true, 0..self.len(), true) }
    }

    /// Signed-maximum-value-assigns. All bits are set to 1, except for the most
    /// significant bit.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn imax_(&mut self) {
        unsafe { self.digit_set(true, 0..self.len(), false) }
        *self.last_mut() = (MAX >> 1) >> self.unused();
    }

    /// Signed-minimum-value-assigns. Only the most significant bit is set.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn imin_(&mut self) {
        unsafe { self.digit_set(false, 0..self.len(), false) }
        *self.last_mut() = (IDigit::MIN as Digit) >> self.unused();
    }

    /// Unsigned-one-assigns. Only the least significant bit is set. The
    /// unsigned distinction is important, because a positive one value does
    /// not exist for signed integers with a bitwidth of 1.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn uone_(&mut self) {
        unsafe { self.digit_set(false, 0..self.len(), false) }
        *self.first_mut() = 1;
    }

    /// Not-assigns `self`
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn not_(&mut self) {
        unsafe_for_each_mut!(self, x, { *x = !*x }, true);
    }

    /// Copy-assigns the bits of `rhs` to `self`
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn copy_(&mut self, rhs: &Self) -> Option<()> {
        if self.bw() != rhs.bw() {
            return None
        }
        unsafe {
            ptr::copy_nonoverlapping(rhs.as_ptr(), self.as_mut_ptr(), self.len());
        }
        Some(())
    }

    /// Or-assigns `rhs` to `self`
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn or_(&mut self, rhs: &Self) -> Option<()> {
        unsafe_binop_for_each_mut!(self, rhs, x, y, { *x |= y }, false)
    }

    /// And-assigns `rhs` to `self`
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn and_(&mut self, rhs: &Self) -> Option<()> {
        unsafe_binop_for_each_mut!(self, rhs, x, y, { *x &= y }, false)
    }

    /// Xor-assigns `rhs` to `self`
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn xor_(&mut self, rhs: &Self) -> Option<()> {
        unsafe_binop_for_each_mut!(self, rhs, x, y, { *x ^= y }, false)
    }

    /// And-assigns a range of ones to `self`. Useful for masking. An empty or
    /// reversed range zeroes `self`. `None` is returned if `range.start >
    /// self.bw()` or `range.end > self.bw()`.
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn range_and_(&mut self, range: Range<usize>) -> Option<()> {
        if range.start > self.bw() || range.end > self.bw() {
            return None
        }
        // Originally, I considered returning `None` when `range.start == self.bw()` to
        // make things more strict, but I quickly found a case where this made things
        // awkard for the [Bits::field] test in `multi_bw.rs`. If the width of the field
        // being copied was equal to the bitwidth, a `range_and_` would have an
        // input range of `self.bw()..self.bw()`. This zeros `self` as intended because
        // of the natural `range.start >= range.end` check.
        if range.start >= range.end {
            self.zero_();
            return Some(())
        }
        let start = digits_u(range.start);
        let end = digits_u(range.end);
        let start_bits = extra_u(range.start);
        let end_bits = extra_u(range.end);
        // Safety: the early `None` return above prevents any out of bounds indexing.
        unsafe {
            // zero all digits up to the start of the range
            self.digit_set(false, 0..start, false);
            match (end_bits == 0, start == end) {
                (false, false) => {
                    *self.get_unchecked_mut(start) &= MAX << start_bits;
                    *self.get_unchecked_mut(end) &= MAX >> (BITS - end_bits);
                }
                (false, true) => {
                    // The range is entirely contained in one digit
                    *self.get_unchecked_mut(start) &=
                        (MAX << start_bits) & (MAX >> (BITS - end_bits));
                }
                (true, _) => {
                    // Avoid overshift from `(BITS - end_bits)`
                    *self.get_unchecked_mut(start) &= MAX << start_bits;
                    // zero the end
                    if end < self.len() {
                        *self.get_unchecked_mut(end) = 0;
                    }
                }
            }
            // zero the rest of the digits
            if (end + 1) < self.len() {
                self.digit_set(false, (end + 1)..self.len(), false);
            }
        }
        Some(())
    }

    /// Or-assigns `rhs` to `self` at a position `shl`. Set bits of `rhs` that
    /// are shifted beyond the bitwidth of `self` are truncated.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn digit_or_(&mut self, rhs: Digit, shl: usize) {
        if shl >= self.bw() {
            return
        }
        // Safety: `digits < self.len()` because of the above check. The `digits + 1`
        // index is checked.
        let bits = extra_u(shl);
        let digits = digits_u(shl);
        unsafe {
            if bits == 0 {
                *self.get_unchecked_mut(digits) |= rhs;
            } else {
                *self.get_unchecked_mut(digits) |= rhs << bits;
                if (digits + 1) < self.len() {
                    *self.get_unchecked_mut(digits + 1) |= rhs >> (BITS - bits);
                }
            }
        }
    }
}
