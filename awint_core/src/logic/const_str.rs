use core::fmt;

use awint_internals::*;
use const_fn::const_fn;
use SerdeError::*;

use crate::Bits;

/// Runs all pre serialization checks except for equal width and `Overflow`
/// checks
const fn verify_for_bytes_(src: &[u8], radix: u8) -> Result<(), SerdeError> {
    if radix < 2 || radix > 36 {
        return Err(InvalidRadix)
    }
    const_for!(i in {0..src.len()} {
        let b = src[i];
        if b == b'_' {
            continue;
        }
        let in_decimal_range = b'0' <= b && b < b'0'.wrapping_add(if radix < 10 {radix} else {10});
        let in_lower_range = (radix > 10)
            && (b'a' <= b)
            && (b < b'a'.wrapping_add(radix).wrapping_sub(10));
        let in_upper_range = (radix > 10)
            && (b'A' <= b)
            && (b < b'A'.wrapping_add(radix).wrapping_sub(10));
        if radix <= 10 {
            if !in_decimal_range {
                return Err(InvalidChar)
            }
        } else if !(in_decimal_range || in_lower_range || in_upper_range) {
            return Err(InvalidChar)
        }
    });
    Ok(())
}

/// # `const` string representation conversion
///
/// Note: the `awint_ext` crate has higher level allocating functions
/// `Awi::bits_to_string_radix`, `Awi::bits_to_vec_radix`, and
/// `<Awi as FromStr>::from_str`
impl Bits {
    /// A version of [Bits::bytes_radix_] optimized for power of two
    /// radixes
    #[doc(hidden)]
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn power_of_two_bytes_(
        &mut self,
        sign: Option<bool>,
        src: &[u8],
        radix: u8,
        pad: &mut Self,
    ) -> Result<(), SerdeError> {
        if self.bw() != pad.bw() {
            return Err(NonEqualWidths)
        }
        if !radix.is_power_of_two() {
            return Err(InvalidRadix)
        }
        let log2 = radix.trailing_zeros() as usize;
        if let Err(e) = verify_for_bytes_(src, radix) {
            return Err(e)
        }
        // the accumulator
        pad.zero_();
        let mut shl = 0;
        const_for!(i in {0..src.len()}.rev() {
            let b = src[i];
            if b == b'_' {
                continue;
            }
            let char_digit = if b <= b'9' {
                b.wrapping_sub(b'0')
            } else if b <= b'Z' {
                b.wrapping_sub(b'A').wrapping_add(10)
            } else {
                b.wrapping_sub(b'a').wrapping_add(10)
            } as Digit;
            pad.digit_or_(char_digit, shl);
            shl += log2;
            if shl >= self.bw() {
                let tmp = if let Some(tmp) = BITS
                    .wrapping_sub(char_digit.leading_zeros() as usize).checked_add(shl) {
                    if let Some(tmp) = tmp.checked_sub(log2) {
                        tmp
                    } else {
                        return Err(Overflow)
                    }
                } else {
                    return Err(Overflow)
                };
                // check that the last digit did not cross the end
                if tmp > self.bw() {
                    return Err(Overflow)
                }
                // there may be a bunch of leading zeros, so do not return an error yet
                const_for!(j in {0..i} {
                    match src[j] {
                        b'_' | b'0' => (),
                        _ => return Err(Overflow)
                    }
                });
                break
            }
        });
        if let Some(sign) = sign {
            if sign {
                if pad.lz() == 0 && !pad.is_imin() {
                    // These cannot be represented as negative
                    return Err(Overflow)
                }
                // handles `imin` correctly
                pad.neg_(true);
            } else if pad.lz() == 0 {
                // These cannot be represented as positive
                return Err(Overflow)
            }
        }
        self.copy_(pad).unwrap();
        Ok(())
    }

    /// Assigns to `self` the integer value represented by `src` in the given
    /// `radix`. If `src` should be interpreted as unsigned, `sign` should be
    /// `None`, otherwise it should be set to the sign. In order for this
    /// function to be `const`, two scratchpads `pad0` and `pad1` with the
    /// same bitwidth as `self` must be supplied, which can be mutated by
    /// the function in arbitrary ways.
    ///
    /// # Errors
    ///
    /// `self` is not mutated if an error occurs. See [crate::SerdeError] for
    /// error conditions. The characters `0..=9`, `a..=z`, and `A..=Z` are
    /// allowed depending on the radix. The char `_` is ignored, and all
    /// other chars result in an error. `src` cannot be empty. The value of
    /// the string must be representable in the bitwidth of `self` with the
    /// specified sign, otherwise an overflow error is returned.
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn bytes_radix_(
        &mut self,
        sign: Option<bool>,
        src: &[u8],
        radix: u8,
        pad0: &mut Self,
        pad1: &mut Self,
    ) -> Result<(), SerdeError> {
        if (self.bw() != pad0.bw()) || (self.bw() != pad1.bw()) {
            return Err(NonEqualWidths)
        }
        if radix.is_power_of_two() {
            return self.power_of_two_bytes_(sign, src, radix, pad0)
        }
        if let Err(e) = verify_for_bytes_(src, radix) {
            return Err(e)
        }
        // the accumulator
        pad0.zero_();
        // contains the radix exponential
        pad1.uone_();
        const_for!(i in {0..src.len()}.rev() {
            let b = src[i];
            if b == b'_' {
                continue;
            }
            let char_digit = if radix <= 10 || b <= b'9' {
                b.wrapping_sub(b'0')
            } else if b <= b'Z' {
                b.wrapping_sub(b'A').wrapping_add(10)
            } else {
                b.wrapping_sub(b'a').wrapping_add(10)
            } as Digit;
            let o0 = pad0.digit_mul_add_(pad1, char_digit).unwrap();
            if o0 {
                return Err(Overflow)
            }
            let o1 = pad1.digit_cin_mul_(0, radix as Digit);
            if o1 != 0 {
                // there may be a bunch of leading zeros, so do not return an error yet
                const_for!(j in {0..i} {
                    match src[j] {
                        b'_' | b'0' => (),
                        _ => return Err(Overflow)
                    }
                });
                break
            }
        });
        if let Some(sign) = sign {
            if sign {
                if pad0.lz() == 0 && !pad0.is_imin() {
                    // These cannot be represented as negative
                    return Err(Overflow)
                }
                // handles `imin` correctly
                pad0.neg_(true);
            } else if pad0.lz() == 0 {
                // These cannot be represented as positive
                return Err(Overflow)
            }
        }
        self.copy_(pad0).unwrap();
        Ok(())
    }

    /// Assigns the `[u8]` representation of `self` to `dst` (sign indicators,
    /// prefixes, and postfixes not included). `signed` specifies if `self`
    /// should be interpreted as signed. `radix` specifies the radix, and
    /// `upper` specifies if letters should be uppercase. In order for this
    /// function to be `const`, a scratchpad `pad` with the same bitwidth as
    /// `self` must be supplied. Note that if `dst.len()` is more than what
    /// is needed to store the representation, the leading bytes will all be
    /// set to b'0'.
    ///
    /// # Errors
    ///
    /// Note: If an error is returned, `dst` may be set to anything
    ///
    /// This function can fail from `NonEqualWidths`, `InvalidRadix`, and
    /// `Overflow` (if `dst` cannot represent the value of `self`). See
    /// [crate::SerdeError].
    #[const_fn(cfg(feature = "const_support"))]
    pub const fn to_bytes_radix(
        &self,
        signed: bool,
        dst: &mut [u8],
        radix: u8,
        upper: bool,
        pad: &mut Self,
    ) -> Result<(), SerdeError> {
        // there's going to be a lot of potential for downstream confusion if we do not
        // check
        self.assert_cleared_unused_bits();
        if self.bw() != pad.bw() {
            return Err(NonEqualWidths)
        }
        if radix < 2 || radix > 36 {
            return Err(InvalidRadix)
        }
        pad.copy_(self).unwrap();
        // happens to do the right thing to `imin`
        pad.neg_(signed && pad.msb());
        const_for!(i in {0..dst.len()}.rev() {
            let rem = pad.digit_udivide_inplace_(radix as Digit).unwrap() as u8;
            if rem < 10 {
                dst[i] = b'0'.wrapping_add(rem);
            } else if upper {
                dst[i] = b'A'.wrapping_add(rem).wrapping_sub(10);
            } else {
                dst[i] = b'a'.wrapping_add(rem).wrapping_sub(10);
            }
        });
        if !pad.is_zero() {
            Err(Overflow)
        } else {
            Ok(())
        }
    }

    /// Writes the bits content as hexadecimal to `f`, with underscores every 8
    /// digits. I have decided on including the "0x" prefix and bitwidth suffix
    /// always, because it is confusing in `assert_` debugging otherwise.
    #[inline]
    pub(crate) fn debug_format_hexadecimal(
        &self,
        f: &mut fmt::Formatter,
        upper: bool,
    ) -> fmt::Result {
        self.assert_cleared_unused_bits();
        f.write_fmt(format_args!("0x"))?;
        const_for!(j0 in {0..(self.bw() >> 2).wrapping_add(1)}.rev() {
            if (self.get_digit(j0 << 2) & 0b1111) != 0 {
                // we have reached the first nonzero character
                const_for!(j1 in {0..j0.wrapping_add(1)}.rev() {
                    let mut char_digit = (self.get_digit(j1 << 2) & 0b1111) as u8;
                    if char_digit < 10 {
                        char_digit += b'0';
                    } else if upper {
                        char_digit += b'A'.wrapping_sub(10);
                    } else {
                        char_digit += b'a'.wrapping_sub(10);
                    }
                    // Safety: we strictly capped the range of possible values above with `& 0b1111`
                    let c = unsafe { char::from_u32_unchecked(char_digit as u32) };
                    f.write_fmt(format_args!("{c}"))?;
                    if ((j1 % 8) == 0) && (j1 != 0) {
                        f.write_fmt(format_args!("_"))?;
                    }
                });
                break
            }
            if j0 == 0 {
                // we have reached the end without printing anything, print at least one '0'
                f.write_fmt(format_args!("{}", '0'))?;
            }
        });
        f.write_fmt(format_args!("_u{}", self.bw()))
    }

    #[inline]
    pub(crate) fn debug_format_octal(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.assert_cleared_unused_bits();
        f.write_fmt(format_args!("0o"))?;
        const_for!(j0 in {0..(self.bw() / 3).wrapping_add(1)}.rev() {
            if (self.get_digit(j0.wrapping_mul(3)) & 0b111) != 0 {
                // we have reached the first nonzero character
                const_for!(j1 in {0..j0.wrapping_add(1)}.rev() {
                    let mut char_digit = (self.get_digit(j1.wrapping_mul(3)) & 0b111) as u8;
                    char_digit += b'0';
                    // Safety: we strictly capped the range of possible values above with `& 0b111`
                    let c = unsafe { char::from_u32_unchecked(char_digit as u32) };
                    if let Err(e) = f.write_fmt(format_args!("{c}")) {
                        return Err(e)
                    }
                    if ((j1 % 8) == 0) && (j1 != 0) {
                        f.write_fmt(format_args!("_"))?;
                    }
                });
                break
            }
            if j0 == 0 {
                // we have reached the end without printing anything, print at least one '0'
                f.write_fmt(format_args!("{}", '0'))?;
            }
        });
        f.write_fmt(format_args!("_u{}", self.bw()))
    }

    // TODO this could be optimized
    #[inline]
    pub(crate) fn debug_format_binary(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.assert_cleared_unused_bits();
        f.write_fmt(format_args!("0b"))?;
        const_for!(j0 in {0..self.bw()}.rev() {
            if (self.get_digit(j0) & 0b1) != 0 {
                // we have reached the first nonzero character
                const_for!(j1 in {0..(j0 + 1)}.rev() {
                    let mut char_digit = (self.get_digit(j1) & 0b1) as u8;
                    char_digit += b'0';
                    // Safety: we strictly capped the range of possible values above with `& 0b1`
                    let c = unsafe { char::from_u32_unchecked(char_digit as u32) };
                    if let Err(e) = f.write_fmt(format_args!("{c}")) {
                        return Err(e)
                    }
                    if ((j1 % 8) == 0) && (j1 != 0) {
                        f.write_fmt(format_args!("_"))?;
                    }
                });
                break
            }
            if j0 == 0 {
                // we have reached the end without printing anything, print at least one '0'
                f.write_fmt(format_args!("{}", '0'))?;
            }
        });
        f.write_fmt(format_args!("_u{}", self.bw()))
    }
}
