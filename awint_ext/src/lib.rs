//! Externally allocated arbitrary width integers
//!
//! This crate contains another storage type called `ExtAwi` to go along with
//! `InlAwi` in the `awint_core` crate. This crate is separate because it
//! requires support for `alloc`.

#![feature(const_fn_transmute)]
#![feature(const_mut_refs)]
#![feature(vec_into_raw_parts)]
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
#![deny(unsafe_op_in_unsafe_fn)]

extern crate alloc;

mod extawi;
#[cfg(feature = "serde_support")]
mod serde;
mod strings;

pub use extawi::ExtAwi;
pub use strings::{bits_to_string_radix, bits_to_vec_radix};

pub mod prelude {
    pub use crate::{bits_to_string_radix, bits_to_vec_radix, ExtAwi};
}
