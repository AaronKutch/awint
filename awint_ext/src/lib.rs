//! Externally allocated arbitrary width integers
//!
//! This crate contains storage types with external storage, `ExtAwi` and `Awi`,
//! to go along with `InlAwi` in the `awint_core` crate. This crate is separate
//! because it requires support for `alloc`. Also includes `FP` because it
//! practically requires allocation to use. This crate is intended to be used
//! through the main `awint` crate, available with the "alloc" feature.

#![cfg_attr(feature = "const_support", feature(const_mut_refs))]
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

#[doc(hidden)]
pub use awint_core;
#[doc(hidden)]
pub use awint_core::awint_internals;
mod awi_struct;
mod extawi;
mod fp_struct;
#[cfg(feature = "serde_support")]
mod serde;
pub(crate) mod string_internals;
pub use awi_struct::Awi;
pub use awint_core::{bw, Bits, InlAwi, OrdBits, SerdeError};
pub use extawi::ExtAwi;
pub use fp_struct::{FPType, FP};

/// Subset of `awint::awi`
pub mod awi {
    pub use awint_core::awi::*;
    pub use Option::{None, Some};
    pub use Result::{Err, Ok};

    pub use crate::{Awi, ExtAwi, FPType, FP};
}

/// Fixed point related items
pub mod fp {
    pub use super::fp_struct::{F32, F64};
    pub use crate::{FPType, FP};
}
