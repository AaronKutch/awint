use alloc::{string::String, vec::Vec};
use core::{cmp, num::NonZeroUsize};

use awint_core::Bits;
use awint_internals::{SerdeError::*, *};

use crate::ExtAwi;

/// # non-`const` string representation conversion
impl ExtAwi {
    // note: we use the name `..._to_vec` instead of `..._to_bytes` to avoid name
    // collisions and confusion with the literal byte values instead of chars.

    /// Creates a `Vec<u8>` representing `bits` (sign indicators, prefixes, and
    /// postfixes not included). This function performs allocation. This is
    /// a wrapper around [awint_core::Bits::to_bytes_radix] that truncates
    /// leading zeros. An additional `min_chars` specifies the minimum
    /// number of characters that should exist. `min_chars` specifies the
    /// minimum number of chars in the integer part, inserting leading '0's if
    /// there are not enough chars, just like Rust's built in `{:0d}`
    /// formatting. Note that an empty vector will be returned if
    /// `min_chars == 0 && bits.is_zero()`.
    ///
    /// # Errors
    ///
    /// This can only return an error if `radix` is not in the range 2..=36 or
    /// if resource exhaustion occurs.
    pub fn bits_to_vec_radix(
        bits: &Bits,
        signed: bool,
        radix: u8,
        upper: bool,
        min_chars: usize,
    ) -> Result<Vec<u8>, SerdeError> {
        let mut dst = alloc::vec![0;
            cmp::max(
                min_chars,
                chars_upper_bound(bits.bw().wrapping_sub(bits.lz()), radix)?
            )
        ];
        let mut pad = ExtAwi::zero(bits.nzbw());
        // note: do not unwrap in case of exhaustion
        bits.to_bytes_radix(signed, &mut dst, radix, upper, pad.const_as_mut())?;
        let len = dst.len();
        for i in 0..len {
            if dst[i] != b'0' {
                // most significant digit
                let msd = i;
                // move downwards to get rid of leading zeros
                dst.copy_within(msd..len, 0);
                // this should be done for the sake of capacity determinism and for memory
                // limited contexts
                for _ in 0..msd {
                    dst.pop();
                }
                dst.shrink_to_fit();
                break
            }
            if i == len.wrapping_sub(min_chars) {
                // terminate early to keep the minimum number of chars
                // move the digits which are written in big endian form downwards to get rid of
                // leading zeros
                dst.copy_within(i..len, 0);
                for _ in 0..i {
                    dst.pop();
                }
                dst.shrink_to_fit();
                break
            }
            if (i + 1) == len {
                // all zeros
                for _ in 0..len {
                    dst.pop();
                }
                dst.shrink_to_fit();
                break
            }
        }
        Ok(dst)
    }

    /// Creates a string representing `bits`. This function performs allocation.
    /// This does the same thing as [ExtAwi::bits_to_vec_radix] but with a
    /// `String`.
    pub fn bits_to_string_radix(
        bits: &Bits,
        signed: bool,
        radix: u8,
        upper: bool,
        min_chars: usize,
    ) -> Result<String, SerdeError> {
        // It is impossible for the `from_utf8` conversion to panic because
        // `to_vec_radix` sets all chars to valid utf8
        Ok(String::from_utf8(ExtAwi::bits_to_vec_radix(
            bits, signed, radix, upper, min_chars,
        )?)
        .unwrap())
    }

    /// Creates an `ExtAwi` representing the given arguments. This function
    /// performs allocation. This is a wrapper around
    /// [awint_core::Bits::bytes_radix_assign] that zero or sign resizes the
    /// result to match `bw`.
    ///
    /// # Errors
    ///
    /// See the error conditions of [Bits::bytes_radix_assign]. Note that `-` is
    /// an invalid character even though `to_vec_radix` can return `-`. This
    /// is because we need to handle both unsigned and signed integer
    /// inputs, specified only by `sign`. If the input is a negative signed
    /// integer representation with `-` appended to the front, the subslice
    /// `src[1..]` can be taken and `sign` can be set to `Some(true)`.
    pub fn from_bytes_radix(
        sign: Option<bool>,
        src: &[u8],
        radix: u8,
        bw: NonZeroUsize,
    ) -> Result<ExtAwi, SerdeError> {
        let tmp_bw = awint_internals::bw(
            (sign.is_some() as usize)
                .checked_add(bits_upper_bound(src.len(), radix)?)
                .ok_or(Overflow)?,
        );
        let mut awi = ExtAwi::zero(tmp_bw);
        let mut pad0 = ExtAwi::zero(tmp_bw);
        let mut pad1 = ExtAwi::zero(tmp_bw);

        let tmp = awi.const_as_mut();
        // note: do not unwrap in case of exhaustion
        tmp.bytes_radix_assign(sign, src, radix, pad0.const_as_mut(), pad1.const_as_mut())?;

        let mut final_awi = ExtAwi::zero(bw);
        let x = final_awi.const_as_mut();
        if sign.is_none() {
            if x.zero_resize_assign(tmp) {
                return Err(Overflow)
            }
        } else if x.sign_resize_assign(tmp) {
            return Err(Overflow)
        }
        Ok(final_awi)
    }

    /// Creates an `ExtAwi` representing the given arguments. This does the same
    /// thing as [ExtAwi::from_bytes_radix] but with an `&str`.
    pub fn from_str_radix(
        sign: Option<bool>,
        str: &str,
        radix: u8,
        bw: NonZeroUsize,
    ) -> Result<ExtAwi, SerdeError> {
        ExtAwi::from_bytes_radix(sign, str.as_bytes(), radix, bw)
    }

    // note: these functions are not under `FP` because `FP` is a generic struct
    // agnostic to `ExtAwi`

    /// Creates an `ExtAwi` representing the given arguments. This function
    /// performs allocation. In addition to the arguments and semantics from
    /// [ExtAwi::from_bytes_radix], this function includes the ability to deal
    /// with general fixed point integer deserialization. `src` is now split
    /// into separate `integer` and `fraction` parts. An exponent `exp` further
    /// multiplies the numerical value by `radix^exp`. `fp` is the location
    /// of the fixed point in the output representation of the numerical
    /// value (e.x. for a plain integer `fp == 0`). `fp` can be negative or
    /// greater than the bitwidth.
    ///
    /// This function uses a single rigorous round-to-even that occurs after
    /// the exponent and fixed point multiplier are applied and before any
    /// numerical information is lost.
    ///
    /// See [crate::FP::to_vec_general] for the inverse of this function.
    ///
    /// # Errors
    ///
    /// See the error conditions of [ExtAwi::from_bytes_radix]. The precision
    /// can now be arbitrarily large (any overflow in the low numerical
    /// significance direction will be rounded), but overflow can still happen
    /// in the more significant direction. Empty strings are interpreted as a
    /// zero value.
    pub fn from_bytes_general(
        sign: Option<bool>,
        integer: &[u8],
        fraction: &[u8],
        exp: isize,
        radix: u8,
        bw: NonZeroUsize,
        fp: isize,
    ) -> Result<ExtAwi, SerdeError> {
        let i_len = integer.len();
        let f_len = fraction.len();
        let exp_sub_f_len = exp
            .checked_sub(isize::try_from(f_len).ok().ok_or(Overflow)?)
            .ok_or(Overflow)?;

        // The problem we encounter is that the only way to do the correct banker's
        // rounding in the general case is to consider the integer part, the entire
        // fractional part, fixed point multiplier, and exponent all at once.
        //
        // `((i_part * 2^fp) + (f_part * 2^fp * radix^f_len)) * radix^exp`
        // <=> `((i * radix^f_len) + f) * r^(exp - f_len) * 2^fp`

        // TODO we can optimize away leading and trailing '0's

        // this width includes space for everything
        let tmp_bw = NonZeroUsize::new(
            // the +1 is for the shift left on `rem` and for possible `quo` increment overflow
            (sign.is_some() as usize)
                .checked_add(bits_upper_bound(
                    i_len
                        .checked_add(f_len)
                        .ok_or(Overflow)?
                        .checked_add(exp_sub_f_len.unsigned_abs())
                        .ok_or(Overflow)?,
                    radix,
                )?)
                .ok_or(Overflow)?
                .checked_add(fp.unsigned_abs())
                .ok_or(Overflow)?
                .checked_add(1)
                .ok_or(Overflow)?,
        )
        .unwrap();
        let mut numerator = if i_len > 0 {
            // note: do not unwrap in case of exhaustion
            let mut i_part = ExtAwi::from_bytes_radix(None, integer, radix, tmp_bw)?;
            // multiply by `radix^f_len` here
            for _ in 0..f_len {
                i_part.const_as_mut().short_cin_mul(0, usize::from(radix));
            }
            i_part
        } else {
            ExtAwi::zero(tmp_bw)
        };
        let num = numerator.const_as_mut();
        if f_len > 0 {
            let mut f_part = ExtAwi::from_bytes_radix(
                None, // avoids overflow corner case
                fraction, radix, tmp_bw,
            )?;
            num.add_assign(f_part.const_as_mut());
        }
        let mut denominator = ExtAwi::uone(tmp_bw);
        let den = denominator.const_as_mut();

        if exp_sub_f_len < 0 {
            for _ in 0..exp_sub_f_len.unsigned_abs() {
                den.short_cin_mul(0, usize::from(radix));
            }
        } else {
            for _ in 0..exp_sub_f_len.unsigned_abs() {
                num.short_cin_mul(0, usize::from(radix));
            }
        }
        if fp < 0 {
            den.shl_assign(fp.unsigned_abs()).unwrap();
        } else {
            num.shl_assign(fp.unsigned_abs()).unwrap();
        }
        let mut quotient = ExtAwi::zero(tmp_bw);
        let quo = quotient.const_as_mut();
        let mut remainder = ExtAwi::zero(tmp_bw);
        let rem = remainder.const_as_mut();
        Bits::udivide(quo, rem, num, den).unwrap();
        // The remainder `rem` is in the range `0..den`. We use banker's rounding to
        // choose when to round up `quo`.
        rem.shl_assign(1);
        if den.ult(rem).unwrap() {
            // past the halfway point, round up
            quo.inc_assign(true);
        } else if den == rem {
            // round to even
            let odd = quo.lsb();
            quo.inc_assign(odd);
        } // else truncated is correct
        if let Some(true) = sign {
            quo.neg_assign(true);
        }

        let mut res = ExtAwi::zero(bw);
        let x = res.const_as_mut();
        if sign.is_none() {
            if x.zero_resize_assign(quo) {
                return Err(Overflow)
            }
        } else if x.sign_resize_assign(quo) {
            return Err(Overflow)
        }
        Ok(res)
    }

    /// Creates an `ExtAwi` representing the given arguments. This does the same
    /// thing as [ExtAwi::from_bytes_general] but with `&str`s.
    pub fn from_str_general(
        sign: Option<bool>,
        integer: &str,
        fraction: &str,
        exp: isize,
        radix: u8,
        bw: NonZeroUsize,
        fp: isize,
    ) -> Result<ExtAwi, SerdeError> {
        ExtAwi::from_bytes_general(
            sign,
            integer.as_bytes(),
            fraction.as_bytes(),
            exp,
            radix,
            bw,
            fp,
        )
    }
}

impl core::str::FromStr for ExtAwi {
    type Err = SerdeError;

    // TODO extend this `-0x1234.5678p-3i32f-16`

    /// Creates an `ExtAwi` described by `s`. There are two modes of operation
    /// which use [ExtAwi::from_str_radix] differently.
    ///
    /// In general mode, the bitwidth must be specified after a 'u' (unsigned)
    /// or 'i' (signed) suffix. A prefix of "0b" specifies a binary radix, "0o"
    /// specifies an octal radix, "0x" specifies hexadecimal, else decimal.
    /// For some examples, "42u10" entered into this function creates an
    /// `ExtAwi` with bitwidth 10 and unsigned value 42. "-42i10" results in
    /// bitwidth 10 and signed value of -42. "0xffff_ffffu32" results in
    /// bitwidth 32 and an unsigned value of 0xffffffff (also 4294967295 in
    /// decimal and u32::MAX). "0x1_0000_0000u32" results in an error with
    /// `SerdeError::Overflow`, because it exceeds the maximum unsigned
    /// value for a 32 bit integer. "123" results in
    /// `SerdeError::InvalidChar`, because no bitwidth suffix
    /// has been supplied and this function has assumed binary mode, in which
    /// '2' and '3' are invalid chars.
    ///
    /// If no 'u' or 'i' chars are present, this function will use binary mode
    /// and assume the input is a radix 2 string with only the chars '0' and
    /// '1'. In this mode, the bitwidth will be equal to the number of
    /// chars, including leading zeros. For some examples, 42 in binary is
    /// 101010. If "101010" is entered into this function, it will return an
    /// `ExtAwi` with bitwidth 6 and unsigned value 42. "0000101010" results
    /// in bitwidth 10 and unsigned value 42. "1111_1111" results in bitwidth
    /// 8 and signed value -128 or equivalently unsigned value 255.
    ///
    /// A missing significand or suffix will result in `SerdeError::Empty`. Even
    /// if the value is zero, there must be at least one '0' char in the
    /// significand (e.x. `0x0u8` not `0xu8`), otherwise `SerdeError::Empty` is
    /// returned.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.is_ascii() {
            return Err(InvalidChar)
        }
        let s = s.as_bytes();
        if s.is_empty() {
            return Err(Empty)
        }

        // there should only be one 'u' or 'i' or none in the case of a binary string
        let iu = s.iter().position(|c| *c == b'u');
        let ii = s.iter().position(|c| *c == b'i');
        let (signed, i) = match (iu, ii) {
            (Some(i), None) => (false, i),
            (None, Some(i)) => (true, i),
            (None, None) => {
                // binary case

                // do not count `_` for the bitwidth
                let mut bw = 0;
                for c in s {
                    if *c != b'_' {
                        bw += 1;
                    }
                }
                if bw == 0 {
                    return Err(Empty)
                }
                return ExtAwi::from_bytes_radix(None, s, 2, NonZeroUsize::new(bw).unwrap())
            }
            _ => return Err(Empty),
        };

        // find bitwidth
        let bw = if i.checked_add(1).ok_or(Overflow)? < s.len() {
            match String::from_utf8(Vec::from(&s[i.checked_add(1).ok_or(Overflow)?..]))
                .unwrap()
                .parse::<usize>()
            {
                Ok(bw) => bw,
                Err(_) => return Err(InvalidChar),
            }
        } else {
            return Err(Empty)
        };

        // find sign
        let (src, sign) = if signed {
            if s[0] == b'-' {
                (&s[1..i], Some(true))
            } else {
                (&s[..i], Some(false))
            }
        } else {
            (&s[..i], None)
        };
        if src.is_empty() {
            return Err(Empty)
        }

        // find radix
        let (src, radix) = if src.len() >= 2 {
            match (src[0], src[1]) {
                (b'0', b'x') => (&src[2..], 16),
                (b'0', b'o') => (&src[2..], 8),
                (b'0', b'b') => (&src[2..], 2),
                _ => (src, 10),
            }
        } else {
            (src, 10)
        };
        if src.is_empty() {
            return Err(Empty)
        }

        match NonZeroUsize::new(bw) {
            None => Err(ZeroBitwidth),
            Some(bw) => ExtAwi::from_bytes_radix(sign, src, radix, bw),
        }
    }
}
