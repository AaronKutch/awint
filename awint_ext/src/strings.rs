use alloc::{string::String, vec::Vec};
use core::{cmp, num::NonZeroUsize};

use awint_core::{Bits, InlAwi};

use crate::{
    awint_internals::{SerdeError::*, *},
    ExtAwi,
};

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
    /// [awint_core::Bits::bytes_radix_] that zero or sign resizes the
    /// result to match `bw`.
    ///
    /// # Errors
    ///
    /// See the error conditions of [Bits::bytes_radix_]. Note that `-` is
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
        let tmp_bw = crate::awint_internals::bw(
            (sign.is_some() as usize)
                .checked_add(bits_upper_bound(src.len(), radix)?)
                .ok_or(Overflow)?,
        );
        let mut awi = ExtAwi::zero(tmp_bw);
        let mut pad0 = ExtAwi::zero(tmp_bw);
        let mut pad1 = ExtAwi::zero(tmp_bw);

        let tmp = awi.const_as_mut();
        // note: do not unwrap in case of exhaustion
        tmp.bytes_radix_(sign, src, radix, pad0.const_as_mut(), pad1.const_as_mut())?;

        let mut final_awi = ExtAwi::zero(bw);
        let x = final_awi.const_as_mut();
        if sign.is_none() {
            if x.zero_resize_(tmp) {
                return Err(Overflow)
            }
        } else if x.sign_resize_(tmp) {
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
        let mut i_len = 0usize;
        for c in integer {
            if *c != b'_' {
                i_len += 1;
            }
        }
        let mut f_len = 0usize;
        for c in fraction {
            if *c != b'_' {
                f_len += 1;
            }
        }
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
                i_part.const_as_mut().digit_cin_mul(0, Digit::from(radix));
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
            num.add_(f_part.const_as_mut()).unwrap();
        }
        let mut denominator = ExtAwi::uone(tmp_bw);
        let den = denominator.const_as_mut();

        if exp_sub_f_len < 0 {
            for _ in 0..exp_sub_f_len.unsigned_abs() {
                den.digit_cin_mul(0, Digit::from(radix));
            }
        } else {
            for _ in 0..exp_sub_f_len.unsigned_abs() {
                num.digit_cin_mul(0, Digit::from(radix));
            }
        }
        if fp < 0 {
            den.shl_(fp.unsigned_abs()).unwrap();
        } else {
            num.shl_(fp.unsigned_abs()).unwrap();
        }
        let mut quotient = ExtAwi::zero(tmp_bw);
        let quo = quotient.const_as_mut();
        let mut remainder = ExtAwi::zero(tmp_bw);
        let rem = remainder.const_as_mut();
        Bits::udivide(quo, rem, num, den).unwrap();
        // The remainder `rem` is in the range `0..den`. We use banker's rounding to
        // choose when to round up `quo`.
        rem.shl_(1).unwrap();
        if den.ult(rem).unwrap() {
            // past the halfway point, round up
            quo.inc_(true);
        } else if den == rem {
            // round to even
            let odd = quo.lsb();
            quo.inc_(odd);
        } // else truncated is correct
        if let Some(true) = sign {
            quo.neg_(true);
        }

        let mut res = ExtAwi::zero(bw);
        let x = res.const_as_mut();
        if sign.is_none() {
            if x.zero_resize_(quo) {
                return Err(Overflow)
            }
        } else if x.sign_resize_(quo) {
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

// TODO 0e-3_n123.456_i32f16 0xp-3_n123.456_i32f16 allow leading 'n'

// TODO leading 'r' for reversimals

// TODO default 0. shift for fixing very large fp problem, and fix perf

impl core::str::FromStr for ExtAwi {
    type Err = SerdeError;

    /// Creates an `ExtAwi` described by `s`. There are three modes of operation
    /// which invoke [ExtAwi::from_str_radix] or [ExtAwi::from_str_general]
    /// differently.
    ///
    /// Note: there is currently a
    /// [bug](https://github.com/rust-lang/rust/issues/108385) in Rust that
    /// causes certain fixed point literals to fail to parse when attempting
    /// to use them in the concatenation macros. In case of getting
    /// "literal is not supported" errors, use `ExtAwi::from_str` directly.
    ///
    /// Additionally, note that it is easy to cause resource exhaustion with
    /// large bitwidths, exponents, or fixed points that can approach
    /// `usize::MAX`. In a future version of `awint` we should have a guarded
    /// function for helping with entering literals through things like UIs.
    ///
    /// All valid inputs must begin with '0'-'9' or a '-' followed by '0'-'9'.
    ///
    /// If only ' _ ', '0', and '1' chars are present, this function uses binary
    /// mode. It will interpret the input as a binary string, the number of '0's
    /// and '1's of which is the bitwidth (including leading '0's and excluding
    /// ' _ 's). For example: 42 in binary is 101010. If "101010" is entered
    /// into this function, it will return an `ExtAwi` with bitwidth 6 and
    /// unsigned value 42. "0000101010" results in bitwidth 10 and unsigned
    /// value 42. "1111_1111" results in bitwidth 8 and signed value -128 or
    /// equivalently unsigned value 255.
    ///
    /// In integer mode, a decimal bitwidth must be specified after a 'u'
    /// (unsigned) or 'i' (signed) suffix. A prefix of "0b" specifies a binary
    /// radix, "0o" specifies an octal radix, "0x" specifies hexadecimal,
    /// otherwise a decimal radix is used. For example: "42u10" entered into
    /// this function creates an `ExtAwi` with bitwidth 10 and unsigned
    /// value 42. "-42i10" results in bitwidth 10 and signed value of -42.
    /// "0xffff_ffffu32" results in bitwidth 32 and an unsigned value of
    /// 0xffffffff (also 4294967295 in decimal and u32::MAX).
    /// "0x1_0000_0000u32" results in an error with `SerdeError::Overflow`,
    /// because it exceeds the maximum unsigned value for a 32 bit integer.
    /// "123" results in `SerdeError::EmptyBitwidth`, because it is not in
    /// binary mode and no bitwidth suffix has been supplied.
    ///
    /// If, after the bitwidth, an 'f' char is present, fixed point mode is
    /// activated. A decimal fixed point position must be specified after the
    /// 'f' that tells where the fixed point will be in the resulting bits (see
    /// [crate::FP] for more). If the most significant numerical bit would be
    /// cut off, `SerdeError::Overflow` is returned.
    ///
    /// Additionally, an exponent char 'e' (for non-hexadecimal radixes only) or
    /// 'p' can be included after the integer or fraction parts but before the
    /// bitwidth suffix. The exponent as typed uses the radix of the integer
    /// part, and it is raised to the same radix when modifying the numerical
    /// value. The exponent can only be negative for fixed point mode. For
    /// example: "123e5u32" has numerical value 12300000. "123e-5u32" returns an
    /// error since it is trying to use a negative exponent in integer mode.
    /// "-0x1234.5678p-3i32f16" has a numerical value of -0x1234.5678 *
    /// 0x10^-0x3 and uses [ExtAwi::from_bytes_general] to round-to-even to a 32
    /// bit fixed point number with fixed point position 16. You probably want
    /// to use underscores to make it clearer where different parts are, e.x.
    /// "-0x1234.5678_p-3_i32_f16".
    ///
    /// For all parts including the integer, fraction, exponent, bitwidth, and
    /// fixed point parts, if their prefix char exists but there is not at least
    /// one '0' for them, some kind of empty error is returned. For example:
    /// "0xu8" should be "0x0u8". ".i8f0" should be "0.0i8f0". "1u32f" should be
    /// "1u32f0".
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut sign = None;
        let mut integer = None;
        let mut fraction = None;
        let mut exp = None;
        let mut exp_negative = false;
        let mut radix = None;
        let bitwidth;
        let mut fp = None;
        let mut fp_negative = false;

        let is_integral = |c: u8, radix: Option<u8>| {
            let is_underscore = c == b'_';
            let is_binary = (b'0' <= c) && (c <= b'1');
            let is_octal = (b'0' <= c) && (c <= b'7');
            let is_decimal = (b'0' <= c) && (c <= b'9');
            let is_lowerhex = (b'a' <= c) && (c <= b'f');
            let is_upperhex = (b'A' <= c) && (c <= b'F');
            match radix {
                // assuming binary or decimal
                None => is_underscore || is_decimal,
                Some(2) => is_underscore || is_binary,
                Some(8) => is_underscore || is_octal,
                Some(16) => is_underscore || is_decimal || is_lowerhex || is_upperhex,
                _ => unreachable!(),
            }
        };

        let is_empty_or_all_underscores = |s: &[u8]| {
            let mut all_underscores = true;
            for c in s {
                if *c != b'_' {
                    all_underscores = false;
                    break
                }
            }
            all_underscores
        };

        let s = s.as_bytes();
        if s.is_empty() {
            return Err(Empty)
        }

        let mut i = 0;
        if s[i] == b'-' {
            if s.len() < 2 {
                return Err(Empty)
            }
            sign = Some(true);
            i += 1;
        }
        if (s[i] == b'u') || (s[i] == b'i') {
            // case that we want a better error for
            return Err(EmptyInteger)
        }
        // first char after a possible '-' should always be '0'-'9'
        if !((b'0' <= s[i]) && (s[i] <= b'9')) {
            return Err(InvalidChar)
        }

        if (s[i] == b'0') && ((i + 1) < s.len()) {
            if s[i + 1] == b'b' {
                radix = Some(2);
                i += 2;
            } else if s[i + 1] == b'o' {
                i += 2;
                radix = Some(8);
            } else if s[i + 1] == b'x' {
                radix = Some(16);
                i += 2;
            }
            // else it might be binary mode or decimal radix
        }

        if sign.is_none() && radix.is_none() {
            // check for binary mode, we have prefix checks above and reverse iteration here
            // to reduce checking time
            let mut binary_mode = true;
            let mut w = 0;
            for c in s.iter().rev() {
                let c = *c;
                if !((c == b'_') || (c == b'0') || (c == b'1')) {
                    binary_mode = false;
                    break
                }
                if c != b'_' {
                    w += 1;
                }
            }
            if binary_mode {
                if let Some(w) = NonZeroUsize::new(w) {
                    return ExtAwi::from_bytes_radix(None, s, 2, w)
                } else {
                    // there was '_' only
                    return Err(EmptyInteger)
                }
            }
        }

        // integer part, can be followed by '.' for fraction, 'e' or 'p' for exponent,
        // or 'u' or 'i' for bitwidth
        let integer_start = i;
        let mut fraction_start = None;
        let mut exp_start = None;
        loop {
            if i >= s.len() {
                break
            }
            if !is_integral(s[i], radix) {
                if s[i] == b'.' {
                    fraction_start = Some(i + 1);
                } else if s[i] == b'u' {
                    if sign.is_some() {
                        return Err(NegativeUnsigned)
                    }
                    sign = None;
                } else if s[i] == b'i' {
                    if sign.is_none() {
                        sign = Some(false);
                    }
                } else if (s[i] == b'e') || (s[i] == b'p') {
                    exp_start = Some(i + 1);
                } else {
                    return Err(InvalidChar)
                }
                integer = Some(&s[integer_start..i]);
                i += 1;
                break
            }
            i += 1;
        }

        // fraction part, can be followed by ' or 'p' for exponent, or 'u' or 'i' for
        // bitwidth
        if let Some(fraction_start) = fraction_start {
            loop {
                if i >= s.len() {
                    break
                }
                if !is_integral(s[i], radix) {
                    if s[i] == b'u' {
                        if sign.is_some() {
                            return Err(NegativeUnsigned)
                        }
                        sign = None;
                    } else if s[i] == b'i' {
                        if sign.is_none() {
                            sign = Some(false);
                        }
                    } else if (s[i] == b'e') || (s[i] == b'p') {
                        exp_start = Some(i + 1);
                    } else {
                        return Err(InvalidChar)
                    }
                    fraction = Some(&s[fraction_start..i]);
                    i += 1;
                    break
                }
                i += 1;
            }
        }

        // exponent part, can be followed by 'u' or 'i' for bitwidth
        if let Some(mut exp_start) = exp_start {
            loop {
                if i >= s.len() {
                    break
                }
                if !is_integral(s[i], radix) {
                    if s[i] == b'-' {
                        if exp_negative {
                            return Err(InvalidChar)
                        }
                        exp_negative = true;
                        exp_start += 1;
                        i += 1;
                        continue
                    } else if s[i] == b'u' {
                        if sign.is_some() {
                            return Err(NegativeUnsigned)
                        }
                        sign = None;
                    } else if s[i] == b'i' {
                        if sign.is_none() {
                            sign = Some(false);
                        }
                    } else {
                        return Err(InvalidChar)
                    }
                    exp = Some(&s[exp_start..i]);
                    i += 1;
                    break
                }
                i += 1;
            }
        }

        // bitwidth part, can be followed by 'f' for fixed point
        let bitwidth_start = i;
        let mut fp_start = None;
        loop {
            if i >= s.len() {
                bitwidth = Some(&s[bitwidth_start..i]);
                break
            }
            if !is_integral(s[i], None) {
                if s[i] == b'f' {
                    fp_start = Some(i + 1);
                } else {
                    return Err(InvalidChar)
                }
                bitwidth = Some(&s[bitwidth_start..i]);
                i += 1;
                break
            }
            i += 1;
        }

        // fixed point part
        if let Some(mut fp_start) = fp_start {
            loop {
                if i >= s.len() {
                    fp = Some(&s[fp_start..i]);
                    break
                }
                if !is_integral(s[i], None) {
                    if s[i] == b'-' {
                        if fp_negative {
                            return Err(InvalidChar)
                        }
                        fp_negative = true;
                        fp_start += 1;
                        i += 1;
                        continue
                    } else {
                        return Err(InvalidChar)
                    }
                }
                i += 1;
            }
        }

        let radix = radix.unwrap_or(10);
        if let Some(bitwidth) = bitwidth {
            if is_empty_or_all_underscores(bitwidth) {
                return Err(EmptyBitwidth)
            }
            let pad0 = &mut InlAwi::from_usize(0);
            let pad1 = &mut InlAwi::from_usize(0);
            let mut usize_awi = InlAwi::from_usize(0);
            usize_awi.bytes_radix_(None, bitwidth, 10, pad0, pad1)?;
            let w = if let Some(w) = NonZeroUsize::new(usize_awi.to_usize()) {
                w
            } else {
                return Err(ZeroBitwidth)
            };
            if let Some(integer) = integer {
                if is_empty_or_all_underscores(integer) {
                    return Err(EmptyInteger)
                }
                let exp = if let Some(exp) = exp {
                    if is_empty_or_all_underscores(exp) {
                        return Err(EmptyExponent)
                    }
                    usize_awi.bytes_radix_(Some(exp_negative), exp, radix, pad0, pad1)?;
                    usize_awi.to_isize()
                } else {
                    0
                };
                if let Some(fp) = fp {
                    if is_empty_or_all_underscores(fp) {
                        return Err(EmptyFixedPoint)
                    }
                    // fixed point mode

                    usize_awi.bytes_radix_(Some(fp_negative), fp, 10, pad0, pad1)?;
                    let fp = usize_awi.to_isize();
                    let fraction = if let Some(fraction) = fraction {
                        if is_empty_or_all_underscores(fraction) {
                            return Err(EmptyFraction)
                        }
                        fraction
                    } else {
                        &[]
                    };

                    ExtAwi::from_bytes_general(sign, integer, fraction, exp, radix, w, fp)
                } else {
                    // integer mode

                    if (exp < 0) || (fraction.is_some()) {
                        return Err(Fractional)
                    }
                    if exp > 0 {
                        // there are a lot of tricky edge cases, just use this
                        ExtAwi::from_bytes_general(sign, integer, &[], exp, radix, w, 0)
                    } else {
                        ExtAwi::from_bytes_radix(sign, integer, radix, w)
                    }
                }
            } else {
                Err(EmptyInteger)
            }
        } else {
            Err(EmptyBitwidth)
        }
    }
}
