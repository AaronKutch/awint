use core::{
    borrow::{Borrow, BorrowMut},
    ops::{Deref, DerefMut},
};

use crate::{data::inlawi::UsizeInlAwi, Bits, InlAwi};

impl<const BW: usize, const LEN: usize> Deref for InlAwi<BW, LEN> {
    type Target = Bits;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.internal_as_ref()
    }
}

impl<const BW: usize, const LEN: usize> DerefMut for InlAwi<BW, LEN> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Bits {
        self.internal_as_mut()
    }
}

impl<const BW: usize, const LEN: usize> Borrow<Bits> for InlAwi<BW, LEN> {
    #[inline]
    fn borrow(&self) -> &Bits {
        self
    }
}

impl<const BW: usize, const LEN: usize> AsRef<Bits> for InlAwi<BW, LEN> {
    #[inline]
    fn as_ref(&self) -> &Bits {
        self
    }
}

impl<const BW: usize, const LEN: usize> BorrowMut<Bits> for InlAwi<BW, LEN> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut Bits {
        self
    }
}

impl<const BW: usize, const LEN: usize> AsMut<Bits> for InlAwi<BW, LEN> {
    #[inline]
    fn as_mut(&mut self) -> &mut Bits {
        self
    }
}

impl From<bool> for InlAwi<1, { Bits::unstable_raw_digits(1) }> {
    /// Creates an `InlAwi` with one bit set to this `bool`
    fn from(x: bool) -> Self {
        Self::from_bool(x)
    }
}

macro_rules! inlawi_from {
    ($($w:expr, $u:ident $from_u:ident $i:ident $from_i:ident);*;) => {
        $(
            impl From<$u> for InlAwi<$w, {Bits::unstable_raw_digits($w)}> {
                /// Creates an `InlAwi` with the same bitwidth and bits as the integer
                fn from(x: $u) -> Self {
                    Self::$from_u(x)
                }
            }

            impl From<$i> for InlAwi<$w, {Bits::unstable_raw_digits($w)}> {
                /// Creates an `InlAwi` with the same bitwidth and bits as the integer
                fn from(x: $i) -> Self {
                    Self::$from_i(x)
                }
            }
        )*
    };
}

inlawi_from!(
    8, u8 from_u8 i8 from_i8;
    16, u16 from_u16 i16 from_i16;
    32, u32 from_u32 i32 from_i32;
    64, u64 from_u64 i64 from_i64;
    128, u128 from_u128 i128 from_i128;
);

impl From<usize> for UsizeInlAwi {
    /// Creates an `InlAwi` with the same bitwidth and bits as the integer
    fn from(x: usize) -> Self {
        Self::from_usize(x)
    }
}

impl From<isize> for UsizeInlAwi {
    /// Creates an `InlAwi` with the same bitwidth and bits as the integer
    fn from(x: isize) -> Self {
        Self::from_isize(x)
    }
}

/// If `self` and `other` have unmatching bit widths, `false` will be returned.
impl PartialEq for Bits {
    fn eq(&self, rhs: &Self) -> bool {
        self.bw() == rhs.bw() && self.const_eq(rhs).unwrap()
    }
}

/// If `self` and `other` have unmatching bit widths, `false` will be returned.
impl Eq for Bits {}

/// Common trait for obtaining `&Bits`
///
/// This trait exists to avoid blanket impl problems, and to give custom storage
/// types a way to independently define other common traits
pub trait AsBits {
    fn as_bits(&self) -> &Bits;
}

impl<'a, T: AsBits + ?Sized> AsBits for &'a T {
    fn as_bits(&self) -> &Bits {
        <T as AsBits>::as_bits(self)
    }
}

impl<'a, T: AsBits + ?Sized> AsBits for &'a mut T {
    fn as_bits(&self) -> &Bits {
        <T as AsBits>::as_bits(self)
    }
}

/// Common trait for obtaining `&mut Bits`
pub trait AsMutBits: AsBits {
    fn as_mut_bits(&mut self) -> &mut Bits;
}

impl<'a, T: AsMutBits + ?Sized> AsMutBits for &'a mut T {
    fn as_mut_bits(&mut self) -> &mut Bits {
        <T as AsMutBits>::as_mut_bits(self)
    }
}

impl AsBits for Bits {
    #[inline]
    fn as_bits(&self) -> &Bits {
        self
    }
}

impl AsMutBits for Bits {
    #[inline]
    fn as_mut_bits(&mut self) -> &mut Bits {
        self
    }
}

impl<const BW: usize, const LEN: usize> AsBits for InlAwi<BW, LEN> {
    fn as_bits(&self) -> &Bits {
        self.internal_as_ref()
    }
}

impl<const BW: usize, const LEN: usize> AsMutBits for InlAwi<BW, LEN> {
    fn as_mut_bits(&mut self) -> &mut Bits {
        self.internal_as_mut()
    }
}
