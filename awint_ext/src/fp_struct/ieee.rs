use core::borrow::BorrowMut;

use awint_core::{Bits, InlAwi};

use crate::FP;

/// A minimal `FP<inlawi_ty!(25)>` real number representation of an IEEE-754 32
/// bit floating point number.
///
/// This derives from 23 mantissa bits, the omitted leading 1 bit, and sign bit.
/// This cannot represent the infinity and NaN cases, but subnormal values can
/// be represented. Canonically, the fixed point is always 150 (the 127 bias
/// plus 23 mantissa bits) minus the raw IEEE-754 exponent.
pub type F32 = FP<InlAwi<25, { Bits::unstable_raw_digits(25) }>>;

/// A minimal `FP<inlawi_ty!(54)>` real number representation of an IEEE-754 64
/// bit floating point number.
///
/// This derives from 52 mantissa bits, the omitted leading 1 bit, and sign bit.
/// This cannot represent the infinity and NaN cases, but subnormal values can
/// be represented. Canonically, the fixed point is always 1075 (the 1023 bias
/// plus 52 mantissa bits) minus the raw IEEE-754 exponent.
pub type F64 = FP<InlAwi<54, { Bits::unstable_raw_digits(54) }>>;

impl F32 {
    /// Translates the IEEE-754 value of `f` to an [F32](crate::fp::F32),
    /// handling subnormal values correctly and casting the values of
    /// infinities and NaN to zero (with the fixed point always being 150
    /// minus the raw exponent).
    pub fn from_f32(f: f32) -> Self {
        let exponent = ((f.to_bits() >> 23) & ((1 << 8) - 1)) as u8;
        let sign = (f.to_bits() >> 31) != 0;
        let mantissa = f.to_bits() & ((1 << 23) - 1);

        let bits = InlAwi::zero();
        let mut res = FP::new(
            true,
            bits,
            ((1isize << 7) - 1 + 23).wrapping_sub(exponent as isize),
        )
        .unwrap();
        if exponent == 0 {
            if mantissa != 0 {
                // denormal
                let mut number = mantissa;
                if sign {
                    number = number.wrapping_neg();
                }
                res.u32_(number);
            } // else do nothing becuase it is zero
        } else if exponent != u8::MAX {
            // add leading 1
            let mut number = mantissa | (1 << 23);
            if sign {
                number = number.wrapping_neg();
            }
            res.u32_(number);
        } // else it is Infinity or NaN which is left as zero
        res
    }
}

impl F64 {
    /// Translates the IEEE-754 value of `f` to an [F64](crate::fp::F64),
    /// handling subnormal values correctly and casting the values of
    /// infinities and NaN to zero (with the fixed point always being 1075
    /// minus the raw exponent).
    pub fn from_f64(f: f64) -> Self {
        let exponent = ((f.to_bits() >> 52) & ((1 << 11) - 1)) as u16;
        let sign = (f.to_bits() >> 63) != 0;
        let mantissa = f.to_bits() & ((1 << 52) - 1);

        let bits = InlAwi::zero();
        let mut res = FP::new(
            true,
            bits,
            ((1isize << 10) - 1 + 52).wrapping_sub(exponent as isize),
        )
        .unwrap();
        if exponent == 0 {
            if mantissa != 0 {
                // denormal
                let mut number = mantissa;
                if sign {
                    number = number.wrapping_neg();
                }
                res.u64_(number);
            } // else do nothing becuase it is zero
        } else if exponent != ((1 << 11) - 1) {
            // add leading 1
            let mut number = mantissa | (1 << 52);
            if sign {
                number = number.wrapping_neg();
            }
            res.u64_(number);
        } // else it is Infinity or NaN which is left as zero
        res
    }
}

impl<B: BorrowMut<Bits>> FP<B> {
    /// Floating-assigns `FP::from_f32(f)` to `this`. Note that this modifies
    /// `this.fp` according to [floating_](FP::floating_).
    pub fn f32_(this: &mut Self, f: f32) {
        FP::floating_(this, &mut FP::from_f32(f)).unwrap()
    }

    /// The same as [f32_](FP::f32_) except `None` is returned if `f` is an
    /// infinity or NaN
    pub fn checked_f32_(this: &mut Self, f: f32) -> Option<()> {
        let exponent = ((f.to_bits() >> 23) & ((1 << 8) - 1)) as u8;
        if exponent == u8::MAX {
            return None
        }
        FP::f32_(this, f);
        Some(())
    }

    /// Floating-assigns `FP::from_f64(f)` to `this`. Note that this modifies
    /// `this.fp` according to [floating_](FP::floating_).
    pub fn f64_(this: &mut Self, f: f64) {
        FP::floating_(this, &mut FP::from_f64(f)).unwrap()
    }

    /// The same as [f64_](FP::f64_) except `None` is returned if `f` is an
    /// infinity or NaN
    pub fn checked_f64_(this: &mut Self, f: f64) -> Option<()> {
        let exponent = ((f.to_bits() >> 52) & ((1 << 11) - 1)) as u16;
        if exponent == ((1 << 11) - 1) {
            return None
        }
        FP::f64_(this, f);
        Some(())
    }

    /// Translates `this` to its IEEE-754 32 bit floating point value, using
    /// truncation rounding. Infinities and NaN are never returned. If the
    /// significant numerical value would be unrepresentable (i.e. the most
    /// significant numerical bit is 2^128 or greater), `None` is returned.
    pub fn try_to_f32(this: &mut Self) -> Option<f32> {
        if this.is_zero() {
            return Some(0.0)
        }
        let sign = this.is_negative();
        // note: reinterpret as unsigned
        this.neg_(sign);
        let lz = this.lz();
        // most significant numerical bit
        let msnb = FP::rel_sb(this).1.wrapping_sub(lz as isize);
        let res = if msnb > 127 {
            // overflow
            None
        } else if msnb < (-128 - 23) {
            // less than what subnormals can represent
            Some(0.0)
        } else {
            let mut mantissa = InlAwi::from_u32(0);
            let sig = this.sig();
            if msnb <= -127 {
                // subnormal
                let (from, width) = if sig >= 23 {
                    (sig.wrapping_sub(23), 23)
                } else {
                    (0, sig)
                };
                mantissa.field_from(this, from, width).unwrap();
                Some(f32::from_bits(mantissa.to_u32() | ((sign as u32) << 31)))
            } else {
                // normal
                let (to, from, width) = if sig >= 24 {
                    (0, sig.wrapping_sub(24), 23)
                } else {
                    (24usize.wrapping_sub(sig), 0, sig.wrapping_sub(1))
                };
                mantissa.field(to, this, from, width).unwrap();
                let exponent = (msnb as u32).wrapping_add(127);
                Some(f32::from_bits(
                    mantissa.to_u32() | (exponent << 23) | ((sign as u32) << 31),
                ))
            }
        };
        this.neg_(sign);
        res
    }

    /// Translates `this` to its IEEE-754 64 bit floating point value, using
    /// truncation rounding. Infinities and NaN are never returned. If the
    /// significant numerical value would be unrepresentable (i.e. the most
    /// significant numerical bit is 2^1024 or greater), `None` is returned.
    pub fn try_to_f64(this: &mut Self) -> Option<f64> {
        if this.is_zero() {
            return Some(0.0)
        }
        let sign = this.is_negative();
        // note: reinterpret as unsigned
        this.neg_(sign);
        let lz = this.lz();
        // most significant numerical bit
        let msnb = FP::rel_sb(this).1.wrapping_sub(lz as isize);
        let res = if msnb > 1023 {
            // overflow
            None
        } else if msnb < (-1024 - 52) {
            // less than what subnormals can represent
            Some(0.0)
        } else {
            let mut mantissa = InlAwi::from_u64(0);
            let sig = this.sig();
            if msnb <= -1023 {
                // subnormal
                let (from, width) = if sig >= 52 {
                    (sig.wrapping_sub(52), 52)
                } else {
                    (0, sig)
                };
                mantissa.field_from(this, from, width).unwrap();
                Some(f64::from_bits(mantissa.to_u64() | ((sign as u64) << 63)))
            } else {
                // normal
                let (to, from, width) = if sig >= 53 {
                    (0, sig.wrapping_sub(53), 52)
                } else {
                    (53usize.wrapping_sub(sig), 0, sig.wrapping_sub(1))
                };
                mantissa.field(to, this, from, width).unwrap();
                let exponent = (msnb as u64).wrapping_add(1023);
                Some(f64::from_bits(
                    mantissa.to_u64() | (exponent << 52) | ((sign as u64) << 63),
                ))
            }
        };
        this.neg_(sign);
        res
    }
}
