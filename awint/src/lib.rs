//! This crate compiles all the interfaces of `awint_core`,
//! `awint_macros`, and `awint_ext` (when the default "alloc" feature is
//! enabled). Enabling the "dag" feature flag also enables the `dag_prelude` and
//! `dag` modules and a reexport of `awint_dag`.

#![cfg_attr(not(feature = "std"), no_std)]

#[doc(hidden)]
pub use awint_core::awint_internals;
pub use awint_core::{bw, Bits, InlAwi, SerdeError};
#[cfg(feature = "dag")]
pub use awint_dag;
#[cfg(feature = "alloc")]
pub use awint_ext::{ExtAwi, FPType, FP};
pub use awint_macros::*;

/// Reexports every user-intended macro, structure, and function except for
/// `SerdeError`.
pub mod prelude {
    pub use awint_core::{bw, Bits, InlAwi};
    #[cfg(feature = "alloc")]
    pub use awint_ext::{ExtAwi, FPType, FP};
    pub use awint_macros::*;
}

/// The same as `prelude` with some of the exact same functions and macros, but
/// the `awi` structs are swapped with their `dag` equivalents
#[cfg(feature = "dag")]
pub mod dag_prelude {
    pub use awint_core::bw;
    pub use awint_dag::{Bits, ExtAwi, InlAwi};
    pub use awint_macros::*;
}

/// Contains all the regular arbitrary width integer structs and most of
/// `core::primitive::*`. This is useful when using the regular structs in a
/// context with structs imported from `awint_dag`.
pub mod awi {
    #[cfg(not(feature = "alloc"))]
    pub use awint_core::awi::*;
    #[cfg(feature = "alloc")]
    pub use awint_ext::awi::*;
    pub use Option::{None, Some};
    pub use Result::{Err, Ok};
}

/// Contains all the mimicking arbitrary width integer structs and the mimicking
/// versions of `core::primitive::*`, in case of using the DAG constructing
/// structs in a regular arbitrary width integer context
#[cfg(feature = "dag")]
pub mod dag {
    pub use awint_dag::{
        mimick::{Bits, ExtAwi, InlAwi},
        primitive::*,
    };
}
