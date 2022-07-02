use core::{
    borrow::{Borrow, BorrowMut},
    ops::{Deref, DerefMut, Index, IndexMut, RangeFull},
};

use super::inlawi::UsizeInlAwi;
use crate::{Bits, InlAwi};

impl<const BW: usize, const LEN: usize> const Deref for InlAwi<BW, LEN> {
    type Target = Bits;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.const_as_ref()
    }
}

impl<const BW: usize, const LEN: usize> const DerefMut for InlAwi<BW, LEN> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Bits {
        self.const_as_mut()
    }
}

impl<const BW: usize, const LEN: usize> const Index<RangeFull> for InlAwi<BW, LEN> {
    type Output = Bits;

    #[inline]
    fn index(&self, _i: RangeFull) -> &Bits {
        self.const_as_ref()
    }
}

impl<const BW: usize, const LEN: usize> const Borrow<Bits> for InlAwi<BW, LEN> {
    #[inline]
    fn borrow(&self) -> &Bits {
        self.const_as_ref()
    }
}

impl<const BW: usize, const LEN: usize> const AsRef<Bits> for InlAwi<BW, LEN> {
    #[inline]
    fn as_ref(&self) -> &Bits {
        self.const_as_ref()
    }
}

impl<const BW: usize, const LEN: usize> const IndexMut<RangeFull> for InlAwi<BW, LEN> {
    #[inline]
    fn index_mut(&mut self, _i: RangeFull) -> &mut Bits {
        self.const_as_mut()
    }
}

impl<const BW: usize, const LEN: usize> const BorrowMut<Bits> for InlAwi<BW, LEN> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut Bits {
        self.const_as_mut()
    }
}

impl<const BW: usize, const LEN: usize> const AsMut<Bits> for InlAwi<BW, LEN> {
    #[inline]
    fn as_mut(&mut self) -> &mut Bits {
        self.const_as_mut()
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
