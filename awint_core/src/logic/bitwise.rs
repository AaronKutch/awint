use core::{ops::Range, ptr};

use awint_internals::*;

use crate::Bits;

/// # Bitwise
impl Bits {
    /// Zero-assigns. Same as the Unsigned-minimum-value. All bits are set to 0.
    pub const fn zero_assign(&mut self) {
        unsafe { self.digit_set(false, 0..self.len(), false) }
    }

    /// Unsigned-maximum-value-assigns. All bits are set to 1.
    pub const fn umax_assign(&mut self) {
        unsafe { self.digit_set(true, 0..self.len(), true) }
    }

    /// Signed-maximum-value-assigns. All bits are set to 1, except for the most
    /// significant bit.
    pub const fn imax_assign(&mut self) {
        unsafe { self.digit_set(true, 0..self.len(), false) }
        *self.last_mut() = (isize::MAX as usize) >> self.unused();
    }

    /// Signed-minimum-value-assigns. Only the most significant bit is set.
    pub const fn imin_assign(&mut self) {
        unsafe { self.digit_set(false, 0..self.len(), false) }
        *self.last_mut() = (isize::MIN as usize) >> self.unused();
    }

    /// Unsigned-one-assigns. Only the least significant bit is set. The
    /// unsigned distinction is important, because a positive one value does
    /// not exist for signed integers with a bitwidth of 1.
    pub const fn uone_assign(&mut self) {
        unsafe { self.digit_set(false, 0..self.len(), false) }
        *self.first_mut() = 1;
    }

    /// Not-assigns `self`
    pub const fn not_assign(&mut self) {
        for_each_mut!(self, x, { *x = !*x }, true);
    }

    /// Copy-assigns the bits of `rhs` to `self`
    pub const fn copy_assign(&mut self, rhs: &Self) -> Option<()> {
        if self.bw() != rhs.bw() {
            return None
        }
        unsafe {
            ptr::copy_nonoverlapping(rhs.as_ptr(), self.as_mut_ptr(), self.len());
        }
        Some(())
    }

    /// Resize-copy-assigns `rhs` to `self`. If `self.bw() >= rhs.bw()`, the
    /// copied value of `rhs` will be extended with bits set to `extension`. If
    /// `self.bw() < rhs.bw()`, the copied value of `rhs` will be truncated.
    pub const fn resize_assign(&mut self, rhs: &Self, extension: bool) {
        // Safety: the exact number of digits needed are copied or set
        unsafe {
            if self.bw() <= rhs.bw() {
                // truncation
                ptr::copy_nonoverlapping(rhs.as_ptr(), self.as_mut_ptr(), self.len());
                self.clear_unused_bits();
            } else {
                ptr::copy_nonoverlapping(rhs.as_ptr(), self.as_mut_ptr(), rhs.len());
                if extension && (rhs.unused() != 0) {
                    *self.get_unchecked_mut(rhs.len() - 1) |= MAX << rhs.extra();
                }
                self.digit_set(extension, rhs.len()..self.len(), extension)
            }
        }
    }

    /// Zero-resize-copy-assigns `rhs` to `self` and returns overflow. This is
    /// the same as `lhs.resize_assign(rhs, false)`, but returns `true` if the
    /// unsigned meaning of the integer is changed.
    pub const fn zero_resize_assign(&mut self, rhs: &Self) -> bool {
        self.resize_assign(rhs, false);
        if self.bw() < rhs.bw() {
            // Safety: `self.len() <= rhs.len()` because of the above check
            unsafe {
                // check if there are set bits that would be truncated
                if (self.extra() != 0) && ((rhs.get_unchecked(self.len() - 1) >> self.extra()) != 0)
                {
                    return true
                }
                const_for!(i in {self.len()..rhs.len()} {
                    if rhs.get_unchecked(i) != 0 {
                        return true
                    }
                });
            }
        }
        false
    }

    /// Sign-resize-copy-assigns `rhs` to `self` and returns overflow. This is
    /// the same as `lhs.resize_assign(rhs, rhs.msb())`, but returns `true` if
    /// the signed meaning of the integer is changed.
    pub const fn sign_resize_assign(&mut self, rhs: &Self) -> bool {
        self.resize_assign(rhs, rhs.msb());
        // this function is far harder to implement than it would first seem
        if self.bw() < rhs.bw() {
            // Safety: `self.len() <= rhs.len()` because of the above check
            unsafe {
                if rhs.msb() {
                    // check if the new most significant bit is unset (which would mean overflow
                    // from negative to positive)
                    if !self.msb() {
                        return true
                    }
                    // check if there are unset bits that would be truncated
                    if self.len() == rhs.len() {
                        // first and only digit
                        if rhs.extra() != 0 {
                            //  rhs extra mask and lhs cutoff mask
                            let expected = (MAX >> (BITS - rhs.extra())) & (MAX << self.extra());
                            if (rhs.last() & expected) != expected {
                                return true
                            }
                        } else {
                            let expected = MAX << self.extra();
                            if (rhs.last() & expected) != expected {
                                return true
                            }
                        }
                        // avoid the other tests if this is the only digit
                        return false
                    }
                    // first digit
                    if self.extra() != 0 {
                        let expected = MAX << self.extra();
                        if (rhs.get_unchecked(self.len() - 1) & expected) != expected {
                            return true
                        }
                    }
                    // middle digits
                    const_for!(i in {self.len()..(rhs.len() - 1)} {
                        if rhs.get_unchecked(i) != MAX {
                            return true
                        }
                    });
                    // last digit
                    if rhs.extra() != 0 {
                        let expected = MAX >> (BITS - rhs.extra());
                        if (rhs.last() & expected) != expected {
                            return true
                        }
                    } else if rhs.last() != MAX {
                        return true
                    }
                } else {
                    // check if the new most significant bit is set (which would mean overflow from
                    // positive to negative)
                    if self.msb() {
                        return true
                    }
                    // check if there are set bits that would be truncated
                    if (self.extra() != 0)
                        && ((rhs.get_unchecked(self.len() - 1) >> self.extra()) != 0)
                    {
                        return true
                    }
                    // Safety: `self.len() <= rhs.len()` because of the above check
                    const_for!(i in {self.len()..rhs.len()} {
                        if rhs.get_unchecked(i) != 0 {
                            return true
                        }
                    });
                }
            }
        }
        false
    }

    /// Or-assigns `rhs` to `self`
    pub const fn or_assign(&mut self, rhs: &Self) -> Option<()> {
        binop_for_each_mut!(self, rhs, x, y, { *x |= y }, false)
    }

    /// And-assigns `rhs` to `self`
    pub const fn and_assign(&mut self, rhs: &Self) -> Option<()> {
        binop_for_each_mut!(self, rhs, x, y, { *x &= y }, false)
    }

    /// Xor-assigns `rhs` to `self`
    pub const fn xor_assign(&mut self, rhs: &Self) -> Option<()> {
        binop_for_each_mut!(self, rhs, x, y, { *x ^= y }, false)
    }

    /// And-assigns a range of ones to `self`. Useful for masking. An empty or
    /// reversed range zeroes `self`. `None` is returned if `range.start >
    /// self.bw()` or `range.end > self.bw()`.
    pub const fn range_and_assign(&mut self, range: Range<usize>) -> Option<()> {
        if range.start > self.bw() || range.end > self.bw() {
            return None
        }
        // Originally, I considered returning `None` when `range.start == self.bw()` to
        // make things more strict, but I quickly found a case where this made things
        // awkard for the [Bits::field] test in `multi_bw.rs`. If the width of the field
        // being copied was equal to the bitwidth, a `range_and_assign` would have an
        // input range of `self.bw()..self.bw()`. This zeros `self` as intended because
        // of the natural `range.start >= range.end` check.
        if range.start >= range.end {
            self.zero_assign();
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
    pub const fn usize_or_assign(&mut self, rhs: usize, shl: usize) {
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
