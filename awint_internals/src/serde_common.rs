//! Common serialization utilities

use core::fmt;

use crate::USIZE_BITS;

// The reason this is here is because I need the free functions in `awint_core`
// for speeding up certain serialization tasks, but the free functions also need
// `SerdeError` to make them more ergonomic in `awint_ext`.

/// A serialization or deserialization error
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SerdeError {
    /// The input is empty
    Empty,
    /// The input is missing the integer part, even if it has a bitwidth or
    /// other part.
    EmptyInteger,
    /// A fraction is given but it is empty
    EmptyFraction,
    /// An exponent suffix is given but it is empty
    EmptyExponent,
    /// A bitwidth suffix is given but it is empty
    EmptyBitwidth,
    /// A fixed point suffix is given but it is empty
    EmptyFixedPoint,
    /// There is an unrecognized character that is not `_`, `-`, `0..=9`,
    /// `a..=z`, or `A..=Z` depending on the radix and other context
    InvalidChar,
    /// A radix is not in the range `2..=36`
    InvalidRadix,
    /// If an input bitwidth is zero
    ZeroBitwidth,
    /// If some kind of width does not match in contexts that require equal
    /// widths
    NonEqualWidths,
    /// An input was marked as both negative and unsigned
    NegativeUnsigned,
    /// If a fraction or negative exponent was used without fixed point mode
    Fractional,
    /// The value represented by the string cannot fit in the specified unsigned
    /// or signed integer. This may also be thrown in case of internal
    /// algorithms failing from extreme string lengths approaching memory
    /// exhaustion.
    Overflow,
}

impl fmt::Display for SerdeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

use SerdeError::*;

/// Binary logarithms of the integers 2..=36 rounded up and in u16p13 fixed
/// point format
pub const LB_I3F13: [u16; 37] = [
    0, 0, 8192, 12985, 16384, 19022, 21177, 22998, 24576, 25969, 27214, 28340, 29369, 30315, 31190,
    32006, 32768, 33485, 34161, 34800, 35406, 35982, 36532, 37058, 37561, 38043, 38507, 38953,
    39382, 39797, 40198, 40585, 40960, 41324, 41677, 42020, 42353,
];

#[test]
fn lb_u16p13() {
    use core::ops::Mul;
    for i in 2..=36 {
        assert_eq!(
            LB_I3F13[i],
            (i as f64).log2().mul((1 << 13) as f64).ceil() as u16
        )
    }
}

/// Reciprocal binary logarithms of the numbers 2..=36 rounded up and in u16p15
/// fixed point format
pub const INV_LB_I1F15: [u16; 37] = [
    0, 0, 32768, 20675, 16384, 14113, 12677, 11673, 10923, 10338, 9865, 9473, 9141, 8856, 8607,
    8388, 8192, 8017, 7859, 7714, 7582, 7461, 7349, 7244, 7147, 7057, 6972, 6892, 6817, 6746, 6678,
    6615, 6554, 6496, 6441, 6389, 6339,
];

#[test]
fn inv_lb_u16p15() {
    use core::ops::Mul;
    for i in 2..=36 {
        assert_eq!(
            INV_LB_I1F15[i],
            (i as f64).log2().powi(-1).mul((1 << 15) as f64).ceil() as u16
        )
    }
}

/// This is used for quickly calculating the maximum number of bits
/// needed for a string representation of a number in some radix to be
/// represented. This may give more bits than needed, but is guaranteed to never
/// underestimate the number of bits needed.
/// Returns `None` if we see memory exhaustion
pub const fn bits_upper_bound(len: usize, radix: u8) -> Result<usize, SerdeError> {
    if radix < 2 || radix > 36 {
        return Err(InvalidRadix)
    }

    // For example, when the radix 10 string "123456789" is going to be converted to
    // an awint, `LB_I3F13` is indexed by 10 which gives 27214. 27214 multiplied
    // by the length of the string plus 1 is 27214 * (9 + 1) = 272140. This is
    // shifted right by 13 and added with 1 which produces 34. The actual number
    // of bits needed is 27. The increments are needed to guard against edge
    // cases such as all digits being the maximum for the given radix (e.g.
    // "999999999" which needs 30 bits). The relative overshoot seems quite
    // large in this example but improves for larger strings and odd bases.

    // The multiplication is checked as an extreme safeguard.
    if let Some(tmp) = (LB_I3F13[radix as usize] as u128).checked_mul((len as u128).wrapping_add(1))
    {
        // `len` should not be larger than `isize::MAX`.
        let estimate = (tmp >> 13).wrapping_add(1);
        if (estimate & (!((1u128 << (USIZE_BITS - 1)) - 1))) == 0 {
            return Ok(estimate as usize)
        }
    }
    Err(Overflow)
}

/// This takes an input of significant bits and gives an upper bound for the
/// number of characters in the given `radix` needed to represent those bits.
pub const fn chars_upper_bound(significant_bits: usize, radix: u8) -> Result<usize, SerdeError> {
    if radix < 2 || radix > 36 {
        return Err(InvalidRadix)
    }

    if let Some(tmp) = (INV_LB_I1F15[radix as usize] as u128)
        .checked_mul((significant_bits as u128).wrapping_add(1))
    {
        let estimate = (tmp >> 15).wrapping_add(1);
        // check that it would fit within `isize::MAX`
        if (estimate & (!((1u128 << (USIZE_BITS - 1)) - 1))) == 0 {
            return Ok(estimate as usize)
        }
    }
    Err(Overflow)
}

/// The same as [bits_upper_bound](crate::bits_upper_bound) except it panics
/// internally in case of overflow
pub const fn panicking_bits_upper_bound(len: usize, radix: u8) -> usize {
    match bits_upper_bound(len, radix) {
        Ok(o) => o,
        // TODO can't const panic with format strings yet
        Err(_e) => panic!(),
    }
}

/// The same as [chars_upper_bound](crate::chars_upper_bound) except it panics
/// internally in case of overflow
pub const fn panicking_chars_upper_bound(significant_bits: usize, radix: u8) -> usize {
    match chars_upper_bound(significant_bits, radix) {
        Ok(o) => o,
        // TODO can't const panic with format strings yet
        Err(_e) => panic!(),
    }
}
