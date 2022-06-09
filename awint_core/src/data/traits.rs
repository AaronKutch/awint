use core::{
    borrow::{Borrow, BorrowMut},
    ops::{Deref, DerefMut, Index, IndexMut, RangeFull},
};

use crate::{Bits, InlAwi};

impl<const BW: usize, const LEN: usize> Deref for InlAwi<BW, LEN> {
    type Target = Bits;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.const_as_ref()
    }
}

impl<const BW: usize, const LEN: usize> DerefMut for InlAwi<BW, LEN> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Bits {
        self.const_as_mut()
    }
}

impl<const BW: usize, const LEN: usize> Index<RangeFull> for InlAwi<BW, LEN> {
    type Output = Bits;

    #[inline]
    fn index(&self, _i: RangeFull) -> &Bits {
        self.const_as_ref()
    }
}

impl<const BW: usize, const LEN: usize> Borrow<Bits> for InlAwi<BW, LEN> {
    #[inline]
    fn borrow(&self) -> &Bits {
        self.const_as_ref()
    }
}

impl<const BW: usize, const LEN: usize> AsRef<Bits> for InlAwi<BW, LEN> {
    #[inline]
    fn as_ref(&self) -> &Bits {
        self.const_as_ref()
    }
}

impl<const BW: usize, const LEN: usize> IndexMut<RangeFull> for InlAwi<BW, LEN> {
    #[inline]
    fn index_mut(&mut self, _i: RangeFull) -> &mut Bits {
        self.const_as_mut()
    }
}

impl<const BW: usize, const LEN: usize> BorrowMut<Bits> for InlAwi<BW, LEN> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut Bits {
        self.const_as_mut()
    }
}

impl<const BW: usize, const LEN: usize> AsMut<Bits> for InlAwi<BW, LEN> {
    #[inline]
    fn as_mut(&mut self) -> &mut Bits {
        self.const_as_mut()
    }
}
