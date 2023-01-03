//! # Accompanying procedural macros to `awint`
//!
//! ## Scope Dependencies
//!
//! All of the macros often require `Bits` and `InlAwi` to be in scope. This
//! could be from `awint_core` or be a reexport from `awint`. `extawi!` and
//! similar always requires `ExtAwi` to be in scope, which could be imported
//! from `awint_ext` or be a reexport from `awint`. `cc!` may require none,
//! one, or both storage types depending on the input. Macros with fallible
//! inputs require `Option<T>` variants to be in scope.
//!
//! ## Concatenations of Components
//!
//! Some of the macros accept "concatenations of components". A component can
//! be a literal, a variable, or a filler. Components are written in a big
//! endian order (so that the big-endianness of literals flows the same way
//! visually), and concatenations are written in a little endian order (because
//! large concatentations will usually be formatted on different lines, and we
//! want the data flow to be downwards), so the general layout of the input to
//! a macro is:
//!
//! ```text
//! macro!(
//!     ..., component 2, component 1, component 0; // concatenation 0
//!     ..., component 2, component 1, component 0; // concatenation 1
//!     ..., component 2, component 1, component 0; // concatenation 2
//!                             ⋮
//! )
//! ```
//! (there may also be a specified initialization prefixed with a `:` at the
//! beginning, but we will get to that later)
//!
//! The first concatenation, or concatenation 0, is the "source" concatenation,
//! and the following concatenations are "sink" concatenations. Either
//! statically at compile time or dynamically at run time, the macro will check
//! if the concatenations all have the same bitwidths. If so, the corresponding
//! bits from the source get copied to the corresponding bits of the sinks. The
//! construction macros additionally return the source concatenation in a
//! storage type such as `InlAwi` or `ExtAwi`. The sink concatenations are
//! optional. If there is only a source concatenation, `inlawi_` and `extawi_`
//! will just construct the value of the source, and `cc_` will just perform
//! bounds checks.
//!
//! These macros automate a large number of things for the user:
//!  - Avoiding borrow errors even when the same variable is being used in
//!    different concatenations and in ranges of itself
//!  - Using only `const` capable constructions and functions if possible
//!  - All bounds checks are run before any fielding happens, so that no
//!    mutation or allocation occurs when an error is returned, matching the
//!    common behavior of functions in the `awint` system
//!  - Trying to optimize away as many bounds checks as possible
//!  - Trying to optimize away intermediate buffers
//!  - Concatenating literals together at compile time, and even returning
//!    infallibly if possible
//!  - Trying to use the most efficient copying method
//!
//! Before going into detail on each component type, we will first explain all
//! the error conditions for the index bounds checks. Even though `cc_` macros
//! with only a source concatenation do no construction or copying, they are
//! useful for bounds checks. Here, we pass the simplest variable input to the
//! `cc!` macro, a single concatenation with a single component.
//!
//! ```
//! use awint::awi::*;
//!
//! let x = ExtAwi::zero(bw(10));
//! let r0 = 2;
//! let r1 = 8;
//! let r2 = 20;
//!
//! // The input is just the variable `x`. The macro is able to determine that
//! // no bounds checks are needed, so it returns `()`.
//! assert_eq!(cc!(x), ());
//!
//! // This component is the variable `x` indexed with the bit range `r0..r1`.
//! // The bounds checks succeed, so the macro returns `Some(())`.
//! assert!(cc!(x[r0..r1]).is_some());
//!
//! // Inclusive ranges and single bit indexes are also recognized.
//! // this range is equivalent to `r0..(r1 + 1)`
//! assert!(cc!(x[r0..=r1]).is_some());
//! // this range is equivalent to `r0..(r0 + 1)`
//! assert!(cc!(x[r0]).is_some());
//!
//! // We could also use a static range. We call it "static" because the macro
//! // is able to know the value of the range at compile time.
//! assert!(cc!(x[2..8]).is_some());
//!
//! // The macro recognizes decimal, hexadecimal, octal and binary static
//! // values so that they seamlessly work with Rust literals.
//! assert!(cc!(x[2..0x8]).is_some());
//! assert!(cc!(x[0b10..0o10]).is_some());
//!
//! // The first kind of invalid bound is a reversed range, in which the
//! // start of the range is larger than the end of the range.
//! assert!(cc!(x[r1..r0]).is_none());
//!
//! // Here, the macro is able to determine at compile time that the range is
//! // reversed.
//! // error: determined statically that this has a reversed range
//! //cc!(x[8..2]);
//!
//! // The macros are able to perform some limited recognition of bounds of the
//! // form `(arbitrary + statically_known)` or
//! // `(-statically_known - -arbitrary)`, etc. This recognition is usually not
//! // externally visible unless you trigger it at compile time, because the
//! // macro still has to check that the arbitrary part of the range does not
//! // cause the bound to go out of range, but it does eliminate some checks
//! // and improves performance.
//!
//! // error: determined statically that this has a reversed range
//! //cc!(x[(r0 + 5)..(-5 + r0)]);
//!
//! // note that even though `r0` is arbitrary, the macro is able to recognize
//! // that it needs a `inlawi_ty(5)` without us telling it more explicitly
//! assert_eq!(inlawi!(x[r0..(r0 + 5)]).unwrap(), inlawi!(0u5));
//!
//! // The second kind of invalid bound is a range that extends beyond the
//! // width of the variable or literal. Earlier, the values of 2 and 8 were
//! // less than or equal to `x.bw()`, so the check succeeded. Here the value
//! // of 20 causes the macro to return `None`.
//! assert!(cc!(x[r0..r2]).is_none());
//!
//! // Note: the widths of concatenations can be zero for the `cc_` macros.
//! // Static zero width ranges will cause a compile-time panic as a warning
//! // about components that do nothing, but they are achievable with dynamic
//! // ranges.
//! let r0 = 5;
//! let r1 = 5;
//! assert!(cc!(x[r0..r1]).is_some());
//! let r0 = 10;
//! let r1 = 10;
//! assert!(cc!(x[r0..r1]).is_some());
//! // The restriction about values not being larger than `x.bw()` still
//! // applies.
//! let r0 = 11;
//! let r1 = 11;
//! assert!(cc!(x[r0..r1]).is_none());
//!
//! // `InlAwi`s and `ExtAwi`s cannot have zero bitwidths, so zero width
//! // concatenations will cause their macros to panic at compile time or
//! // return `None`.
//! let r = 5;
//! assert!(extawi!(x[r..5]).is_none());
//! // error: determined statically that this concatenation has zero width
//! //let _ = extawi!(x[5..5]);
//!
//! // The macros are parsed by using `proc-macro2` token trees and look only
//! // at punctuation in top level delimited groups. It will separate by ','
//! // and ';' to get components, and then each component will be parsed again
//! // for top level delimited groups, and if the last token tree in the
//! // component is "[]" delimited it will treat that as a bit indexer. This
//! // allows for almost every conceivable Rust expression being used:
//! assert!(
//!     cc!([inlawi!(0u10); 4][3][
//!         (|| {let _ = (4..5, 6..=7); let _ = "'\".,;"; 9})()
//!     ]).is_some()
//! );
//! // The first `[inlawi!(0u10); 4]` is an array of 4 `InlAwi`s, the middle
//! // `[3]` indexes the array of `InlAwi`s, and the rightmost "[]" delimited
//! // group is interpreted as a single bit index. The parsing is able to
//! // ignore inner puncutation and just use the `9` result of the closure as
//! // the index. Of course, you wouldn't want to use expressions this complex
//! // in a single line, most of the time you should use external bindings.
//!
//! let awis = [inlawi!(0); 4];
//! // If you are using normal indexing but do not want the macro to interpret
//! // it as a bit indexing, wrap the component in parenthesis, and then the
//! // macro will ignore everything inside (since it is not a top level "[]"
//! // group) and treat the thing as a single variable with no range applied.
//! assert_eq!(cc!( (&awis[3]) ), ());
//!
//! // The third error condition occurs when concatenation bitwidths are
//! // unequal, but first we need to go into more detail on the component
//! // types.
//! ```
//!
//! ### Literals
//!
//! After differentiating components and ranges, the parser will try to parse
//! range values as hexadecimal, octal, binary, or decimal `i128` values.
//! Later in codgen it will be converted to `usize`, and the compiler will
//! complain if literals are too large for the target architecture. The parser
//! will also attempt to parse components with the `FromStr` implementation of
//! `ExtAwi`. See that documentation for more details.
//!
//! Note: The `FromStr` implementation allows for signed and unsigned values,
//! binary, octal, decimal, and hexadecimal bases, but for the remainder of
//! this documentation we will mainly be using unsigned hexadecimal for
//! literals and decimal for range bounds. This is because hexadecimal neatly
//! divides along bit multiples of 4, and the large base allows one to easily
//! see where different groups of 4 bits are being copied.
//!
//! ```
//! use awint::awi::*;
//!
//! // Here, we pass a single concatenation of 3 literals to the `inlawi!`
//! // construction macro. This constructs an `InlAwi` out of a 4 bit signed
//! // negative 1, a 16 bit binary string, and a 8 bit unsigned 42. The
//! // total bitwidth is 4 + 16 + 8 = 28.
//! let awi: inlawi_ty!(28) = inlawi!(-1i4, 0000_0101_0011_1001, 42u8);
//! assert_eq!(awi, inlawi!(1111_0000010100111001_00101010));
//!
//! // Literals can have static ranges applied to them, which might be useful
//! // in some circumstances for readability. The macros automatically truncate
//! // and concatenate constants with statically known bounds together for best
//! // runtime performance.
//! assert_eq!(inlawi!(0x654321_u24[4..16]), inlawi!(0x432_u12));
//!
//! // Arbitrary dynamic ranges using things from outside the macro can also
//! // be applied. The macros will assume the range bounds to result in
//! // `usize`, and the compiler will typecheck this.
//! let x: usize = 8;
//! let awi = ExtAwi::zero(bw(12));
//! // At runtime, `x` evaluates to 8 and `x + awi.bw()` evaluates to 20
//! assert_eq!(
//!     extawi!(0x98765_u20[x..(x + awi.bw())]).unwrap(),
//!     extawi!(0x987u12)
//! );
//! ```
//!
//! ### Variables
//!
//! Anything that has well defined `bw() -> usize`, `Deref::deref() -> &Bits`,
//! `DerefMut::deref_mut() -> &mut Bits`, `AsRef::as_ref() -> &Bits`, and
//! `AsMut::as_mut() -> &mut Bits` functions can be used as a variable.
//! Arbitrary `Bits` references, `ExtAwi`, `InlAwi`, `FP<B>` and other well
//! defined arbitrary width integer types can thus be used as variables.
//!
//! ```
//! use awint::awi::*;
//!
//! let source = inlawi!(0xc4di64);
//! // a bunch of zeroed 64 bit arbitrary width integers from different
//! // storage types and construction methods.
//! let mut awi: ExtAwi = ExtAwi::zero(bw(64));
//! let mut a: &mut Bits = awi.const_as_mut();
//! let mut b: ExtAwi = extawi!(0i64);
//! let mut c: inlawi_ty!(64) = <inlawi_ty!(64)>::zero();
//! let mut d: FP<inlawi_ty!(64)> = FP::new(false, inlawi!(0u64), 32).unwrap();
//!
//! // Use the `cc` macro to copy the source concatenation `source` to the sink
//! // concatenations `a`, `b`, `c`, and `d`. Here, every concatenation is just
//! // a single variable component.
//! cc!(source; a; b; c; d).unwrap();
//!
//! assert_eq!(a, inlawi!(0xc4di64).const_as_ref());
//! assert_eq!(b, extawi!(0xc4di64));
//! assert_eq!(c, inlawi!(0xc4di64));
//!
//! let awi = inlawi!(0xau4);
//! let a = awi.const_as_ref();
//! let b = extawi!(0xbu4);
//! let c = inlawi!(0xcu4);
//!
//! // Use `extawi` to infallibly concatenate variables together. Here, there
//! // is only one concatenation with multiple variable components.
//! assert_eq!(extawi!(a, b, c), extawi!(0xabcu12));
//! ```
//!
//! Now that we have both literals and variables, we can demonstrate more
//! complicated interactions. In case it still isn't clear what the
//! "corresponding copying" means, here we have a source concatenation of 4
//! components each with 3 hexadecimal digits being copied onto a sink
//! concatenation of 3 components each with 4 hexadecimal digits.
//!
//! ```
//! use awint::awi::*;
//!
//! let y3 = inlawi!(0xba9u12);
//! let y2 = inlawi!(0x876u12);
//! let y1 = inlawi!(0x543u12);
//! let y0 = inlawi!(0x210u12);
//!
//! let mut z2 = inlawi!(0u16);
//! let mut z1 = inlawi!(0u16);
//! let mut z0 = inlawi!(0u16);
//!
//! cc!(
//!     y3, y2, y1, y0;
//!     z2, z1, z0;
//! ).unwrap();
//!
//! assert_eq!(z2, inlawi!(0xba98u16));
//! assert_eq!(z1, inlawi!(0x7654u16));
//! assert_eq!(z0, inlawi!(0x3210u16));
//! ```
//!
//! Visually, the components in the two concatenations are being aligned like
//! this:
//!
//! ```text
//! |-----y3----|-----y2----|-----y1----|-----y0----|
//! | b   a   9 | 8   7   6 | 5   4   3 | 2   1   0 |
//! | b   a   9   8 | 7   6   5   4 | 3   2   1   0 |
//! |-------z2------|-------z1------|-------z0------|
//! ```
//!
//! Again, arbitrary ranges can be applied to variables, note there are no
//! borrowing errors:
//!
//! ```
//! use awint::awi::*;
//!
//! let mut a = inlawi!(0x9876543210u40);
//!
//! // Note: because the range bounds cannot be negative, ranges starting
//! // with 0 (e.x. `0..r`) can have the zero omitted and just use `..r`.
//! // Ranges ending like `r..` will include all bits up to the most
//! // significant bit of the variable.
//! let b = extawi!(
//!     a[..=7], a[(a.bw() - 16)..];
//!     a[(5 * 4)..(9 * 4)], a[..(2 * 4)];
//! ).unwrap();
//! assert_eq!(a, inlawi!(0x9109843276u40));
//! assert_eq!(b, extawi!(0x109876_u24));
//! ```
//!
//! Also note: if you see the compiler complain about mutability of `Bits`
//! references, ignore the help message about removing the `&mut` and make
//! the reference itself mutable. This happens because the macro can't tell
//! if what it is binding to is already a mutable reference and not a storage
//! type, and so always tries to take a `&mut Bits` reference of variables.
//! ```
//! use awint::awi::*;
//!
//! // error: cannot borrow x and z as mutable ...
//! //fn test(x: &mut Bits) {
//! //    cc!(0x123u12; x).unwrap();
//! //
//! //    let mut y = inlawi!(0u12);
//! //    let z = y.const_as_mut();
//! //    cc!(0x123u12; z).unwrap();
//! //}
//!
//! // add the extra `mut` in front of references
//! fn test(mut x: &mut Bits) {
//!     cc!(0x123u12; x).unwrap();
//!
//!     let mut y = inlawi!(0u12);
//!     let mut z = y.const_as_mut();
//!     cc!(0x123u12; z).unwrap();
//! }
//! ```
//!
//! Also, if you are using the `const_support` feature and are trying to use
//! the macros in a `const` context, as of Rust 1.63 you need to add all of
//! ```text
//! #![feature(const_mut_refs)]
//! #![feature(const_option)]
//! #![feature(const_trait_impl)]
//! ```
//! to your crate roots or else you will encounter many confusing or misleading
//! compiler errors.
//!
//! ### Fillers
//!
//! The third type of component is written as a range by itself. When used in
//! sources, corresponding sink bits are left unmutated. When used in sinks,
//! corresponding source bits have no effect.
//!
//! ```
//! use awint::awi::*;
//!
//! // filler bits in source concatenations have no effects and sink bits are
//! // preserved
//! let x = inlawi!(0xabcd_u16);
//!
//! let mut y = inlawi!(0x123456_u24);
//! cc!(
//!     x, ..8;
//!     y;
//! ).unwrap();
//! assert_eq!(y, inlawi!(0xabcd56_u24));
//!
//! y = inlawi!(0x123456_u24);
//! cc!(
//!     ..8, x;
//!     y;
//! ).unwrap();
//! assert_eq!(y, inlawi!(0x12abcd_u24));
//!
//! // filler bits in sink concatenations can act as spacers that drop the
//! // effects of source bits, note that the effect is processed independently
//! // for every sink concatenation.
//! let x = inlawi!(0x123456_u24);
//! let mut y = inlawi!(0u16);
//! let mut z = inlawi!(0u16);
//! cc!(
//!     x;
//!     y, ..8;
//!     ..8, z;
//! ).unwrap();
//! assert_eq!(y, inlawi!(0x1234_u16));
//! assert_eq!(z, inlawi!(0x3456_u16));
//! ```
//!
//! Fillers are also useful in cases where all concatenations lack a needed
//! degree of determinable width, and we want a cheap way to specify it:
//!
//! ```
//! use awint::awi::*;
//!
//! let x = extawi!(-99i44);
//!
//! // error: `InlAwi` construction macros need at least one concatenation to
//! // have a width that can be determined statically by the macro
//! //inlawi!(x);
//!
//! // because the second concatenation is only filler, no operations occur,
//! // but the macro will be able to determine that the common width is 44.
//! assert_eq!(inlawi!(x; ..44).unwrap(), inlawi!(-99i44));
//!
//! // `extawi!` and `cc!` can also run into the problem with unbounded
//! // fillers, which will be explained later. We can additionally use a
//! // dynamic range (but be sure to always use a static filler concatenation
//! // if possible so that internal buffers can be on the stack).
//!
//! // error: ... no concatenation has a statically or dynamically determinable
//! // width
//! //let _ = cc!(zero: 1, .., 1010; .., y);
//!
//! let r = 16;
//! let mut y = InlAwi::from_u8(0);
//! cc!(zero:
//!     1, .., 1010;
//!     .., y;
//!     ..r;
//! ).unwrap();
//! assert_eq!(y, InlAwi::from_u8(0b1010));
//! ```
//!
//! We run into a problem when using fillers in the source concatenation of a
//! construction macro. In the `cc` macro the source bits are not returned, so
//! it doesn't matter that they do not have a well defined value. In a
//! construction macro however, all the bits of the source need to have some
//! kind of set value.
//! ```
//! use awint::awi::*;
//!
//! // error: a construction macro with unspecified initialization cannot have
//! // a filler in the source concatenation
//! //let x = extawi!(..8);
//!
//! // we introduce the initialization specifier, which can be one of "zero",
//! // "umax", "imax", "imin", or "uone" corresponding to the standard
//! // construction functions of `ExtAwi` and `InlAwi` (and you can add your
//! // own initializations by implementing traits for `InlAwi` and `ExtAwi`
//! // with functions matching the formats of the `InlAwi` constructors and the
//! // hidden `panicking_` functions on `ExtAwi`)
//! let x = extawi!(umax: ..8);
//! assert_eq!(x, ExtAwi::umax(bw(8)));
//!
//! let mut x = extawi!(0u64);
//! // equivalent to `x.umax_()`
//! cc!(umax: ..; x);
//! assert_eq!(x, ExtAwi::umax(bw(64)));
//!
//! let mut x = extawi!(-99i44);
//! // "umax" is all set bits, so we are sign extending this negative value
//! let r = 128;
//! assert_eq!(extawi!(umax: .., x; ..r).unwrap(), extawi!(-99i128));
//!
//! assert_eq!(inlawi!(zero: 0xau4, ..4, 0xbu4, ..4), inlawi!(0xa0b0u16));
//! ```
//!
//! ### Unbounded fillers
//!
//! Unbounded fillers can be thought as dynamically resizing fillers that
//! try to expand until the bitwidths of different concatenations match. To
//! understand how unbounded fillers interact, consider these three cases:
//!
//! ```
//! use awint::awi::*;
//!
//! // This first case has no fillers in the first concatenation, so the filler
//! // in the second concatenation will expand to be of bitwidth `12 - y.bw()`.
//!
//! let mut y = extawi!(0u8);
//! cc!(
//!     0x321u12;
//!     .., y;
//! ).unwrap();
//! assert_eq!(y, extawi!(0x21u8));
//!
//! // Do the same call again, but with `y.bw() == 4`
//! let mut y = extawi!(0u4);
//! cc!(
//!     0x321u12;
//!     .., y;
//! ).unwrap();
//! assert_eq!(y, extawi!(0x1u4));
//!
//! // The 12 bits of the first concatenation cannot correspond with
//! // the minimum 16 bits of the second, so this call returns `None`.
//! let mut y = extawi!(0u16);
//! assert!(cc!(0x321u12; .., y).is_none());
//!
//! // This second case ends up enforcing that `y.bw()` is at least 12. The 12
//! // bits of the literal always get copied to the least significant bits of
//! // `y`.
//!
//! let mut y = extawi!(0u20);
//! cc!(
//!     .., 0x321u12;
//!     y;
//! ).unwrap();
//! assert_eq!(y, extawi!(0x00321u20));
//!
//! let mut y = extawi!(0u32);
//! cc!(umax:
//!     .., 0x321u12;
//!     y;
//! ).unwrap();
//! assert_eq!(y, extawi!(0xffff_f321_u32));
//!
//! let mut y = extawi!(0u8);
//! assert!(cc!(.., 0x321u12; y).is_none());
//!
//! // The third case allows `y.bw()` to be any possible bitwidth. If
//! // `y.bw() < 12` it will act like the first case, otherwise it acts like
//! // the second case. Because there are no restrictions on concatenation
//! // widths and there are no ranges that could index the variables out of
//! // bounds, these calls are infallible and no `Option` is returned.
//!
//! let mut y = extawi!(0u20);
//! cc!(
//!     .., 0x321u12;
//!     .., y;
//! );
//! assert_eq!(y, extawi!(0x00321u20));
//!
//! let mut y = extawi!(0u4);
//! cc!(
//!     .., 0x321u12;
//!     .., y;
//! );
//! assert_eq!(y, extawi!(0x1u4));
//!
//! // Unbounded fillers are also allowed in less significant positions, in
//! // which case alignment of the components occurs starting from the most
//! // significant bit.
//!
//! let mut y = extawi!(0u20);
//! cc!(
//!     0x321u12, ..;
//!     y, ..;
//! );
//! assert_eq!(y, extawi!(0x32100u20));
//!
//! // The macros are even smart enough to do this:
//!
//! let mut y = extawi!(0u24);
//! cc!(umax: 0x3u4, .., 0x21u8; y).unwrap();
//! assert_eq!(y, extawi!(0x3fff21u24));
//!
//! // Note again that filler widths cannot be negative, and so this will cause
//! // an error because we are trying to compress the 4 bit and 8 bit
//! // components into a less than 12 bit space.
//!
//! let mut y = extawi!(0u8);
//! assert!(cc!(0x3u4, .., 0x21u8; y).is_none());
//! ```
//!
//! Only one unbounded filler per concatenation is allowed. Consider this case,
//! in which it would be ambiguous about how the middle component should be
//! aligned.
//!
//! ```text
//! cc!(
//!     .., 0x321u12, ..;
//!     y;
//! ); // Error: there is more than one unbounded filler
//! ```
//!
//! Additionally, multiple concatenations with unbounded fillers must all have
//! their fillers aligned to the same end or have a concatenation without
//! an unbounded filler.
//!
//! ```text
//! // all allowed:
//! cc!(
//!     .., x;
//!     .., y;
//!     .., z;
//! );
//! cc!(
//!     x, ..;
//!     y, ..;
//!     z, ..;
//! );
//! // allowed, because the macro can infer the alignment by using the bitwidth
//! // of `var`
//! cc!(
//!     .., x;
//!     y, .., z;
//!     a, ..;
//!     var;
//! );
//! // disallowed, because the overlaps are ambiguous:
//! cc!(
//!     .., x;
//!     y, ..;
//! );
//! cc!(
//!     a, .., x;
//!     b, .., y;
//!     c, .., z;
//! );
//! ```
//!
//! It is technically possible to infer that ambiguous overlap could not occur
//! in this case, but this is still disallowed by the macro, and it is more
//! readable to just split the macro into two for both alignments.
//!
//! ```text
//! cc!(
//!     0x4567u16, .., 0x123u12;
//!     y0[..4], .., y1[..4];
//! );
//! // the above macro is semantically equivalent to these two macros combined:
//! cc!(
//!     0x4567u16, ..;
//!     y0[..4], ..;
//! );
//! cc!(
//!     .., 0x123u12;
//!     .., y1[..4];
//! );
//! ```
//!
//! ### Other Notes
//!
//! - If dynamic values are used in ranges or as variables multiple times, they
//!   should not use generator like behavior (e.x. using a function that changes
//!   its output between calls in `x[f()..=f()]`), or else you may get
//!   unexpected behavior. The parser and code generator treats identical
//!   strings like they produce the same value every time, and would call `f()`
//!   only once.
//! - In general, the macros use the `Bits::field` operation to copy different
//!   bitfields independently to a buffer, then field from the buffer to the
//!   sink components. When concatenations take the form `variable or constant
//!   with full range; var_1[..]; var_2[..]; var_3[..], ...`, the macros use
//!   `Bits::copy_` to directly copy without an intermediate buffer. This
//!   copy assigning mode cannot copy between `Bits` references that point to
//!   the same underlying storage, because it results in aliasing. Thus, trying
//!   to do something like `cc!(x; x)` results in the borrow checker complaining
//!   about macro generated variables within the macro being borrowed as both
//!   immutable and mutable. `cc!(x; x)` is semantically a no-op anyway, so it
//!   should not be used.
//! - The code generated by the macros takes special care to avoid panicking or
//!   overflowing. If a range is reversed (the start value is larger than the
//!   end value), the macro will will return a compile time error or `None` at
//!   runtime before trying to calculate something like `end - start`. The only
//!   way to overflow is to exceed `usize::MAX` in concatenation widths, or the
//!   individual arbitrary expressions entered into the macro overflow (e.x.
//!   `..(usize::MAX + 1)`).
//! - In case you want to see the code generated by a macro, you can use
//!   `awint_macro_internals` functions like `awint_macro_inlawi` and call it
//!   with the macro input as a string

extern crate proc_macro;
use awint_macro_internals::{
    awint_macro_cc, awint_macro_extawi, awint_macro_inlawi, unstable_native_inlawi_ty,
};
use proc_macro::TokenStream;

/// Specifies an `InlAwi` _type_ in terms of its bitwidth
#[proc_macro]
pub fn inlawi_ty(input: TokenStream) -> TokenStream {
    let w = input
        .to_string()
        .parse::<u128>()
        .expect("Input should parse as an unsigned integer");
    assert!(
        w != 0,
        "Tried to make an `InlAwi` type with an invalid bitwidth of 0"
    );
    unstable_native_inlawi_ty(w).parse().unwrap()
}

// TODO `0x1234.5678p4i32p16`
// prefix,integral,point,fractional,exPonent (can't use 'e' because it doesn't
// work in hexadecimal, maybe allow for decimal),signed,bitwidth,point position

// C4D
/// Copy Corresponding Concatenations of Components Dynamically. Takes
/// concatenations of components as an input, and copies bits of the source to
/// corresponding bits of the sinks. Returns `()` if the operation is
/// infallible, otherwise returns `Option<()>`. Returns `None` if component
/// indexes are out of bounds or if concatenation bitwidths mismatch. Performs
/// allocation in general, but will try to avoid allocation if the common
/// bitwdith can be determined statically, or if concatenations are all of
/// single components. See the [crate documentation](crate) for more.
#[proc_macro]
pub fn cc(input: TokenStream) -> TokenStream {
    match awint_macro_cc(&input.to_string()) {
        Ok(s) => s.parse().unwrap(),
        Err(s) => panic!("{}", s),
    }
}

/// A concatenations of components macro, additionally using the source value to
/// construct an `InlAwi`. See the [crate documentation](crate) for more.
#[proc_macro]
pub fn inlawi(input: TokenStream) -> TokenStream {
    match awint_macro_inlawi(&input.to_string()) {
        Ok(s) => s.parse().unwrap(),
        Err(s) => panic!("{}", s),
    }
}

/// A concatenations of components macro, additionally using the source value to
/// construct an `ExtAwi`. See the [crate documentation](crate) for more.
#[proc_macro]
pub fn extawi(input: TokenStream) -> TokenStream {
    match awint_macro_extawi(&input.to_string()) {
        Ok(s) => s.parse().unwrap(),
        Err(s) => panic!("{}", s),
    }
}

// I considered `bits!` and `bits_mut!` macros, but they tend to run into weird
// errors that would confuse people, better to have us decide and use the
// storage types directly
