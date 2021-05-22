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

#![feature(const_fn_transmute)]
#![feature(const_maybe_uninit_as_ptr)]
#![feature(const_mut_refs)]
#![feature(const_ptr_offset)]
#![feature(const_ptr_read)]
#![feature(const_panic)]
#![feature(const_ptr_write)]
#![feature(const_intrinsic_copy)]
#![feature(const_slice_from_raw_parts)]
#![feature(const_swap)]
#![feature(const_raw_ptr_deref)]
#![feature(const_option)]
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
    pub use crate::{bw, Bits, InlAwi, SerdeError};
}
