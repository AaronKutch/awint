//! Arbitrary width integers library
//!
//! This is the core library of the `awint` system of crates. This crate is
//! strictly `no-std` and `no-alloc`, not even requiring an allocator to be
//! compiled. This crate supplies the `Bits` reference type and the `InlAwi`
//! storage type.

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

pub mod prelude {
    pub use crate::{bw, Bits, InlAwi};
}

/// Subset of `awint::awi`
pub mod awi {
    // everything except for `char`, `str`, `f32`, and `f64`
    pub use core::primitive::{
        bool, i128, i16, i32, i64, i8, isize, u128, u16, u32, u64, u8, usize,
    };

    pub use crate::{Bits, InlAwi};
}
