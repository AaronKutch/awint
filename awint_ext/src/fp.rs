use core::{
    borrow::BorrowMut,
    fmt,
    fmt::Debug,
    hash::{Hash, Hasher},
    num::NonZeroUsize,
    ops::{Deref, DerefMut},
};

use awint_core::Bits;

/// Fixed-Point Type, containing signedness, bitwidth, and fixed point
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FPType {
    pub signed: bool,
    pub bw: NonZeroUsize,
    pub fp: isize,
}

/// Fixed-Point wrapper around structs that implement `Borrow<Bits>` and
/// `BorrowMut<Bits>`. Adds on signedness and fixed-point information.
#[derive(Debug)]
pub struct FP<B: BorrowMut<Bits>> {
    signed: bool,
    fp: isize,
    bits: B,
}

impl<B: BorrowMut<Bits>> FP<B> {
    /// Creates a fixed-point wrapper `FP<B>` from a specified signedness `signed`, wrapped value `B`, and fixed point `fp`
    #[inline]
    pub fn new(signed: bool, bits: B, fp: isize) -> Self {
        Self { signed, fp, bits }
    }

    /// Returns the inner `B` value
    #[inline]
    pub fn into_inner(this: Self) -> B {
        this.bits
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

    /// Returns the sign of `self`, returning `Some(self.const_as_ref().msb())` if `self.signed()`, and `None` otherwise.
    #[inline]
    pub fn sign(&self) -> Option<bool> {
        if self.signed() {
            Some(self.const_as_ref().msb())
        } else {
            None
        }
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

    /// Returns the fixed point of `self`
    #[inline]
    pub fn fp(&self) -> isize {
        self.fp
    }

    /// Returns the `FPType` of `self`
    #[inline]
    pub fn fp_type(&self) -> FPType {
        FPType {
            signed: self.signed(),
            fp: self.fp(),
            bw: self.nzbw(),
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

impl<B: PartialEq + BorrowMut<Bits>> PartialEq for FP<B> {
    /// The signedness, fixed point, and `PartialEq` implementation on
    /// `FP::into_inner(self)` must all be `true` in order for this to return
    /// `true`
    fn eq(&self, rhs: &Self) -> bool {
        (self.signed == rhs.signed) && (self.fp == rhs.fp) && (self.bits == rhs.bits)
    }
}

impl<B: PartialEq + Eq + BorrowMut<Bits>> Eq for FP<B> {}

macro_rules! impl_fmt {
    ($($ty:ident)*) => {
        $(
            /// Forwards to the corresponding impl for `Bits`
            impl<B: fmt::$ty + BorrowMut<Bits>> fmt::$ty for FP<B> {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    // TODO what about sign, fp, and `B`?
                    // `0x1234.5678x-3p16u32`
                    // `0o1234.567x-3p16u32`
                    // `0b11010.011x-3p16u32`
                    fmt::$ty::fmt(self.const_as_ref(), f)
                }
            }
        )*
    };
}

impl_fmt!(Display LowerHex UpperHex Octal Binary);

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
