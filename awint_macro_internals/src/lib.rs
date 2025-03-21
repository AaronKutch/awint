//! This crate exists because of limitations with `proc-macro` crates. We need
//! to be able to test errors returned from the code generation function while
//! also being able to test the macros themselves. This might also be reused by
//! people who have new storage types.
//!
//! This is also available as a hidden reexport from the main `awint` crate if
//! the "std" feature is enabled

#![allow(clippy::needless_range_loop)]
#![allow(clippy::manual_repeat_n)]
#![allow(clippy::comparison_chain)]
// TODO
#![allow(unexpected_cfgs)]
#![cfg_attr(feature = "const_support", feature(const_trait_impl))]

mod cc_macro;
mod component;
mod concatenation;
mod errors;
mod lower_structs;
mod lowering;
mod misc;
mod names;
mod ranges;
mod token_stream;
mod token_tree;

pub use awint_ext::{self, awint_core};
pub use cc_macro::*;
pub use component::*;
pub use concatenation::*;
pub use errors::*;
pub use lower_structs::*;
pub use lowering::*;
pub use misc::*;
pub use names::*;
pub use ranges::*;
pub use token_stream::*;
pub use token_tree::*;
pub use triple_arena;
#[cfg(feature = "debug")]
pub use triple_arena_render;

pub fn awint_macro_cc(input: &str) -> Result<String, String> {
    let code_gen = CodeGen {
        static_width: false,
        return_type: None,
        must_use: awint_must_use,
        static_construction_fn: awint_static_construction_fn,
        lit_construction_fn: awint_unreachable_construction_fn,
        construction_fn: cc_construction_fn,
        const_wrapper: identity_const_wrapper,
        fn_names: AWINT_FN_NAMES,
    };
    cc_macro(input, code_gen, AWINT_NAMES)
}

pub fn awint_macro_inlawi(input: &str) -> Result<String, String> {
    let code_gen = CodeGen {
        static_width: true,
        return_type: Some("InlAwi"),
        must_use: awint_must_use,
        static_construction_fn: awint_static_construction_fn,
        lit_construction_fn: awint_inlawi_lit_construction_fn,
        construction_fn: inlawi_construction_fn,
        const_wrapper: identity_const_wrapper,
        fn_names: AWINT_FN_NAMES,
    };
    cc_macro(input, code_gen, AWINT_NAMES)
}

pub fn awint_macro_extawi(input: &str) -> Result<String, String> {
    let code_gen = CodeGen {
        static_width: false,
        return_type: Some("ExtAwi"),
        must_use: awint_must_use,
        static_construction_fn: awint_static_construction_fn,
        lit_construction_fn: awint_extawi_lit_construction_fn,
        construction_fn: extawi_construction_fn,
        const_wrapper: identity_const_wrapper,
        fn_names: AWINT_FN_NAMES,
    };
    cc_macro(input, code_gen, AWINT_NAMES)
}

pub fn awint_macro_awi(input: &str) -> Result<String, String> {
    let code_gen = CodeGen {
        static_width: false,
        return_type: Some("Awi"),
        must_use: awint_must_use,
        static_construction_fn: awint_static_construction_fn,
        lit_construction_fn: awint_awi_lit_construction_fn,
        construction_fn: awi_construction_fn,
        const_wrapper: identity_const_wrapper,
        fn_names: AWINT_FN_NAMES,
    };
    cc_macro(input, code_gen, AWINT_NAMES)
}

pub fn awint_macro_bits(input: &str) -> Result<String, String> {
    let code_gen = CodeGen {
        static_width: true,
        return_type: Some("&'static Bits"),
        must_use: awint_must_use,
        static_construction_fn: awint_static_construction_fn,
        lit_construction_fn: awint_bits_lit_construction_fn,
        construction_fn: inlawi_construction_fn,
        const_wrapper: awint_bits_const_wrapper,
        fn_names: AWINT_FN_NAMES,
    };
    cc_macro(input, code_gen, AWINT_NAMES)
}
