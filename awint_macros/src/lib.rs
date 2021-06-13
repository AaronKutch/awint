//! # Accompanying procedural macros to `awint`
//!
//! ## Dependencies
//!
//! All of the macros usually require `InlAwi` to be in scope. This could be
//! from `awint_core` or be a reexport from `awint`. `extawi!` always requires
//! `ExtAwi` to be in scope, which could be imported from `awint_ext` or be a
//! reexport from `awint`. `cc!` may require none, one, or both depending on the
//! input.
//!
//! ## Concatenations of Components
//!
//! Some of the macros accept "concatenations of components". A component can be
//! a literal, a variable, or a filler. Components are written in a big endian
//! order (like literals), and concatenations are written in a little endian
//! order (because large concatentations will usually be formatted on different
//! lines, and we want the data flow to be downwards), so the general layout of
//! the input to a macro is:
//!
//! ```text
//! macro!(
//!     ..., component 2, component 1, component 0; // concatenation 0
//!     ..., component 2, component 1, component 0; // concatenation 1
//!     ..., component 2, component 1, component 0; // concatenation 2
//!                             ⋮
//! )
//! ```
//!
//! The first concatenation, or concatenation 0, is the "source" concatenation,
//! and the following concatenations are "sink" concatenations. Either
//! statically at compile time or dynamically at run time, the macro will check
//! if the concatenations all have the same bitwidths. If so, the corresponding
//! bits from the source get copied to the corresponding bits of the sinks. The
//! construction macros additionally return the source concatenation in a
//! storage type such as `InlAwi` or `ExtAwi`.
//!
//! ### Literals
//!
//! When the parser sees that the first character of a component is '-' or
//! '0'..='9', and it isn't part of a lone range, it will assume the component
//! is a literal and pass it to the `FromStr` implementation of `ExtAwi`. See
//! that documentation for more details.
//!
//! ```
//! use awint::{InlAwi, inlawi, inlawi_ty};
//!
//! // Here, we pass a single concatenation of 3 literals to the `inlawi!`
//! // construction macro. constructs an `InlAwi` out of a 4 bit signed
//! // negative 1, a 16 bit binary string, and a 8 bit unsigned 42. The
//! // total bitwidth is 4 + 16 + 8 = 28.
//! let awi: inlawi_ty!(28) = inlawi!(-1i4, 0000_0101_0011_1001, 42u8);
//! assert_eq!(awi, inlawi!(1111_0000010100111001_00101010));
//!
//! // Literals can have static ranges applied to them, which might be useful
//! // in some circumstances for readability.
//! assert_eq!(inlawi!(0x654321_u24[4..16]), inlawi!(0x432_u12));
//!
//! // Arbitrary dynamic ranges using things from outside the macro can also
//! // be applied. The macros will assume the range bounds to result in `usize`.
//!
//! // TODO
//! //let x = 4;
//! //let y = 8;
//! //let awi = ExtAwi::zero(bw(12));
//! //assert_eq!(extawi!(0x98765_u20[y..(awi.bw() + x)]).unwrap(), extawi!(87u8));
//! ```
//!
//! ### Variables
//!
//!
//!
//! ### Fillers
//!
//! The third type of component is written as a range with no variable or
//! literal attached. When used in sources, corresponding sink bits are left
//! unmutated. When used in sinks, corresponding source bits have no effect.
//!
//! Fillers are not allowed in the source bits of the construction macros
//! `inlawi!` and `extawi!`, because it would be ambiguous what those bits
//! should initially be set to.
//!
//! ### Unbounded fillers
//!
//! Unbounded fillers can be thought as dynamically resizing fillers that
//! try to expand until the bitwidths of different concatenations match.
//!
//! // To understand how unbounded fillers interact, consider these three cases:
//! cc!{
//!     0x321u12;
//!     .., y;
//! }
//! cc!{
//!     .., 0x321u12;
//!     y;
//! }
//! cc!{
//!     .., 0x321u12;
//!     .., y;
//! }
//!
//! The first case has no fillers in the first concatenation, so the filler in
//! the second concatenation will expand to be of bitwidth `12 - y.bw()`.
//!
//! TODO
//!
//! Note that bitwidths cannot be negative, so if `y.bw() > 12` the macro will
//! return `None`
//!
//! The second case ends up enforcing that `y.bw()` is at least 12. The 12 bits
//! of the literal always get copied to the least significant 12 bits of `y`.
//!
//! The third case allows `y.bw()` to be anything. If `y.bw() < 12` it will act
//! like the first case, otherwise it acts like the second case.
//!
//! Unbounded fillers are also allowed in less significant positions, in
//! which case alignment of the components occurs starting from the most
//! significant bit.
//! cc!{
//!     0x321u12, ..;
//!     y, ..;
//! }
//!
//! Concatenations are even smart enough to do this:
//! cc!(0x3u4, .., 0x21u8; y)
//! cc!(0x3u4, .., 0x21u8; y)
//!
//! Note again that filler bitwidths cannot be negative, and so this will cause
//! an error because we are trying to compress the 4 bit and 8 bit components
//! into a less than 12 bit space.
//! //cc!(0x3u4, .., 0x21u8; 0..1)
//!
//! Only one unbounded filler per concatenation is allowed. Consider this case,
//! in which it would be ambiguous about where the middle component
//! should be aligned.
//! cc!{
//!     .., 0x321u12, ..;
//!     y;
//! }
//!
//! // Additionally, multiple concatenations with unbounded fillers must all
//! have their fillers aligned to the same end or have a concatenation without
//! an unbounded filler.
//! // all allowed:
//! cc!{
//!     .., x;
//!     .., y;
//!     .., z;
//! }
//! cc!{
//!     x, ..;
//!     y, ..;
//!     z, ..;
//! }
//! cc!{
//!     .., x;
//!     y, .., z;
//!     a, ..;
//!     b;
//! }
//! // disallowed, because the overlaps are ambiguous
//! cc!{
//!     .., x;
//!     y, ..;
//! }
//! cc!{
//!     a, .., x;
//!     b, .., y;
//!     c, .., z;
//! }
//!
//! // It is technically possible to infer that ambiguous overlap could not
//! occur in this case, but this is still disallowed by the macro, and it is
//! more readable to just split the macro into two for both alignments. cc!{
//!     0x4567u16, .., 0x123u12;
//!     y0[..4], .., y1[..4];
//! }
//! // the above macro is equivalent to these two macros combined:
//! cc!{
//!     0x4567u16, ..;
//!     y0[..4], ..;
//! }
//! cc!{
//!     .., 0x123u12;
//!     .., y1[..4];
//! }
//!
//! ### Other Notes
//!
//! - Because bitwidths cannot be zero any range "0..r" is equivalent to "..r"
//!   in the macros
//! - No warnings about unused `Option<()>`s are produced from procedural
//!   macros. If the `cc!` macro is called without appending `.unwrap()` or
//!   otherwise handling the `Option`, it will silently do nothing if `None` is
//!   returned.
//! - The macros have to be unsanitized to support using arbitrary variables and
//!   ranges. A best effort with plenty of parenthesis scoping is made so that
//!   only the most obviously bad inputs (such as multiple unmatched parenthesis
//!   and inputing things like "unsafe") could cause compiling broken checks or
//!   errors that extend beyond the scope of the macro.

#![feature(proc_macro_hygiene)]
#![allow(clippy::needless_range_loop)]

extern crate alloc;
use alloc::{format, string::ToString};

extern crate proc_macro;
use awint_ext::internals::code_gen;
use awint_internals::*;
use proc_macro::TokenStream;

/// Specifies an `InlAwi` _type_ in terms of its bitwidth as a `usize` literal.
#[proc_macro]
pub fn inlawi_ty(input: TokenStream) -> TokenStream {
    let bw = input
        .to_string()
        .parse::<usize>()
        .expect("Input should parse as a `usize`");
    if bw == 0 {
        panic!("Tried to make an `InlAwi` type with an invalid bitwidth of 0");
    }
    format!("InlAwi<{}, {}>", bw, raw_digits(bw))
        .parse()
        .unwrap()
}

// TODO `0x1234.5678p4i32p16`
// prefix,integral,point,fractional,exPonent (can't use 'e' because it doesn't
// work in hexadecimal, maybe allow for decimal),signed,bitwidth,point position

// C4D
/// Copy Corresponding Concatenations of Components Dynamically. Takes
/// concatenations of components as an input, and copies bits of the source to
/// corresponding bits of the sinks. Returns an `Option<()>`, and returns `None`
/// if component indexes are out of bounds or if concatenation bitwidths
/// mismatch. Performs allocation in general, but will try to avoid allocation
/// if ranges are all static or if concatenations are all of single components.
/// See the documentation of `awint_macros` for more.
#[proc_macro]
pub fn cc(input: TokenStream) -> TokenStream {
    match code_gen(&input.to_string(), false, false) {
        Ok(s) => s.parse().unwrap(),
        Err(s) => panic!("{}", s),
    }
}

/// Takes concatenations of components as an input, and copies bits of the
/// source to corresponding bits of the sinks. The source is also used to
/// construct an `InlAwi`. The common width must be statically determinable by
/// the macro (e.g. at least one concatenation must have only literal ranges),
/// and the source cannot contain fillers. Returns a plain `InlAwi` if only
/// literals with literal ranges are used, otherwise returns `Option<InlAwi>`.
/// Returns `None` if component indexes are out of range or if concatenation
/// bitwidths mismatch. See the documentation of `awint_macros` for more.
#[proc_macro]
pub fn inlawi(input: TokenStream) -> TokenStream {
    match code_gen(&input.to_string(), true, true) {
        Ok(s) => s.parse().unwrap(),
        Err(s) => panic!("{}", s),
    }
}

/// Takes concatenations of components as an input, and copies bits of the
/// source to corresponding bits of the sinks. The source is also used to
/// construct an `ExtAwi`. The common width must be dynamically determinable by
/// the macro (e.g. not all concatenations can have unbounded fillers), and the
/// source cannot contain fillers. Returns a plain `ExtAwi` if only literals
/// with literal ranges are used, otherwise returns `Option<ExtAwi>`. Returns
/// `None` if component indexes are out of range or if concatenation
/// bitwidths mismatch. See the documentation of `awint_macros` for more.
#[proc_macro]
pub fn extawi(input: TokenStream) -> TokenStream {
    match code_gen(&input.to_string(), false, true) {
        Ok(s) => s.parse().unwrap(),
        Err(s) => panic!("{}", s),
    }
}

macro_rules! inlawi_construction {
    ($($fn_name:ident, $inlawi_fn:expr, $doc:expr);*;) => {
        $(
            #[doc = $doc]
            #[doc = "construction of an `InlAwi`."]
            #[doc = "The input should be a `usize` literal indicating bitwidth."]
            #[proc_macro]
            pub fn $fn_name(input: TokenStream) -> TokenStream {
                let bw = input
                    .to_string()
                    .parse::<usize>()
                    .expect("Input should parse as a `usize`");
                if bw == 0 {
                    panic!("Tried to construct an `InlAwi` with an invalid bitwidth of 0");
                }
                format!("InlAwi::<{}, {}>::{}()", bw, raw_digits(bw), $inlawi_fn)
                    .parse()
                    .unwrap()
            }
        )*
    };
}

inlawi_construction!(
    inlawi_zero, "zero", "Zero-value";
    inlawi_umax, "umax", "Unsigned-maximum-value";
    inlawi_imax, "imax", "Signed-maximum-value";
    inlawi_imin, "imin", "Signed-minimum-value";
    inlawi_uone, "uone", "Unsigned-one-value";
);
