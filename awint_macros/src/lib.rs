//! # Accompanying procedural macros to `awint`
//!
//! ## Dependencies
//!
//! All of the macros usually require `InlAwi` to be in scope. This could be
//! from `awint_core` or be a reexport from `awint`. `extawi!` always requires
//! `ExtAwi` to be in scope, which could be imported from `awint_ext` or be a
//! reexport from `awint`.
//!
//!
//! ## Concatenations of Components
//!
//! All of the macros in this crate except for `inlawi_ty` accept what we call
//! "concatenations of components". A component can be a literal, a variable, or
//! a filler. Components are written in a big endian order (like literals), and
//! concatenations are written in a little endian order (because large
//! concatentations will usually be formatted on different lines, and we want
//! the data flow to be downwards), so the general layout of the input to a
//! macro is:
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
//! ## Literals
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
//! // Literals can have ranges applied to them, which might be useful
//! // in some circumstances for readability.
//! assert_eq!(inlawi!(0x12345_u20[4..16]), inlawi!(0x234_u12));
//! ```
//!
//! ## Variables
//!
//! ## Fillers
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
//! con!{
//!     0x321u12;
//!     .., y;
//! }
//! con!{
//!     .., 0x321u12;
//!     y;
//! }
//! con!{
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
//! con!{
//!     0x321u12, ..;
//!     y, ..;
//! }
//!
//! Concatenations are even smart enough to do this:
//! con!(0x3u4, .., 0x21u8; y)
//! con!(0x3u4, .., 0x21u8; y)
//!
//! Note again that filler bitwidths cannot be negative, and so this will cause
//! an error because we are trying to compress the 4 bit and 8 bit components
//! into a less than 12 bit space.
//! //con!(0x3u4, .., 0x21u8; 0..1)
//!
//! Only one unbounded filler per concatenation is allowed. Consider this case,
//! in which it would be ambiguous about where the middle component
//! should be aligned.
//! con!{
//!     .., 0x321u12, ..;
//!     y;
//! }
//!
//! // Additionally, multiple concatenations with unbounded fillers must all
//! have their fillers aligned to the same end or have a concatenation without
//! an unbounded filler.
//! // all allowed:
//! con!{
//!     .., x;
//!     .., y;
//!     .., z;
//! }
//! con!{
//!     x, ..;
//!     y, ..;
//!     z, ..;
//! }
//! con!{
//!     .., x;
//!     y, .., z;
//!     a, ..;
//!     b;
//! }
//! // disallowed, because the overlaps are ambiguous
//! con!{
//!     .., x;
//!     y, ..;
//! }
//! con!{
//!     a, .., x;
//!     b, .., y;
//!     c, .., z;
//! }
//!
//! // It is technically possible to infer that ambiguous overlap could not
//! occur in this case, but this is still disallowed by the macro, and it is
//! more readable to just split the macro into two for both alignments. con!{
//!     0x4567u16, .., 0x123u12;
//!     y0[..4], .., y1[..4];
//! }
//! // the above macro is equivalent to these two macros combined:
//! con!{
//!     0x4567u16, ..;
//!     y0[..4], ..;
//! }
//! con!{
//!     .., 0x123u12;
//!     .., y1[..4];
//! }

#![feature(proc_macro_hygiene)]
#![allow(clippy::needless_range_loop)]

extern crate alloc;
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

extern crate proc_macro;
use core::num::NonZeroUsize;

use awint_ext::ExtAwi;
use awint_internals::*;
use proc_macro::TokenStream;

use crate::code_gen::code_gen;
mod parse;
pub(crate) use parse::*;
mod code_gen;
pub(crate) mod structs;
mod tests;

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

fn general_inlawi(components: Vec<String>) -> String {
    let mut awis: Vec<ExtAwi> = Vec::new();
    let mut total_bw = 0;
    for component in components {
        match component.parse::<ExtAwi>() {
            Ok(awi) => {
                total_bw += awi.bw();
                awis.push(awi)
            }
            Err(e) => panic!(
                "could not parse component \"{}\": `<ExtAwi as FromStr>::from_str` returned \
                 SerdeError::{:?}",
                component, e
            ),
        }
    }
    if awis.is_empty() {
        panic!("empty input");
    }
    let total_bw = NonZeroUsize::new(total_bw).unwrap();
    let mut awi = ExtAwi::zero(total_bw);
    let mut shl = 0;
    for component in awis {
        awi[..].field(shl, &component[..], 0, component.bw());
        shl += component.bw();
    }

    // no safety worries here, because the `unstable_from_slice` has strict checks
    let mut raw: Vec<usize> = Vec::new();
    raw.extend_from_slice(awi[..].as_slice());
    raw.push(total_bw.get());
    format!(
        "InlAwi::<{}, {}>::unstable_from_slice(&{:?})",
        total_bw,
        awi.raw_len(),
        &raw[..],
    )
}

// TODO `0x1234.5678p4i32p16`
// prefix,integral,point,fractional,exPonent (can't use 'e' because it doesn't
// work in hexadecimal, maybe allow for decimal),signed,bitwidth,point position

// General construction of an `InlAwi`. The input should be a comma-separated
// list of literals that can be parsed by `<ExtAwi as FromStr>::from_str`. The
// resulting arbitrary width integers are concatenated together to create one
// `InlAwi` at compile time. The individual literals are big-endian (processed
// how `from_str` would normally process them), while the component
// concatenation order is little-endian.
/*#[proc_macro]
pub fn inlawi(input: TokenStream) -> TokenStream {
    let mut components: Vec<String> = Vec::new();
    for part in input.to_string().split(',') {
        // Remove whitespace. Ideally, we would not have to do this and send inputs
        // directly to the parser, but `TokenStream::to_string` inserts a space
        // inbetween `-` signs and the integral part.
        let mut component = String::new();
        for c in part.chars() {
            if !c.is_whitespace() {
                component.push(c);
            }
        }
        components.push(component);
    }

    general_inlawi(components).parse().unwrap()
}*/

/// General construction of an `InlAwi`. The input should be a comma-separated
/// list of literals that can be parsed by `<ExtAwi as FromStr>::from_str`. The
/// resulting arbitrary width integers are concatenated together to create one
/// `InlAwi` at compile time. The individual literals and component
/// concatenation order are both big-endian.
#[proc_macro]
pub fn inlawi_be(input: TokenStream) -> TokenStream {
    let mut components: Vec<String> = Vec::new();
    for part in input.to_string().split(',').rev() {
        let mut component = String::new();
        for c in part.chars() {
            if !c.is_whitespace() {
                component.push(c);
            }
        }
        components.push(component);
    }

    general_inlawi(components).parse().unwrap()
}

/// General construction of an `InlAwi`. The input should be a comma-separated
/// list of literals that can be parsed by `<ExtAwi as FromStr>::from_str`. The
/// resulting arbitrary width integers are concatenated together to create one
/// `InlAwi` at compile time. The individual literals and component
/// concatenation order are both little-endian.
#[proc_macro]
pub fn inlawi_le(input: TokenStream) -> TokenStream {
    let mut components: Vec<String> = Vec::new();
    for part in input.to_string().split(',') {
        let mut component = String::new();
        for c in part.chars().rev() {
            if !c.is_whitespace() {
                component.push(c);
            }
        }
        components.push(component);
    }

    general_inlawi(components).parse().unwrap()
}

#[proc_macro]
pub fn con(input: TokenStream) -> TokenStream {
    match code_gen(&input.to_string(), false, false) {
        Ok(s) => s.parse().unwrap(),
        Err(s) => panic!("{}", s),
    }
}

#[proc_macro]
pub fn inlawi(input: TokenStream) -> TokenStream {
    match code_gen(&input.to_string(), true, true) {
        Ok(s) => s.parse().unwrap(),
        Err(s) => panic!("{}", s),
    }
}

#[proc_macro]
pub fn extawi(input: TokenStream) -> TokenStream {
    match code_gen(&input.to_string(), false, true) {
        Ok(s) => s.parse().unwrap(),
        Err(s) => panic!("{}", s),
    }
}
