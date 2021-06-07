//! This crate compiles all the interfaces of `awint_core`, `awint_ext`, and
//! `awint_macros`.

#![cfg_attr(not(feature = "std"), no_std)]

pub use awint_core::prelude::*;
#[cfg(feature = "awint_dag")]
pub use awint_dag;
#[cfg(feature = "alloc")]
pub use awint_ext::prelude::*;
pub use awint_macros::*;

pub mod prelude {
    pub use crate::*;
}

#[cfg(feature = "awint_dag")]
pub mod dag_prelude {
    pub use awint_core::bw;
    pub use awint_dag::{Bits, ExtAwi, InlAwi};
    pub use awint_macros::*;
}
