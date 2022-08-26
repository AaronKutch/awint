use core::cmp::*;

use awint_internals::*;
use const_fn::const_fn;

use crate::Bits;

/// If `self` and `other` have unmatching bit widths, `false` will be returned.
impl PartialEq for Bits {
    fn eq(&self, rhs: &Self) -> bool {
        self.bw() == rhs.bw() && self.const_eq(rhs).unwrap()
    }
}

/// If `self` and `other` have unmatching bit widths, `false` will be returned.
impl Eq for Bits {}

/// # Comparison
impl Bits {
    /// If `self` is zero
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn is_zero(&self) -> bool {
        for_each!(self, x, {
            if x != 0 {
                return false
            }
        });
        true
    }

    /// If `self` is unsigned-maximum
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn is_umax(&self) -> bool {
        for_each!(self, x, {0..(self.len() - 1)} {
            if x != MAX {
                return false
            }
        });
        if self.extra() == 0 {
            self.last() == MAX
        } else {
            self.last() == (MAX >> self.unused())
        }
    }

    /// If `self` is signed-maximum
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn is_imax(&self) -> bool {
        for_each!(self, x, {0..(self.len() - 1)} {
            if x != MAX {
                return false
            }
        });
        if self.extra() == 0 {
            self.last() == MAX >> 1
        } else {
            self.last() == !(MAX << (self.extra() - 1))
        }
    }

    /// If `self` is signed-minimum
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn is_imin(&self) -> bool {
        for_each!(self, x, {0..(self.len() - 1)} {
            if x != 0 {
                return false
            }
        });
        if self.extra() == 0 {
            self.last() == (isize::MIN as usize)
        } else {
            self.last() == (1 << (self.extra() - 1))
        }
    }

    /// If `self` is unsigned-one
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn is_uone(&self) -> bool {
        if self.first() != 1 {
            return false
        }
        for_each!(self, x, {1..self.len()} {
            if x != 0 {
                return false
            }
        });
        true
    }

    /// Equality comparison, `self == rhs`
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn const_eq(&self, rhs: &Self) -> Option<bool> {
        binop_for_each!(self, rhs, x, y, {0..self.len()}.rev() {
            if x != y {
                return Some(false)
            }
        });
        Some(true)
    }

    /// Not-equal comparison, `self != rhs`
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn const_ne(&self, rhs: &Self) -> Option<bool> {
        binop_for_each!(self, rhs, x, y, {0..self.len()}.rev() {
            if x != y {
                return Some(true)
            }
        });
        Some(false)
    }

    /// Unsigned-less-than comparison, `self < rhs`
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn ult(&self, rhs: &Self) -> Option<bool> {
        binop_for_each!(self, rhs, x, y, {0..self.len()}.rev() {
            if x < y {
                return Some(true)
            } else if x != y {
                return Some(false)
            }
            // else it is indeterminant and the next digit has to be checked
        });
        Some(false)
    }

    /// Unsigned-less-than-or-equal comparison, `self <= rhs`
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn ule(&self, rhs: &Self) -> Option<bool> {
        binop_for_each!(self, rhs, x, y, {0..self.len()}.rev() {
            if x < y {
                return Some(true)
            } else if x != y {
                return Some(false)
            }
        });
        Some(true)
    }

    /// Unsigned-greater-than comparison, `self > rhs`
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn ugt(&self, rhs: &Self) -> Option<bool> {
        binop_for_each!(self, rhs, x, y, {0..self.len()}.rev() {
            if x < y {
                return Some(false)
            } else if x != y {
                return Some(true)
            }
        });
        Some(false)
    }

    /// Unsigned-greater-than-or-equal comparison, `self >= rhs`
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn uge(&self, rhs: &Self) -> Option<bool> {
        binop_for_each!(self, rhs, x, y, {0..self.len()}.rev() {
            if x < y {
                return Some(false)
            } else if x != y {
                return Some(true)
            }
        });
        Some(true)
    }

    /// Signed-less-than comparison, `self < rhs`
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn ilt(&self, rhs: &Self) -> Option<bool> {
        binop_for_each!(self, rhs, x, y, {
            if self.msb() != rhs.msb() {
                return Some(self.msb())
            }
        },
        {0..self.len()}.rev() {
            if x < y {
                return Some(true)
            } else if x != y {
                return Some(false)
            }
        });
        Some(false)
    }

    /// Signed-less-than-or-equal comparison, `self <= rhs`
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn ile(&self, rhs: &Self) -> Option<bool> {
        binop_for_each!(self, rhs, x, y, {
            if self.msb() != rhs.msb() {
                return Some(self.msb())
            }
        },
        {0..self.len()}.rev() {
            if x < y {
                return Some(true)
            } else if x != y {
                return Some(false)
            }
        });
        Some(true)
    }

    /// Signed-greater-than comparison, `self > rhs`
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn igt(&self, rhs: &Self) -> Option<bool> {
        binop_for_each!(self, rhs, x, y, {
            if self.msb() != rhs.msb() {
                return Some(rhs.msb())
            }
        },
        {0..self.len()}.rev() {
            if x < y {
                return Some(false)
            } else if x != y {
                return Some(true)
            }
        });
        Some(false)
    }

    /// Signed-greater-than-or-equal comparison, `self >= rhs`
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn ige(&self, rhs: &Self) -> Option<bool> {
        binop_for_each!(self, rhs, x, y, {
            if self.msb() != rhs.msb() {
                return Some(rhs.msb())
            }
        },
        {0..self.len()}.rev() {
            if x < y {
                return Some(false)
            } else if x != y {
                return Some(true)
            }
        });
        Some(true)
    }
}
