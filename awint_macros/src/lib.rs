//! # Accompanying procedural macros to `awint`
//!
//! ```
//! use awint_macros::{inlawi_ty, inlawi, inlawi_le, inlawi_be};
//! // Note: The macros require `InlAwi` to be in scope. This could be from
//! // `awint_core` or a reexport from `awint`.
//! use awint_core::InlAwi;
//!
//! // constructs an `InlAwi` out of a 16 bit negative signed 1, a 16 bit
//! // binary string, and 8 bit unsigned 42. The total bitwidth is
//! // 16 + 16 + 8 = 40.
//! let awi0: inlawi_ty!(40) = inlawi!(-1i16, 0000_0101_0011_1001, 42u8);
//!
//! // We use `inlawi!` again to show the different components in hexadecimal.
//! // Note that while the literals are typed in big-endian, the list and
//! // the way they are concatenated is in little-endian. This matches the
//! // way Rust array literals (e.g. `[0xffff, 0x539, 0x2a]`) work.
//! let awi1 = inlawi!(0xffffu16, 0x539u16, 0x2au8);
//! assert_eq!(awi0, awi1);
//!
//! // `inlawi_be!` interprets both the literals and list order as big-endian
//! let awi2 = inlawi_be!(0x2au8, 0x539u16, 0xffffu16);
//! assert_eq!(awi0, awi2);
//!
//! // `inlawi!` should be used in most circumstances with one literal
//! let awi3 = inlawi!(0x2A_0539_FFFFu40);
//! assert_eq!(awi0, awi3);
//!
//! // `inlawi_le!` uses little endian for both literals and the component
//! // order. This macro is intended mainly for plain binary strings
//! // (e.x. `inlawi_le!(00101, 0011) == inlawi_be!(1100, 10100)`)
//! let awi4 = inlawi_le!(61uffffx0, 61u935x0, 8ua2x0);
//! assert_eq!(awi0, awi4);
//! ```

#![feature(proc_macro_hygiene)]

extern crate alloc;
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

extern crate proc_macro;
use core::num::NonZeroUsize;

use awint_ext::ExtAwi;
use proc_macro::TokenStream;

const BITS: usize = usize::BITS as usize;

fn inlawi_digits(bw: usize) -> usize {
    bw.wrapping_shr(BITS.trailing_zeros())
        .wrapping_add(((bw & (BITS - 1)) != 0) as usize)
        .wrapping_add(1)
}

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
    format!("InlAwi<{}, {}>", bw, inlawi_digits(bw))
        .parse()
        .unwrap()
}

/// Zero-value construction of an `InlAwi`. The input should be a `usize`
/// literal indicating bitwidth.
#[proc_macro]
pub fn inlawi_zero(input: TokenStream) -> TokenStream {
    let bw = input
        .to_string()
        .parse::<usize>()
        .expect("Input should parse as a `usize`");
    if bw == 0 {
        panic!("Tried to construct an `InlAwi` with an invalid bitwidth of 0");
    }
    format!(
        "InlAwi::<{}, {}>::unstable_zero({})",
        bw,
        inlawi_digits(bw),
        bw
    )
    .parse()
    .unwrap()
}

/// Unsigned-maximum-value construction of an `InlAwi`. The input should be a
/// `usize` literal indicating bitwidth.
#[proc_macro]
pub fn inlawi_umax(input: TokenStream) -> TokenStream {
    let bw = input
        .to_string()
        .parse::<usize>()
        .expect("Input should parse as a `usize`");
    if bw == 0 {
        panic!("Tried to construct an `InlAwi` with an invalid bitwidth of 0");
    }
    format!(
        "InlAwi::<{}, {}>::unstable_umax({})",
        bw,
        inlawi_digits(bw),
        bw
    )
    .parse()
    .unwrap()
}

fn general_inlawi(components: Vec<String>) -> String {
    let mut awis: Vec<ExtAwi> = Vec::new();
    for component in components {
        match component.parse::<ExtAwi>() {
            Ok(awi) => awis.push(awi),
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
    let mut total_bw = 0;
    for awi in awis.iter() {
        total_bw += awi.bw();
    }
    let total_bw = NonZeroUsize::new(total_bw).unwrap();
    let mut awi = ExtAwi::zero(total_bw);
    let mut tmp = ExtAwi::zero(total_bw);
    let mut shl = 0;
    for component in awis {
        tmp[..].zero_resize_assign(&component[..]);
        tmp[..].shl_assign(shl).unwrap();
        shl += component.bw();
        awi[..].or_assign(&tmp[..]);
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

/// General construction of an `InlAwi`. The input should be a comma-separated
/// list of literals that can be parsed by `<ExtAwi as FromStr>::from_str`. The
/// resulting arbitrary width integers are concatenated together to create one
/// `InlAwi` at compile time. The individual literals are big-endian (processed
/// how `from_str` would normally process them), while the component
/// concatenation order is little-endian.
#[proc_macro]
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
}

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
