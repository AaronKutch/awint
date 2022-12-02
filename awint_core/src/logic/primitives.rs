use awint_internals::*;
use const_fn::const_fn;

use crate::Bits;

// TODO these could probably be optimized, check assembly

macro_rules! bits_ {
    ($($unsigned_name:ident, $uX:ident, $signed_name:ident, $iX:ident);*;) => {
        $(
            #[const_fn(cfg(feature = "const_support"))]
            pub const fn $unsigned_name(&mut self, x: $uX) {
                const BW: usize = $uX::BITS as usize;
                const LEN: usize = BW / BITS;
                let mut x = x;
                if LEN < 2 {
                    *self.first_mut() = x as usize;
                    if self.len() > 1 {
                        unsafe {
                        self.digit_set(false, 1..self.len(), false);}
                    }
                } else if self.bw() > BW {
                    // Safety: there are at least `LEN` digits in `self`
                    unsafe {
                        const_for!(i in {0..LEN} {
                            *self.get_unchecked_mut(i) = x as usize;
                            x = x.wrapping_shr(usize::BITS);
                        });
                        self.digit_set(false, LEN..self.len(), false);
                    }
                } else {
                    unsafe {
                        const_for!(i in {0..self.len()} {
                            *self.get_unchecked_mut(i) = x as usize;
                            x = x.wrapping_shr(usize::BITS);
                        });
                    }
                }
                self.clear_unused_bits();
            }

            #[const_fn(cfg(feature = "const_support"))]
            pub const fn $signed_name(&mut self, x: $iX) {
                const BW: usize = $iX::BITS as usize;
                const LEN: usize = BW / BITS;
                let mut x = x;
                if LEN < 2 {
                    *self.first_mut() = x as isize as usize;
                    if self.len() >= 1 {
                        // Safety: there is at least 1 digit in `self`
                        unsafe {
                            self.digit_set(x < 0, 1..self.len(), true);
                        }
                    }
                } else if self.bw() >= BW {
                    // Safety: there are at least `LEN` digits in `self`
                    unsafe {
                        let sign = x < 0;
                        const_for!(i in {0..LEN} {
                            *self.get_unchecked_mut(i) = x as isize as usize;
                            x = x.wrapping_shr(usize::BITS);
                        });
                        self.digit_set(sign, LEN..self.len(), true);
                    }
                } else {
                    unsafe {
                        const_for!(i in {0..self.len()} {
                            *self.get_unchecked_mut(i) = x as isize as usize;
                            x = x.wrapping_shr(usize::BITS);
                        });
                    }
                    self.clear_unused_bits();
                }
            }
        )*
    };
}

/// # Primitive assignment
///
/// If `self.bw()` is smaller than the primitive bitwidth, truncation will be
/// used when copying bits from `x` to `self`. If the primitive is unsigned (or
/// is a boolean), then zero extension will be used if `self.bw()` is larger
/// than the primitive bitwidth. If the primitive is signed, then sign extension
/// will be used if `self.bw()` is larger than the primitive bitwidth.
impl Bits {
    bits_!(
        u8_, u8, i8_, i8;
        u16_, u16, i16_, i16;
        u32_, u32, i32_, i32;
        u64_, u64, i64_, i64;
        u128_, u128, i128_, i128;
        usize_, usize, isize_, isize;
    );

    #[const_fn(cfg(feature = "const_support"))]
    pub const fn bool_(&mut self, x: bool) {
        self.zero_();
        *self.first_mut() = x as usize;
    }
}

macro_rules! bits_convert {
    ($($unsigned_name:ident, $uX:ident, $signed_name:ident, $iX:ident);*;) => {
        $(
            #[const_fn(cfg(feature = "const_support"))]
            #[must_use]
            pub const fn $unsigned_name(&self) -> $uX {
                const BW: usize = $uX::BITS as usize;
                const LEN: usize = BW / BITS;
                if LEN < 2 {
                    self.first() as $uX
                } else if self.bw() >= BW {
                    // Safety: there are at least `LEN` digits in `self`
                    let mut tmp = 0;
                    unsafe {
                        const_for!(i in {0..LEN} {
                            tmp |= (self.get_unchecked(i) as $uX) << (i * BITS);
                        });
                    }
                    tmp
                } else {
                    let mut tmp = 0;
                    unsafe {
                        const_for!(i in {0..self.len()} {
                            tmp |= (self.get_unchecked(i) as $uX) << (i * BITS);
                        });
                    }
                    tmp
                }
            }

            #[const_fn(cfg(feature = "const_support"))]
            #[must_use]
            pub const fn $signed_name(&self) -> $iX {
                const BW: usize = $uX::BITS as usize;
                const LEN: usize = BW / BITS;
                if LEN < 2 && self.len() == 1 {
                    let sign_bit = 1usize << (self.bw() - 1);
                    let extension = $uX::MIN.wrapping_sub((self.first() & sign_bit) as $uX);
                    ((self.first() as $uX) | extension) as $iX
                } else {
                    let mut tmp = self.$unsigned_name() as $iX;
                    if self.bw() < BW {
                        if self.msb() {
                            let extension = $uX::MIN.wrapping_sub(1 << (self.bw() - 1)) as $iX;
                            tmp |= extension;
                        }
                    }
                    tmp
                }
            }
        )*
    };
}

/// # Primitive conversion
///
/// If `self.bw()` is larger than the primitive bitwidth, truncation will be
/// used when copying the bits of `self` and returning them. If the primitive is
/// unsigned, then zero extension will be used if `self.bw()` is smaller than
/// the primitive bitwidth. If the primitive is signed, then sign extension will
/// be used if `self.bw()` is smaller than the primitive bitwidth.
impl Bits {
    bits_convert!(
        to_u8, u8, to_i8, i8;
        to_u16, u16, to_i16, i16;
        to_u32, u32, to_i32, i32;
        to_u64, u64, to_i64, i64;
        to_u128, u128, to_i128, i128;
        to_usize, usize, to_isize, isize;
    );

    #[const_fn(cfg(feature = "const_support"))]
    #[must_use]
    pub const fn to_bool(&self) -> bool {
        (self.first() & 1) != 0
    }
}

impl From<&Bits> for bool {
    /// Returns the least significant bit
    fn from(x: &Bits) -> bool {
        x.to_bool()
    }
}

macro_rules! from_bits {
    ($($uX:ident, $to_u:ident, $iX:ident, $to_i:ident);*;) => {
        $(
            impl From<&Bits> for $uX {
                /// Zero-resizes the `Bits` to this integer
                fn from(x: &Bits) -> Self {
                    x.$to_u()
                }
            }

            impl From<&Bits> for $iX {
                /// Sign-resizes the `Bits` to this integer
                fn from(x: &Bits) -> Self {
                    x.$to_i()
                }
            }
        )*
    };
}

from_bits!(
    u8, to_u8, i8, to_i8;
    u16, to_u16, i16, to_i16;
    u32, to_u32, i32, to_i32;
    u64, to_u64, i64, to_i64;
    u128, to_u128, i128, to_i128;
    usize, to_usize, isize, to_isize;
);
