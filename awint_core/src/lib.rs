//! Arbitrary width integers library
//!
//! This is the core library of the `awint` system of crates. This crate is
//! strictly `no-std` and `no-alloc`, not even requiring an allocator to be
//! compiled. This crate supplies the `Bits` reference type and the `InlAwi`
//! storage type.
//!
//! Almost all fallible functions in this crate returns a handleable `Option` or
//! `Result`. The only exceptions are some `core::ops` implementations and the
//! `bw` function.

#![cfg_attr(feature = "const_support", feature(const_maybe_uninit_as_mut_ptr))]
#![cfg_attr(feature = "const_support", feature(const_mut_refs))]
#![cfg_attr(feature = "const_support", feature(const_ptr_read))]
#![cfg_attr(feature = "const_support", feature(const_ptr_write))]
#![cfg_attr(feature = "const_support", feature(const_intrinsic_copy))]
#![cfg_attr(feature = "const_support", feature(const_slice_from_raw_parts))]
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
#![deny(unsafe_op_in_unsafe_fn)]

pub use awint_internals::{bw, SerdeError};

pub(crate) mod data;
pub use data::{Bits, InlAwi};

mod logic;

pub mod prelude {
    pub use crate::{bw, Bits, InlAwi};
}
