use core::ops::{
    AddAssign, BitAndAssign, BitOrAssign, BitXorAssign, MulAssign, ShlAssign, SubAssign,
};

use crate::Bits;

impl ShlAssign<usize> for Bits {
    fn shl_assign(&mut self, s: usize) {
        self.shl_assign(s).unwrap();
    }
}

impl<'a> BitAndAssign<&'a Bits> for Bits {
    fn bitand_assign(&mut self, rhs: &'a Self) {
        self.and_assign(rhs).unwrap();
    }
}

impl<'a> BitOrAssign<&'a Bits> for Bits {
    fn bitor_assign(&mut self, rhs: &'a Self) {
        self.or_assign(rhs).unwrap();
    }
}

impl<'a> BitXorAssign<&'a Bits> for Bits {
    fn bitxor_assign(&mut self, rhs: &'a Self) {
        self.xor_assign(rhs).unwrap();
    }
}

impl<'a> AddAssign<&'a Bits> for Bits {
    fn add_assign(&mut self, rhs: &'a Self) {
        self.add_assign(rhs).unwrap();
    }
}

impl<'a> SubAssign<&'a Bits> for Bits {
    fn sub_assign(&mut self, rhs: &'a Self) {
        self.sub_assign(rhs).unwrap();
    }
}

impl MulAssign<u8> for Bits {
    fn mul_assign(&mut self, rhs: u8) {
        self.short_cin_mul(0, rhs as usize);
    }
}
