use alloc::{string::String, vec::Vec};
use core::{cmp, num::NonZeroUsize};

use awint_core::Bits;
use awint_internals::{SerdeError::*, *};

use crate::ExtAwi;

/// # non-`const` string representation conversion
impl ExtAwi {
    /// Creates a `Vec<u8>` representing `bits`. This function performs
    /// allocation. This is a wrapper around [awint_core::Bits::to_bytes_radix]
    /// that truncates leading zeros and possibly adds `-` as a sign
    /// indicator. An additional `minimum_chars` specifies the minimum
    /// number of characters that should exist. If the sign indicator plus
    /// significand length is less than `minimum_chars`, zeros will be
    /// filled between the sign indicator and significand, just like Rust's
    /// built in `{:0d}` formatting.
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
        minimum_chars: usize,
    ) -> Result<Vec<u8>, SerdeError> {
        let needs_indicator = signed && bits.msb();
        let mut dst = alloc::vec![0;
            cmp::max(
                minimum_chars,
                (needs_indicator as usize)
                    + chars_upper_bound(bits.bw() - bits.lz(), radix)?
            )
        ];
        let mut pad = ExtAwi::zero(bits.nzbw());
        bits.to_bytes_radix(signed, &mut dst[..], radix, upper, pad.const_as_mut())?;
        let len = dst.len();
        for i in 0..len {
            if dst[i] != b'0' {
                // most significant digit
                let msd;
                // exclude sign indicator
                if needs_indicator {
                    msd = i - 1;
                    dst[msd] = b'-';
                } else {
                    msd = i;
                }
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
            if i == len - minimum_chars {
                // terminate early to keep the minimum number of chars
                if needs_indicator {
                    // cannot overwrite a nonzero digit because we added `needs_indicator` to
                    // `dst`'s length earlier
                    dst[i] = b'-';
                }
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
                // all zeros, remove all but one zero
                for _ in 0..(len - 1) {
                    dst.pop();
                }
                dst.shrink_to_fit();
                break
            }
        }
        Ok(dst)
    }

    /// Creates a string representing `bits`. This function performs allocation.
    /// This does the same thing as [ExtAwi::bits_to_vec_radix] except with a
    /// `String`.
    pub fn bits_to_string_radix(
        bits: &Bits,
        signed: bool,
        radix: u8,
        upper: bool,
        minimum_chars: usize,
    ) -> Result<String, SerdeError> {
        // It is impossible for the `from_utf8` conversion to panic because
        // `to_vec_radix` sets all chars to valid utf8
        Ok(String::from_utf8(ExtAwi::bits_to_vec_radix(
            bits,
            signed,
            radix,
            upper,
            minimum_chars,
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
        let tmp_bw =
            awint_internals::bw((sign.is_some() as usize) + bits_upper_bound(src.len(), radix)?);
        let mut awi = ExtAwi::zero(tmp_bw);
        let mut pad0 = ExtAwi::zero(tmp_bw);
        let mut pad1 = ExtAwi::zero(tmp_bw);

        let tmp = awi.const_as_mut();
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
    /// thing as [ExtAwi::from_bytes_radix] except with an `&str`.
    pub fn from_str_radix(
        sign: Option<bool>,
        str: &str,
        radix: u8,
        bw: NonZeroUsize,
    ) -> Result<ExtAwi, SerdeError> {
        ExtAwi::from_bytes_radix(sign, str.as_bytes(), radix, bw)
    }
}

impl core::str::FromStr for ExtAwi {
    type Err = SerdeError;

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
        let bw = if (i + 1) < s.len() {
            match String::from_utf8(Vec::from(&s[(i + 1)..]))
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

        match NonZeroUsize::new(bw) {
            None => Err(ZeroBitwidth),
            Some(bw) => ExtAwi::from_bytes_radix(sign, src, radix, bw),
        }
    }
}
