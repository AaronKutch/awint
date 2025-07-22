use core::{
    borrow::{Borrow, BorrowMut},
    ops::{Deref, DerefMut},
};

use crate::{data::inlawi::UsizeInlAwi, Bits, InlAwi};

impl<const BW: usize, const LEN: usize> const Deref for InlAwi<BW, LEN> {
    type Target = Bits;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.internal_as_ref()
    }
}

impl<const BW: usize, const LEN: usize> const DerefMut for InlAwi<BW, LEN> {
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

impl const From<bool> for InlAwi<1, { Bits::unstable_raw_digits(1) }> {
    /// Creates an `InlAwi` with one bit set to this `bool`
    fn from(x: bool) -> Self {
        Self::from_bool(x)
    }
}

macro_rules! inlawi_from {
    ($($w:expr, $u:ident $from_u:ident $i:ident $from_i:ident);*;) => {
        $(
            impl const From<$u> for InlAwi<$w, {Bits::unstable_raw_digits($w)}> {
                /// Creates an `InlAwi` with the same bitwidth and bits as the integer
                fn from(x: $u) -> Self {
                    Self::$from_u(x)
                }
            }

            impl const From<$i> for InlAwi<$w, {Bits::unstable_raw_digits($w)}> {
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

impl const From<usize> for UsizeInlAwi {
    /// Creates an `InlAwi` with the same bitwidth and bits as the integer
    fn from(x: usize) -> Self {
        Self::from_usize(x)
    }
}

impl const From<isize> for UsizeInlAwi {
    /// Creates an `InlAwi` with the same bitwidth and bits as the integer
    fn from(x: isize) -> Self {
        Self::from_isize(x)
    }
}

/// If `self` and `other` have unmatching bit widths, `false` will be returned.
impl const PartialEq for Bits {
    fn eq(&self, rhs: &Self) -> bool {
        self.bw() == rhs.bw() && self.const_eq(rhs).unwrap()
    }
}

/// If `self` and `other` have unmatching bit widths, `false` will be returned.
impl Eq for Bits {}

/// Common trait for obtaining `&Bits`
///
/// This trait exists to avoid blanket impl problems, and to give custom storage
/// types a way to independently define other common traits.
#[const_trait]
pub trait AsBits {
    #[inline]
    fn as_bits(&self) -> &Bits;
}

impl<'a, T: AsBits> AsBits for &'a T {
    fn as_bits(&self) -> &Bits {
        <T as AsBits>::as_bits(self)
    }
}

impl<'a, T: AsBits> AsBits for &'a mut T {
    fn as_bits(&self) -> &Bits {
        <T as AsBits>::as_bits(self)
    }
}

/// Common trait for obtaining `&mut Bits`
#[const_trait]
pub trait AsMutBits: AsBits {
    #[inline]
    fn as_mut_bits(&mut self) -> &mut Bits;
}

impl<'a, T: AsMutBits> AsMutBits for &'a mut T {
    fn as_mut_bits(&mut self) -> &mut Bits {
        <T as AsMutBits>::as_mut_bits(self)
    }
}

impl const AsBits for Bits {
    fn as_bits(&self) -> &Bits {
        self
    }
}

impl<'a> const AsBits for &'a Bits {
    fn as_bits(&self) -> &Bits {
        self
    }
}

impl<'a> const AsBits for &'a mut Bits {
    fn as_bits(&self) -> &Bits {
        self
    }
}

impl const AsMutBits for Bits {
    fn as_mut_bits(&mut self) -> &mut Bits {
        self
    }
}

impl<const BW: usize, const LEN: usize> const AsBits for InlAwi<BW, LEN> {
    fn as_bits(&self) -> &Bits {
        self.internal_as_ref()
    }
}

impl<const BW: usize, const LEN: usize> const AsMutBits for InlAwi<BW, LEN> {
    fn as_mut_bits(&mut self) -> &mut Bits {
        self.internal_as_mut()
    }
}
