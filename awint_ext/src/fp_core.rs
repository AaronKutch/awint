use core::{
    borrow::BorrowMut,
    fmt,
    fmt::Debug,
    hash::{Hash, Hasher},
    num::NonZeroUsize,
    ops::{Deref, DerefMut},
};

use awint_core::Bits;

use crate::ExtAwi;

/// Fixed-Point Type, containing signedness, bitwidth, and fixed point
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FPType {
    pub signed: bool,
    pub bw: NonZeroUsize,
    pub fp: isize,
}

impl FPType {
    // TODO: when adding `unique_min_integer_digits` add a note for the user to
    // handle the sign indicator by making space for a ' ' character.

    /// Returns the minimum number of digits in a given `radix` needed for
    /// unique representation of the fraction part of this fixed point type.
    /// This function performs allocation. Returns `None` if `radix < 2`.
    #[must_use]
    pub fn unique_min_fraction_digits(&self, radix: usize) -> Option<usize> {
        if radix < 2 {
            return None
        }
        if self.fp <= 0 {
            return Some(0)
        }
        let mut test = ExtAwi::uone(NonZeroUsize::new(self.fp.unsigned_abs()).unwrap());
        let mut digits = 0;
        loop {
            digits += 1;
            if test.short_cin_mul(0, radix) != 0 {
                // as soon as overflow happens, that means
                // `(((radix^digits) * 1 ULP) >> this.fp()) > 0`
                break
            }
        }
        Some(digits)
    }
}

/// Fixed-Point generic struct for `B` that implement `Borrow<Bits>` and
/// `BorrowMut<Bits>`. Adds on signedness and fixed-point information.
/// Implements many traits if `B` also implements them.
///
/// In order to make many operations infallible, `self.fp().unsigned_abs()` and
/// `self.bw()` follow an invariant that they are never greater than
/// `usize::MAX >> 2`.
///
/// NOTE: `B` should not change the bitwidth of the underlying `Bits` during the
/// lifetime of the `FP` struct unless the invariants are upheld. Otherwise,
/// panics and arithmetic errors can occur. Preferably, `into_b` and `FP::new`
/// should be used to create a fresh struct.
pub struct FP<B: BorrowMut<Bits>> {
    signed: bool,
    fp: isize,
    bits: B,
}

// TODO we will probably store the signed and reversimal bits in the 2 lsb bits
// of `fp`

impl<B: BorrowMut<Bits>> FP<B> {
    /// Creates a fixed-point generic `FP<B>` from a specified signedness
    /// `signed`, wrapped value `B`, and fixed point `fp`. This returns `None`
    /// if `bits.bw()` or `fp.unsigned_abs()` are greater than
    /// `usize::MAX >> 2`.
    #[inline]
    pub fn new(signed: bool, bits: B, fp: isize) -> Option<Self> {
        if (bits.borrow().bw() > (usize::MAX >> 2)) || (fp.unsigned_abs() > (usize::MAX >> 2)) {
            None
        } else {
            Some(Self { signed, fp, bits })
        }
    }

    /// Consumes `this`, returning the inner `B`
    #[inline]
    pub fn into_b(self) -> B {
        self.bits
    }

    /// Returns a reference to the `B` in `self`
    #[inline]
    pub fn b(&self) -> &B {
        &self.bits
    }

    /// Returns a mutable reference to the `B` in `self`
    #[inline]
    pub fn b_mut(&mut self) -> &mut B {
        &mut self.bits
    }

    /// Returns a reference to `self` in the form of `&Bits`
    #[inline]
    pub fn const_as_ref(&self) -> &Bits {
        self.bits.borrow()
    }

    /// Returns a reference to `self` in the form of `&mut Bits`
    #[inline]
    pub fn const_as_mut(&mut self) -> &mut Bits {
        self.bits.borrow_mut()
    }

    /// Returns the signedness of `self`
    #[inline]
    pub fn signed(&self) -> bool {
        self.signed
    }

    /// Returns the sign of `self`, returning `Some(self.const_as_ref().msb())`
    /// if `self.signed()`, and `None` otherwise.
    #[inline]
    pub fn sign(&self) -> Option<bool> {
        if self.signed() {
            Some(self.const_as_ref().msb())
        } else {
            None
        }
    }

    /// Returns if `self.signed() && self.const_as_ref().msb()`
    #[inline]
    pub fn is_negative(&self) -> bool {
        self.signed() && self.const_as_ref().msb()
    }

    /// Returns the bitwidth of `self` as a `NonZeroUsize`
    #[inline]
    pub fn nzbw(&self) -> NonZeroUsize {
        self.const_as_ref().nzbw()
    }

    /// Returns the bitwidth of `self` as a `usize`
    #[inline]
    pub fn bw(&self) -> usize {
        self.const_as_ref().bw()
    }

    /// Returns the bitwidth of `self` as an `isize`
    #[inline]
    pub fn ibw(&self) -> isize {
        // this is ok because of the guard in `FP::new`
        self.const_as_ref().bw() as isize
    }

    /// Returns the fixed point of `self`
    #[inline]
    pub fn fp(&self) -> isize {
        self.fp
    }

    /// Returns the `FPType` of `self`. Because `FPType` impls `PartialEq`, this
    /// is useful for quickly determining if two different `FP`s have the same
    /// fixed point type.
    #[inline]
    pub fn fp_ty(&self) -> FPType {
        FPType {
            signed: self.signed(),
            fp: self.fp(),
            bw: self.nzbw(),
        }
    }

    /// Sets the fixed point of `self`. Returns `None` if `fp.unsigned_abs()` is
    /// greater than `usize::MAX >> 2`.
    pub fn set_fp(&mut self, fp: isize) -> Option<()> {
        if fp.unsigned_abs() > (usize::MAX >> 2) {
            None
        } else {
            self.fp = fp;
            Some(())
        }
    }
}

impl<B: BorrowMut<Bits>> Deref for FP<B> {
    type Target = Bits;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.const_as_ref()
    }
}

impl<B: BorrowMut<Bits>> DerefMut for FP<B> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Bits {
        self.const_as_mut()
    }
}

impl<B: Clone + BorrowMut<Bits>> Clone for FP<B> {
    fn clone(&self) -> Self {
        Self {
            signed: self.signed,
            fp: self.fp,
            bits: self.bits.clone(),
        }
    }
}

impl<B: Copy + BorrowMut<Bits>> Copy for FP<B> {}

impl<B: PartialEq + BorrowMut<Bits>> PartialEq for FP<B> {
    /// The signedness, fixed point, and `PartialEq` implementation on
    /// [FP::into_b] must all be `true` in order for this to return
    /// `true`
    fn eq(&self, rhs: &Self) -> bool {
        (self.signed == rhs.signed) && (self.fp == rhs.fp) && (self.bits == rhs.bits)
    }
}

impl<B: PartialEq + Eq + BorrowMut<Bits>> Eq for FP<B> {}

macro_rules! impl_fmt {
    ($($ty:ident, $radix_str:expr, $radix:expr, $upper:expr);*;) => {
        $(
            impl<B: fmt::$ty + BorrowMut<Bits>> fmt::$ty for FP<B> {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    let (integer, fraction) = FP::to_str_general(self, $radix, $upper, 1, 1)
                        .ok().ok_or(fmt::Error)?;
                    let sign = if self.is_negative() {
                        "-"
                    } else {
                        ""
                    };
                    let signed = if self.signed() {
                        'i'
                    } else {
                        'u'
                    };
                    f.write_fmt(format_args!(
                        "{}{}{}.{}_{}{}f{}",
                        sign,
                        $radix_str,
                        integer,
                        fraction,
                        signed,
                        self.bw(),
                        self.fp()
                    ))
                }
            }
        )*
    };
}

impl_fmt!(
    Debug, "", 10, false;
    Display, "", 10, false;
    LowerHex, "0x", 16, false;
    UpperHex, "0x", 16, true;
    Octal, "0o", 8, false;
    Binary, "0b", 2, false;
);

impl<B: Hash + BorrowMut<Bits>> Hash for FP<B> {
    /// Uses the hash of `self.signed()`, `self.fp()`, and the `Hash`
    /// implementation on `FP::into_inner(self)` (not `self.const_as_ref()`)
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.signed.hash(state);
        self.fp.hash(state);
        // should include other state that `B` might have
        self.bits.hash(state);
    }
}
