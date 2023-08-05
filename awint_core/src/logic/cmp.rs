use core::{
    borrow::BorrowMut,
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    ops::{Deref, DerefMut},
};

use awint_internals::*;
use const_fn::const_fn;

use crate::Bits;

/// A wrapper implementing total ordering
///
/// Implements `PartialEq`, `Eq`, `PartialOrd`, and `Ord` by using
/// `Bits::total_cmp`. `Hash` also uses the `Bits`. This does not specify
/// anything other than that it provides a total ordering over bit strings
/// (including differentiating by the bit width). This is intended for fast
/// comparisons in ordered structures.
pub struct OrdBits<B: BorrowMut<Bits>>(pub B);

impl<B: BorrowMut<Bits>> PartialEq for OrdBits<B> {
    fn eq(&self, rhs: &Self) -> bool {
        self.0.borrow() == rhs.0.borrow()
    }
}

impl<B: BorrowMut<Bits>> Eq for OrdBits<B> {}

impl<B: BorrowMut<Bits>> PartialOrd for OrdBits<B> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0.borrow().total_cmp(other.0.borrow()))
    }
}

impl<B: BorrowMut<Bits>> Ord for OrdBits<B> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.borrow().total_cmp(other.0.borrow())
    }
}

impl<B: BorrowMut<Bits>> Hash for OrdBits<B> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.borrow().hash(state);
    }
}

impl<B: BorrowMut<Bits>> Deref for OrdBits<B> {
    type Target = Bits;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0.borrow()
    }
}

impl<B: BorrowMut<Bits>> DerefMut for OrdBits<B> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Bits {
        self.0.borrow_mut()
    }
}

impl<B: Clone + BorrowMut<Bits>> Clone for OrdBits<B> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<B: Copy + BorrowMut<Bits>> Copy for OrdBits<B> {}

macro_rules! impl_fmt {
    ($($ty:ident)*) => {
        $(
            impl<B: fmt::$ty + BorrowMut<Bits>> fmt::$ty for OrdBits<B> {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    fmt::$ty::fmt(&self.0, f)
                }
            }
        )*
    };
}

impl_fmt!(
    Debug Display LowerHex UpperHex Octal Binary
);

/// # Comparison
impl Bits {
    /// If `self` is zero
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn is_zero(&self) -> bool {
        unsafe_for_each!(self, x, {
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
        unsafe_for_each!(self, x, {0..(self.len() - 1)} {
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
        unsafe_for_each!(self, x, {0..(self.len() - 1)} {
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
        unsafe_for_each!(self, x, {0..(self.len() - 1)} {
            if x != 0 {
                return false
            }
        });
        if self.extra() == 0 {
            self.last() == IDigit::MIN as Digit
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
        unsafe_for_each!(self, x, {1..self.len()} {
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
        unsafe_binop_for_each!(self, rhs, x, y, {0..self.len()}.rev() {
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
        unsafe_binop_for_each!(self, rhs, x, y, {0..self.len()}.rev() {
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
        unsafe_binop_for_each!(self, rhs, x, y, {0..self.len()}.rev() {
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
        unsafe_binop_for_each!(self, rhs, x, y, {0..self.len()}.rev() {
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
        unsafe_binop_for_each!(self, rhs, x, y, {0..self.len()}.rev() {
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
        unsafe_binop_for_each!(self, rhs, x, y, {0..self.len()}.rev() {
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
        unsafe_binop_for_each!(self, rhs, x, y, {
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
        unsafe_binop_for_each!(self, rhs, x, y, {
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
        unsafe_binop_for_each!(self, rhs, x, y, {
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
        unsafe_binop_for_each!(self, rhs, x, y, {
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

    /// Total ordering for `self` and `rhs`, including differentiation between
    /// differing bitwidths of `self` and `rhs`. This is intended just to
    /// provide some way of ordering over all possible bit strings.
    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn total_cmp(&self, rhs: &Self) -> Ordering {
        if self.bw() != rhs.bw() {
            if self.bw() < rhs.bw() {
                return Ordering::Less
            } else {
                return Ordering::Greater
            }
        }
        unsafe {
            // Safety: This accesses all regular digits within their bounds. If the
            // bitwidths are equal, then the slice lengths are also equal.
            const_for!(i in {0..self.len()} {
                let x = self.get_unchecked(i);
                let y = rhs.get_unchecked(i);
                if x < y {
                    return Ordering::Less
                } else if x != y {
                    return Ordering::Greater
                }
                // else it is indeterminant and the next digit has to be checked
            });
        }
        Ordering::Equal
    }
}
