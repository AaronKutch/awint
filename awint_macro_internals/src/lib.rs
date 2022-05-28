//! This crate exists because of limitations with `proc-macro` crates. We need
//! to be able to test errors returned from the code generation function while
//! also being able to test the macros themselves. This might also be reused by
//! people who have new storage types.

#![allow(clippy::needless_range_loop)]
// TODO need a refactor
#![allow(clippy::too_many_arguments)]

// TODO after refactor make everything private and find unused functions

// TODO eliminate buffer if source component variables do not have same text as
// any sink variable

mod bimap;
mod cc_macro;
mod component;
mod concatenation;
mod errors;
mod lowering;
mod misc;
mod names;
mod ranges;
mod token_stream;
mod token_tree;
mod lower_structs;

pub use bimap::*;
pub use cc_macro::*;
pub use component::*;
pub use concatenation::*;
pub use errors::*;
pub use lowering::*;
pub use misc::*;
pub use names::*;
pub use ranges::*;
pub use token_stream::*;
pub use token_tree::*;
pub use lower_structs::*;

// FIXME remove
pub fn code_gen(
    _input: &str,
    _specified_initialization: bool,
    _construct_fn: &str,
    _inlawi: bool,
    _return_source: bool,
) -> Result<String, String> {
    Err(String::new())
}
