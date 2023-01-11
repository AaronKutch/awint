//! This crate compiles all the interfaces of `awint_core`, `awint_macros`, and
//! `awint_ext` (when the default "alloc" feature is enabled). Enabling the
//! "dag" feature flag also enables the `dag` module and a reexport of
//! `awint_dag`. There are also hidden developer reexports.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "const_support", feature(const_trait_impl))]

#[doc(hidden)]
pub use awint_core::awint_internals;
pub use awint_core::{bw, Bits, InlAwi, SerdeError};
#[cfg(feature = "dag")]
pub use awint_dag;
#[cfg(feature = "alloc")]
pub use awint_ext::{ExtAwi, FPType, FP};
#[doc(hidden)]
#[cfg(feature = "std")]
pub use awint_macro_internals;
pub use awint_macros::*;

/// Reexports all the regular arbitrary width integer structs, macros, common
/// enums, and most of `core::primitive::*`. This is useful for glob importing
/// everything or for when using the regular items in a context with structs
/// imported from `awint_dag`.
pub mod awi {
    #[cfg(not(feature = "alloc"))]
    pub use awint_core::awi::*;
    #[cfg(feature = "alloc")]
    pub use awint_ext::awi::*;
    pub use awint_macros::*;
    pub use Option::{None, Some};
    pub use Result::{Err, Ok};
}

/// Reexports all the mimicking versions of `awi` items
#[cfg(feature = "dag")]
pub mod dag {
    pub use awint_dag::{
        dag::*,
        mimick::{
            Option::{None, Some},
            Result::{Err, Ok},
        },
    };
    pub use awint_macros::*;
}

/// Reexports items defined within the `awint` crate system
pub mod prelude {
    pub use awint_core::{bw, Bits, InlAwi};
    #[cfg(feature = "alloc")]
    pub use awint_ext::{ExtAwi, FPType, FP};
    pub use awint_macros::*;
}
