use alloc::{string::String, vec::Vec};
use core::num::NonZeroUsize;

use awint_core::{Bits, SerdeError};

use crate::{
    string_internals::{
        bits_to_string_radix, bits_to_vec_radix, internal_from_bytes_general,
        internal_from_bytes_radix, internal_from_str,
    },
    Awi,
};

/// # non-`const` string representation conversion
impl Awi {
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
        bits_to_vec_radix(bits, signed, radix, upper, min_chars)
    }

    /// Creates a string representing `bits`. This function performs allocation.
    /// This does the same thing as [Awi::bits_to_vec_radix] but with a
    /// `String`.
    pub fn bits_to_string_radix(
        bits: &Bits,
        signed: bool,
        radix: u8,
        upper: bool,
        min_chars: usize,
    ) -> Result<String, SerdeError> {
        bits_to_string_radix(bits, signed, radix, upper, min_chars)
    }

    /// Creates an `Awi` representing the given arguments. This function
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
    ) -> Result<Awi, SerdeError> {
        let mut res = Awi::zero(bw);
        internal_from_bytes_radix(&mut res, sign, src, radix)?;
        Ok(res)
    }

    /// Creates an `Awi` representing the given arguments. This does the same
    /// thing as [Awi::from_bytes_radix] but with an `&str`.
    pub fn from_str_radix(
        sign: Option<bool>,
        str: &str,
        radix: u8,
        bw: NonZeroUsize,
    ) -> Result<Awi, SerdeError> {
        let mut res = Awi::zero(bw);
        internal_from_bytes_radix(&mut res, sign, str.as_bytes(), radix)?;
        Ok(res)
    }

    /// Creates an `Awi` representing the given arguments. This function
    /// performs allocation. In addition to the arguments and semantics from
    /// [Awi::from_bytes_radix], this function includes the ability to deal
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
    /// See the error conditions of [Awi::from_bytes_radix]. The precision
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
    ) -> Result<Awi, SerdeError> {
        let mut res = Awi::zero(bw);
        internal_from_bytes_general(&mut res, sign, integer, fraction, exp, radix, fp)?;
        Ok(res)
    }

    /// Creates an `Awi` representing the given arguments. This does the same
    /// thing as [Awi::from_bytes_general] but with `&str`s.
    pub fn from_str_general(
        sign: Option<bool>,
        integer: &str,
        fraction: &str,
        exp: isize,
        radix: u8,
        bw: NonZeroUsize,
        fp: isize,
    ) -> Result<Awi, SerdeError> {
        let mut res = Awi::zero(bw);
        internal_from_bytes_general(
            &mut res,
            sign,
            integer.as_bytes(),
            fraction.as_bytes(),
            exp,
            radix,
            fp,
        )?;
        Ok(res)
    }
}

impl core::str::FromStr for Awi {
    type Err = SerdeError;

    /// Creates an `Awi` described by `s`. There are three modes of operation
    /// which invoke [Awi::from_str_radix] or [Awi::from_str_general]
    /// differently.
    ///
    /// Note: there is currently a
    /// [bug](https://github.com/rust-lang/rust/issues/108385) in Rust that
    /// causes certain fixed point literals to fail to parse when attempting
    /// to use them in the concatenation macros. In case of getting
    /// "literal is not supported" errors, use `Awi::from_str` directly.
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
    /// 0x10^-0x3 and uses [Awi::from_bytes_general] to round-to-even to a 32
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
        internal_from_str(s, |w| Awi::zero(w))
    }
}
