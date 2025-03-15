//! Note: macro docs are in the main `awint` crate

extern crate proc_macro;
use awint_macro_internals::{
    awint_macro_bits, awint_macro_cc, awint_macro_extawi, awint_macro_inlawi,
    unstable_native_inlawi_ty, awint_macro_awi,
};
use proc_macro::TokenStream;

// I can't get rustdoc to handle links in reexported macros at all

/// Specifies an [InlAwi](awint_macro_internals::awint_core::InlAwi) _type_
/// in terms of its bitwidth
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

// C4D
/// Copy Corresponding Concatenations of Components Dynamically.
///
/// Takes concatenations of components as an input, and copies bits of the source to
/// corresponding bits of the sinks. Returns `()` if the operation is
/// infallible, otherwise returns `Option<()>`. Returns `None` if component
/// indexes are out of bounds or if concatenation bitwidths mismatch. Performs
/// allocation in general, but will try to avoid allocation if the common
/// bitwdith can be determined statically, or if concatenations are all of
/// single components. See `awint::macro_docs` for more.
#[proc_macro]
pub fn cc(input: TokenStream) -> TokenStream {
    match awint_macro_cc(&input.to_string()) {
        Ok(s) => s.parse().unwrap(),
        Err(s) => panic!("{}", s),
    }
}

/// A concatenations of components macro, additionally using the source value to
/// construct an [InlAwi](awint_macro_internals::awint_core::InlAwi). See `awint::macro_docs` for more.
#[proc_macro]
pub fn inlawi(input: TokenStream) -> TokenStream {
    match awint_macro_inlawi(&input.to_string()) {
        Ok(s) => s.parse().unwrap(),
        Err(s) => panic!("{}", s),
    }
}

/// A concatenations of components macro, additionally using the source value to
/// construct an [ExtAwi](awint_macro_internals::awint_ext::ExtAwi). See `awint::macro_docs` for more.
#[proc_macro]
pub fn extawi(input: TokenStream) -> TokenStream {
    match awint_macro_extawi(&input.to_string()) {
        Ok(s) => s.parse().unwrap(),
        Err(s) => panic!("{}", s),
    }
}

/// A concatenations of components macro, additionally using the source value to
/// construct an [Awi](awint_macro_internals::awint_ext::Awi). See `awint::macro_docs` for more.
#[proc_macro]
pub fn awi(input: TokenStream) -> TokenStream {
    match awint_macro_awi(&input.to_string()) {
        Ok(s) => s.parse().unwrap(),
        Err(s) => panic!("{}", s),
    }
}

// We make the `bits` macro `&'static`, because making a relaxed `bits` or
// `bits_mut` macro typically leads to unoptimality and weird compiler errors.
// Users should use references from `extawi` or `inlawi` in any other case.

// TODO The only thing we might change is making the configuration
// `static_width: false` if `const` allocation is ever supported.

/// A concatenations of components macro, additionally using the source value to
/// construct a `&'static Bits`.
///
/// Requires `const_support` and some feature flags to work. See `awint::macro_docs` for more.
#[proc_macro]
pub fn bits(input: TokenStream) -> TokenStream {
    match awint_macro_bits(&input.to_string()) {
        Ok(s) => s.parse().unwrap(),
        Err(s) => panic!("{}", s),
    }
}
