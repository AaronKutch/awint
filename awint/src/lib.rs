//! This crate compiles all the interfaces of `awint_core`, `awint_ext`, and
//! `awint_macros`.

#![cfg_attr(not(feature = "std"), no_std)]

pub use awint_core::prelude::*;
#[cfg(feature = "alloc")]
pub use awint_ext::prelude::*;
pub use awint_macros::*;

pub mod prelude {
    pub use crate::*;
}
