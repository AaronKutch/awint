//! This crate compiles all the interfaces of `awint_core`, `awint_ext`, and
//! `awint_macros`.

#![cfg_attr(not(feature = "std"), no_std)]

pub use awint_core::{bw, Bits, InlAwi, SerdeError};
#[cfg(feature = "dag")]
pub use awint_dag;
#[cfg(feature = "alloc")]
pub use awint_ext::{ExtAwi, FPType, FP};
pub use awint_macros::*;

/// Reexports every user-intended macro, structure, and function except for
/// `SerdeError`.
pub mod prelude {
    pub use awint_macros::*;

    pub use crate::{bw, cc, Bits, InlAwi};
    #[cfg(feature = "alloc")]
    pub use crate::{ExtAwi, FPType, FP};
}

#[cfg(feature = "awint_dag")]
pub mod dag_prelude {
    pub use awint_core::bw;
    pub use awint_dag::prelude::*;
    pub use awint_macros::*;
}
