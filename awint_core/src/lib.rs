//! Arbitrary width integers library
//!
//! This is the core library of the `awint` system of crates. This crate is
//! strictly `no-std` and `no-alloc`, not even requiring an allocator to be
//! compiled. This crate supplies the `Bits` reference type and the `InlAwi`
//! storage type. This crate is intended to be used through the main `awint`
//! crate, and "no-alloc" mode can be achieved by disabling default features and
//! _not_ enabling the "alloc" feature.
//!
//! Some information on understanding two's complement and overflow is included
//! here
//!
//! # Dealing with overflows and retaining numerical precision in integer arithmetic
//!
//! In this document I will be using a symbolic notation for unsigned and signed
//! two's complement integers that is based on Rust's and this crate's
//! notations. An `N`-bit unsigned integer type is denoted by `uN`. An `N`-bit
//! signed integer type is denoted by `iN`. We would also describe them as
//! having "bitwidth" `N`. For example, a 64 bit signed integer type would be
//! denoted `i64`. The `^` symbol used below will denote exponentiation.
//!
//! Numerical values such as 42 or -1337 must be translated into a form
//! representable on computers, and only some integer types with some minimum
//! bitwidth are capable of representing them. If an integer type can represent
//! them, then we can prefix the values to the type, e.x. `42u8` is an unsigned 8
//! bit integer with numerical value 42. `-1337i64` is a signed 64 bit integer
//! with numerical value -1337. `-1337i8`, however, is something that does not
//! exist, because -1337 surpasses the numerical limits of signed 8 bit integers
//!
//! |Numerical limits of an `N` bit integer|unsigned|signed|
//! |:-:|:-:|:-:|
//! |minimum value, shorthand `MIN`|`0uN`|`{-2^(N-1)}iN`|
//! |maximum value, shorthand `MAX`|`{(2^N) - 1}uN`|`{2^(N-1) - 1}iN`|
//!
//! For the uninitiated, I will explain where asymmetries and some of the
//! overflow corner cases originate. An `N` bit string of binary digits can have
//! `2^N` states. For unsigned integers, we want to map numerical 0 to one of
//! these states. This leads to the maximum unsigned numerical value being
//! `(2^N) - 1` instead of simply `2^N`.
//!
//! For signed integers, we want numerical 0 to be "in the middle" of the number
//! line, but the problem is that there is an even number of states to go
//! around, so it necessitates that for every signed integer type, there is one
//! representable numerical value that does not have a corresponding
//! representable negative numerical value. Because of how two's complement
//! works, the negative side gets the corner case value that I denote `MIN_iN`.
//! It is important to remember that `MIN_iN != -MAX_iN`, and under wrapping
//! arithmetic we get `-MIN_iN == MIN_iN`.
//!
//! For example, here are all the possible values of a 4 bit unsigned integer,
//! with the literal binary string on the left and the numerical value in
//! decimal on the right
//!
//! ```text
//! 0000 | 0
//! 0001 | 1
//! 0010 | 2
//! 0011 | 3
//! 0100 | 4
//! 0101 | 5
//! 0110 | 6
//! 0111 | 7
//! 1000 | 8
//! 1001 | 9
//! 1010 | 10
//! 1011 | 11
//! 1100 | 12
//! 1101 | 13
//! 1110 | 14
//! 1111 | 15 == 2^4 - 1
//! ```
//!
//! Here is the same example but with the signed interpretations of the bits
//!
//! ```text
//! 0000 | 0
//! 0001 | 1
//! 0010 | 2
//! 0011 | 3
//! 0100 | 4
//! 0101 | 5
//! 0110 | 6
//! 0111 | 7 == 2^(4-1) - 1
//! 1000 | -8 == -2^(4-1)
//! 1001 | -7
//! 1010 | -6
//! 1011 | -5
//! 1100 | -4
//! 1101 | -3
//! 1110 | -2
//! 1111 | -1
//! ```
//!
//! The numerical value is negative if and only if the 'msb' (most significant
//! numerical bit) is set. Also note that the value of -1 is always all set
//! bits, and the signed minimum value is always all zeros with one set msb bit.
//! The magic of two's complement is that the same underlying operation on bits
//! results in both signed and unsigned addition. For example, `4 + -7 == -3`
//! corresponds to `0100 + 1001 == 1101` which also corresponds to `4 + 9 == 13`
//! on the unsigned side. When `1101` is added to `1110`, we would get `11011`,
//! but under 4 bit wrapping arithmetic it gets truncated to `1011`, which is
//! overflow for the unsigned case but is the correct `-3 + -2 = -5` for the
//! signed case.
//!
//! ## Overflow Conditions
//!
//! Before diving into the numerical error properties of the common operations
//! on integers, I will first go over overflow conditions. These are important
//! to go over first, because overflows can result in completely broken
//! numerical sensibilities. There are rare cases in which overflows can undo
//! other overflows in an algorithm, or where we intentionally want to overflow,
//! but I will not go over those in this document. This is only concerned with
//! keeping numerical interpretation intact, while `awint` will allow you to do
//! anything including width dependent operations that don't care about integral
//! properties.
//!
//! I am including left and right shifts because of how important in practice
//! they are when multiplying or dividing powers of two. They are much cheaper
//! than their equivalents. Shifting left by a shift amount `s` will multiply by
//! `2^s`, and shifting right will divide by `2^s`. There is one important
//! difference: right shifts round to negative infinity while normal divisions
//! round to zero.
//!
//! This table gives the conditions for not overflowing. This assumes that the
//! integers `x` and `y` in binary operations have the same type, and have
//! numerical values `X` and `Y`. The shift amount `s` is some nonnegative
//! integer.
//!
/*
note: comment with /* */ when and remove //! when
formatting the rest of this document
*/
//! |Overflowable Operation|unsigned|signed|
//! |:-:|:-:|:-:|
//! |Negation or Absolute Value (`x.neg_(...)` or `x.abs_()`)|depends|`X != MIN` or switches interpretation|
//! |Addition (`x.add_(y)` and others)|`X + X <= MAX`|`MIN <= X + Y <= MAX`|
//! |Multiplication)|`X * Y <= MAX`|`MIN <= X * Y <= MAX`|
//! |Quotient or  Remainder (`Bits::{u/i}divide`)|`Y != 0`|`(X != MIN or Y != -1) && Y != 0`|
//! |Left Shift (`x.shl_(s)`)|`x * (2^s) <= MAX`|`MIN <= X * (2^s) <= MAX`|
//! |Right Shift|use `Bits::lshr_`|use `Bits::ashr_`|
//!
//! Most of these are simply keeping within `MIN` and `MAX` as expected, but
//! there are a few edges cases. For negation, the `X == MIN_iN` case can be
//! avoided if we simply switch our interpretation of the bits from signed to
//! unsigned (similar to how the standard library has `iN::unsigned_abs -> uN`).
//! For multiplication, the plain `mul_` and `mul_add_` functions work for both
//! signed and unsigned, but some other kinds of multiplication have `u` and `i`
//! variations because they do sign extensions internally.  Divisions have a
//! corner case where the value of `MIN / -1` is unrepresentable. There are two
//! kinds of right shifts, because the sign bit needs to be copied for the
//! signed case. Also note that `awint` forbids shifts of `s >= N`, you may need
//! to conditionally assign special values.
//!
//! The table above gives exact overflow conditions. There are some functions
//! that give overflow information cheaply, however often in practical algorithm
//! design, we don't want to expend resources checking for possible
//! overflow with every operation and instead want only one set of checks at the
//! beginning that prevent the possibility of overflow later. Ideally, we could
//! restrict inputs to fit within a certain integer type (e.x. restrict an
//! input to be representable by u32 even though the input is a u64, so that
//! internal calculations have room to grow the numerical values). We also don't
//! want to be dealing separately with the annoying signed `MIN` corner cases,
//! and expand the set of false positives just enough to deal with them all at
//! once.
//!
//! The table below has entries telling the bitwidth of the base type needed to
//! avoid overflow, given the values `x` and `y` can fit into analogous types
//! with smaller bitwidths `n` and `m`, respectively. If there is a
//! special condition that bitwidth can't guarantee, a conditional that should
//! be true is added.
//!
//! |Operation|unsigned|signed|
//! |:-:|:-:|:-|
//! |Negation or Absolute Value|`n`|`n + 1`|
//! |Addition|`max(n, m) + 1`|`max(n, m) + 1`|
//! |Multiplication|`n + m`|`n + m`|
//! |Quotient or  Remainder|`max(n, m), y != 0`|`max(n, m) + 1, y != 0`|
//! |Left Shift|`n + s`|`n + s`|
//! |Right Shift|`n`|`n`|
//!
//! Note: The extra `+ 1` that
//! some signed operations gain versus their unsigned counterparts can be
//! eliminated if MIN_iN is guarded against. If there are consecutive
//! additions and multiplications, you can often reduce the number of extra bits
//! needed, but you need to do bounds calculations manually using the numerical
//! bounds presented at the start.
//!
//! For example, let's say a type representable in `i16` is being multiplied with
//! another `i16`, an `i1` is added to it, and one final `i15` is divided. Our
//! heuristics say that the first step needs 16 + 16 = 32 bits, the next needs
//! max(32, 1) + 1 == 33 bits, and the last needs max(33, 15) + 1 == 34 bits
//! plus a check that the divisor is not zero. If we have only power-of-two
//! sized primitives, we need to cast all the inputs to `i64` (although the
//! first intermediate could be done in an `i32` before being cast to `i64`).
//!
//! Alternatively, we could be given an `iN` as our output type and work
//! backwards to determine the largest inputs we could have without possibility
//! of overflow.
//!
//! For unsigned values that can virtually be represented as `uN`, the bounds
//! check is simply checking if `x < 2^N`.
//!
//! When making the bounds checks for a signed value to be virtually represented
//! as `iN`, the bounds check is `-2^(N-1) <= x && x < 2^(N-1)`.  An efficient
//! way of doing it that also handles the `MIN_iN` case (which only invalidates
//! one input state and in turn removes the need to add an extra bit for some
//! operations), is to:
//!
//! 1. take the wrapping absolute value of the input (the
//! overflowing absolute value of `MIN_iN` is `MIN_iN`)
//!
//! 2. cast it to a `uN` type so
//! we can use unsigned-less-than (e.x. in Rust primitives it is simply `i64 as
//! u64`, in `awint` we reinterpret)
//!
//! 3. Accept the original input if the cast
//! value is `< 2^(N-1)` (the cast `MIN_iN` value exceeds this as well as the
//! normally unrepresentable values). `Bits::sig` quickly calculates the number
//! of significant bits, such that if `x.sig() == 100` then it means that the
//! unsigned value would fit in 100 bits.
//!
//! `awint::Bits` has several casting operations from the concatenation macros,
//! to `Bits::resize_`, `sign_resize_`, and `zero_resize_`. `awint::Awi` has
//! functions to resize inplace.
//!
//! ## Numerical errors
//!
//! As long as overflow is not occuring, negation, addition, multiplication, and
//! left shift are all perfectly lossless and without error on their part. The
//! divisions (quotient, remainder, right shift) are all lossy in general. The
//! quotient together with the remainder and the divisor, however, can
//! give exact information on what is lost when the quotient calculation is
//! done. For example, dividing a real value of 1000 by 3 would produce
//! 333.3333... . When using integer division, the divisor was 3, the quotient
//! is 333, and the remainder is 1. The real value can be recovered by
//! converting to reals, dividing the remainder by the divisor, and adding it to
//! the quotient: `quotient + remainder/divisor  == 333 + 1/3 == (in the reals
//! domain) 333.333...`.
//!
//! The remainder can be interpreted as the error for an instance of a division.
//! The remainder is bounded by the divisor. In one extreme, a divisor of 1
//! results in no error ever in the quotient. In the other extreme, a divisor
//! larger than the dividend erases all the information that the numerator had
//! and the quotient is always 0.
//!
//! If we have a range of values that a numerator and denominator in a division
//! can take, we can calculate the exclusive upper error bound as a fraction of
//! the numerator by the following: divide the maximum divisor value by the
//! minimum numerator value (as real numbers). For example, if the minimum value
//! the numerator can take is 42 and the maximum value the divisor can take is
//! 7, our upper bound is 7/42 = 0.1666... = 16.67%. Because 7 happens to
//! exactly divide 42, the actual error was zero, but we are getting the upper
//! bound for all possible errors.
//!
//! ## Fixed point representations
//!
//! See the higher level `awint_ext::FP` struct for more. This has more
//! refinements planned and a more extensive set of `floating_...` operations to
//! automatically handle these concerns for the user.
//!
//! If you tried the overflow prevention heuristics above on an algorithm with
//! several multiplications, you may notice that the bitwidth required quickly
//! grows to unmanageable levels, even if you are using `u256`. There comes a
//! point where less significant bits must eventually be cut off. There are also
//! algorithms where you will want to multiply or divide by a noninteger number,
//! e.x. multiply 100 by 3 and divide by 7 to emulate multiplication by 3/7, but
//! the result of 42 cut off the fraction of the real answer.
//! Both these cases will be lossy in general no matter what. However, by
//! designing a custom fixed point representation and adjusting based on the
//! numerical error calculations, the error can be reduced to an acceptable
//! level for the given problem.
//!
//! Imagine that we defined a set of integers that behaved such they had a fixed
//! multiplier attached to them. We will use a power of two, because it allows
//! for cheaper shifts to be used in the implementation details. For example,
//! consider multiplying some input `x` by a fixed multiplier 2^32.
//! At the beginning of the program or function or what have you, the fixed
//! multipliers do not exist in the working memory (i.e. only the plain value of
//! `x` exists at first); the fixed multipliers only appear in intermediate
//! computations. Consider trying to multiply the `x` by a rational number 3/7,
//! but we attach a fixed multiplier. For demonstration I attach it to all the
//! numbers, but usually you only need to attach it to a numerator.
//! Algebraically, we would write:
//!
//! `(x*2^32) * (3*2^32) / (7*2^32)`
//!
//! If possible, I like to write an
//! expression as a product of terms, with terms in a denominator being turned
//! into inverses:
//!
//! `x * 2^32 * 3 * 2^32 * 7^-1 * 2^-32`
//!
//! One multiplier of 2^32 can immediately be annihilated with the 2^-32:
//!
//! `x * 2^32 * 3 * 7^-1`
//!
//! We now need to find an order of multiplications and divisions that leads to
//! the least error. Assuming that we have selected our bitwidths appropriately
//! to avoid overflow, the only source of error will be from a division. The
//! integer error is bounded by the value of the divisor, 7 (so the
//! maximum error can be 6). Looking at the algebra, we move the x, 3, and 2^32
//! around however we want, and we could multiply "fancy ones" like 9/9 to
//! increase the divisor, but there is absolutely no way around having an
//! integer error bound smaller than 7. However, we can decrease the error as a
//! percentage of the smallest numerator by using a fixed point multiplier. In
//! fact, we can get good results even if the smallest x value is 1 (x == 0
//! trivially has an error of 0 in this problem always, so we don't need to
//! worry about it).
//!
//! Usually, in order to favor low level performance details that I will not go
//! into in this text, the best order for a problem like this is to compute in
//! order of: adding, multiplying, multiplying by shifts, and then last we would
//! divide. If there were multiple inverted values besides the `7^-1` in this
//! example, we would group them into a common denominator so that there is only
//! one division to be concerned about.
//!
//! `(x * 3 * 2^32) / 7` (The `* 2^32` can be implemented as a left shift by 32)
//!
//! If we plug in x = 100, and assuming we have chosen the right bitwidths to
//! avoid overflow, we get out an integer value of 184070026971. This value
//! seems random, but if we treat as real and divide by 2^32 to undo the fixed
//! multiplier, we get a value of 42.8571428570 v.s. the true value of
//! 42.8571428571... . This means that our division didn't lose the entire
//! fractional part as it would have if we had used `(x * 3) / 7` instead, but
//! an entire 9 extra digits of precision have been saved hidden inside that
//! seemingly random output. With a fixed multiplier of 2^32, the fractional
//! error upper bound is 7 / (1 * 3 * 2^32) = 5.43*10^-10, and this gets better
//! with larger fixed multiplier or minimum `x`.
//!
//! Let's say that we want to use this output in another function, without
//! dividing out the fixed point factor. We can keep it around to keep
//! precision or as a kind of "leverage" against more division errors. Let's say
//! we are adding two fixed point values together. If they both have the same
//! fixed multiplier, then it is a simple direct addition:
//!
//! `a*2^n + b*2^n = (a + b)*2^n`
//!
//! If they have different fixed multipliers, we have a problem because the
//! semantics will change. The number with the larger fixed multiplier could be
//! divided to have its fixed multiplier equal that of the other part, but
//! because that involves error we preferrably multiply the smaller one to match
//! the larger:
//!
//! `assuming n < m, a has multiplier 2^n, b has multiplier 2^m`
//!
//! `(a*2^n) * (2^(m - n)) + (b*2^m) = (a + b)*2^m` (again, note the change is
//! easily achieved with a shift left of `m - n`)
//!
//! If we are multiplying two fixed point numbers together, we get:
//!
//! `a*2^n * b*2^m = (a * b)*2^(n + m)`
//!
//! To go full circle regarding the bitwidth growing to unmaneagable levels, we
//! can periodically do divisions to bring down the multiplier without impacting
//! the fraction too much. For example, let's say we have two inputs that have a
//! common multiplier of 2^32 and integer values 530239482 (virtually
//! 530239482/2^32 approx. = 0.123456) and 3287505407 (virtually 0.765432), and
//! we multiply them. They will result in the integer 1743165164079879174
//! (virtually 0.094497... with fixed multiplier 2^64). We want to do more
//! operations, but we want to stay below 64 bits. What we can do is divide by
//! 2^32 (shift right by 32), and get integer 405862267 (virtually 0.094497...
//! with a fixed multiplier of 2^32). We traded off bits for precision.
//!
//! Remember to include the bitwidth of the multiplier in overflow calculations
//! when it gets multiplied with something. In the division related sections of
//! overflow table, the bitwidth given is what is needed _during_ the division,
//! but afterwards the bitwidth can be reduced.
//!
//! Bonus point: reciprocals like `x^-1` can be independently processed by
//! treating the implicit 1 as something to attach a fixed multiplier to, e.x. `(1*2^62)
//! / x`. If we are dividing by `x` a lot for instance, we could use one
//! division to calculate a reciprocal. As long as `x` is small compared to
//! the multiplier, we can use multiplications by this reciprocal to do as many
//! quick and accurate divisions as we like. We just need to keep track of the
//! multipliers for post processing, which if powers of two can be done mostly
//! with right shifts.

#![cfg_attr(feature = "const_support", feature(const_maybe_uninit_as_mut_ptr))]
#![cfg_attr(feature = "const_support", feature(const_mut_refs))]
#![cfg_attr(feature = "const_support", feature(const_ptr_read))]
#![cfg_attr(feature = "const_support", feature(const_ptr_write))]
#![cfg_attr(feature = "const_support", feature(const_slice_from_raw_parts_mut))]
#![cfg_attr(feature = "const_support", feature(const_swap))]
#![cfg_attr(feature = "const_support", feature(const_option))]
#![cfg_attr(feature = "const_support", feature(const_trait_impl))]
#![no_std]
// We need to be certain in some places that lifetimes are being elided correctly
#![allow(clippy::needless_lifetimes)]
// There are many guaranteed nonzero lengths
#![allow(clippy::len_without_is_empty)]
// We are using special indexing everywhere
#![allow(clippy::needless_range_loop)]
// not const and tends to be longer
#![allow(clippy::manual_range_contains)]
// we need certain hot loops to stay separate
#![allow(clippy::branches_sharing_code)]
// TODO when clippy issue 9175 is fixed remove
#![allow(clippy::question_mark)]
#![deny(unsafe_op_in_unsafe_fn)]

#[doc(hidden)]
pub use awint_internals;
pub use awint_internals::{bw, SerdeError};

pub(crate) mod data;
pub use data::{Bits, InlAwi};

mod logic;

pub use logic::OrdBits;

/// Subset of `awint::awi`
pub mod awi {
    pub use awint_internals::awi::*;
    pub use Option::{None, Some};
    pub use Result::{Err, Ok};

    pub use crate::{Bits, InlAwi};
}
