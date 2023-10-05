use alloc::{string::String, vec::Vec};
use core::{cmp, num::NonZeroUsize, ops::DerefMut};

use awint_core::{Bits, InlAwi};

use crate::{
    awint_internals::{SerdeError::*, *},
    Awi,
};

// Note: we are not making these free functions at least until some allocator
// trait is stabilized and we can parameterize them, the non free functions will
// use the associated allocator and avoid extra imports

pub(crate) fn bits_to_vec_radix(
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
    let mut pad = Awi::zero(bits.nzbw());
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

pub(crate) fn bits_to_string_radix(
    bits: &Bits,
    signed: bool,
    radix: u8,
    upper: bool,
    min_chars: usize,
) -> Result<String, SerdeError> {
    let v = bits_to_vec_radix(bits, signed, radix, upper, min_chars)?;
    // Safety: It is impossible for the `from_utf8` conversion to panic because
    // `to_vec_radix` sets all chars to valid utf8
    unsafe { Ok(String::from_utf8_unchecked(v)) }
}

pub(crate) fn internal_from_bytes_radix(
    bits: &mut Bits,
    sign: Option<bool>,
    src: &[u8],
    radix: u8,
) -> Result<(), SerdeError> {
    let tmp_bw = crate::awint_internals::bw(
        (sign.is_some() as usize)
            .checked_add(bits_upper_bound(src.len(), radix)?)
            .ok_or(Overflow)?,
    );
    let mut val = Awi::zero(tmp_bw);
    let mut pad0 = Awi::zero(tmp_bw);
    let mut pad1 = Awi::zero(tmp_bw);

    let tmp = val.const_as_mut();
    // note: do not unwrap in case of exhaustion
    tmp.bytes_radix_(sign, src, radix, pad0.const_as_mut(), pad1.const_as_mut())?;

    if sign.is_none() {
        if bits.zero_resize_(tmp) {
            return Err(Overflow)
        }
    } else if bits.sign_resize_(tmp) {
        return Err(Overflow)
    }
    Ok(())
}

// note: these functions are not under `FP` because `FP` is a generic struct
// agnostic to `ExtAwi` or `Awi`

pub(crate) fn internal_from_bytes_general(
    bits: &mut Bits,
    sign: Option<bool>,
    integer: &[u8],
    fraction: &[u8],
    exp: isize,
    radix: u8,
    fp: isize,
) -> Result<(), SerdeError> {
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
        let mut i_part = Awi::from_bytes_radix(None, integer, radix, tmp_bw)?;
        // multiply by `radix^f_len` here
        for _ in 0..f_len {
            i_part.const_as_mut().digit_cin_mul_(0, Digit::from(radix));
        }
        i_part
    } else {
        Awi::zero(tmp_bw)
    };
    let num = numerator.const_as_mut();
    if f_len > 0 {
        let mut f_part = Awi::from_bytes_radix(
            None, // avoids overflow corner case
            fraction, radix, tmp_bw,
        )?;
        num.add_(f_part.const_as_mut()).unwrap();
    }
    let mut denominator = Awi::uone(tmp_bw);
    let den = denominator.const_as_mut();

    if exp_sub_f_len < 0 {
        for _ in 0..exp_sub_f_len.unsigned_abs() {
            den.digit_cin_mul_(0, Digit::from(radix));
        }
    } else {
        for _ in 0..exp_sub_f_len.unsigned_abs() {
            num.digit_cin_mul_(0, Digit::from(radix));
        }
    }
    if fp < 0 {
        den.shl_(fp.unsigned_abs()).unwrap();
    } else {
        num.shl_(fp.unsigned_abs()).unwrap();
    }
    let mut quotient = Awi::zero(tmp_bw);
    let quo = quotient.const_as_mut();
    let mut remainder = Awi::zero(tmp_bw);
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

    if sign.is_none() {
        if bits.zero_resize_(quo) {
            return Err(Overflow)
        }
    } else if bits.sign_resize_(quo) {
        return Err(Overflow)
    }
    Ok(())
}

// TODO 0e-3_n123.456_i32f16 0xp-3_n123.456_i32f16 allow leading 'n'

// TODO leading 'r' for reversimals

// TODO default 0. shift for fixing very large fp problem, and fix perf

#[inline]
pub(crate) fn internal_from_str<O: DerefMut<Target = Bits>, F: FnMut(NonZeroUsize) -> O>(
    s: &str,
    mut f: F,
) -> Result<O, SerdeError> {
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
                let mut res = f(w);
                internal_from_bytes_radix(&mut res, None, s, 2)?;
                return Ok(res)
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

                let mut res = f(w);
                internal_from_bytes_general(&mut res, sign, integer, fraction, exp, radix, fp)?;
                Ok(res)
            } else {
                // integer mode

                if (exp < 0) || (fraction.is_some()) {
                    Err(Fractional)
                } else if exp > 0 {
                    // there are a lot of tricky edge cases, just use this
                    let mut res = f(w);
                    internal_from_bytes_general(&mut res, sign, integer, &[], exp, radix, 0)?;
                    Ok(res)
                } else {
                    let mut res = f(w);
                    internal_from_bytes_radix(&mut res, sign, integer, radix)?;
                    Ok(res)
                }
            }
        } else {
            Err(EmptyInteger)
        }
    } else {
        Err(EmptyBitwidth)
    }
}
