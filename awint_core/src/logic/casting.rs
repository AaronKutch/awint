use core::ptr;

use awint_internals::*;
use const_fn::const_fn;

use crate::Bits;

/// # Casting between `Bits` of arbitrary sizes
impl Bits {
    /// Resize-copy-assigns `rhs` to `self`. If `self.bw() >= rhs.bw()`, the
    /// copied value of `rhs` will be extended with bits set to `extension`. If
    /// `self.bw() < rhs.bw()`, the copied value of `rhs` will be truncated.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn resize_(&mut self, rhs: &Self, extension: bool) {
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
    /// the same as `lhs.resize_(rhs, false)`, but returns `true` if the
    /// unsigned meaning of the integer is changed.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn zero_resize_(&mut self, rhs: &Self) -> bool {
        self.resize_(rhs, false);
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
    /// the same as `lhs.resize_(rhs, rhs.msb())`, but returns `true` if
    /// the signed meaning of the integer is changed.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn sign_resize_(&mut self, rhs: &Self) -> bool {
        self.resize_(rhs, rhs.msb());
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
}
