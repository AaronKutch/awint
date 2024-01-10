//! # DAG functionality for `awint`
//!
//! **NOTE**: This crate acts as the backend for the `starlight` crate, but
//! others may use it for their own projects as well.
//!
//! **NOTE**: there is one significant wart, being that the concatenation macros
//! in `dag::*` mode may not be no-ops if a `None` is returned. Use `unwrap`s
//! on the outputs of the concatenation macros and check assertions for now.
//!
//! This crate is intended to be used as a reexport from the `awint` crate with
//! the "dag" feature flag enabled.
//!
//! Outside of the core functionality which will be useful for big integer
//! arithmetic in constrained environments, there is a secondary goal with this
//! system of `awint` crates to create a new kind of RTL description library
//! that is not a DSL but is rather plain Rust code that can be run normally.
//! This `awint_dag` crate supplies a "mimicking" structs with the same names as
//! their counterparts in `awint::awi::*`, the difference being that they
//! have lazy execution, creating a DAG recording the order in which
//! different `Bits` operations are applied.
//!
//! See the documentation of the `starlight` crate for examples of usage.
//!
//! ## Important Notes
//!
//! - If you try to use a mimicking `bool` in an `if` statement or with some
//!   binary operations you will get an error. `Bits::mux_` can be used to
//!   conditionally merge the result of two computational paths instead of using
//!   `if` statements.
//! - The mimicking types have an extra `opaque` constructor kind that has no
//!   definitive bit pattern. This can be accessed through `Bits::opaque_`,
//!   `*Awi::opaque(w)`, and in macros like `inlawi!(opaque: ..8)`. This is
//!   useful for placeholder values in algorithms that prevents evaluation from
//!   doing anything with the sink tree of these values.
//! - The macros from `awint_dag` use whatever `usize`, `*Awi`, and `Bits`
//!   structs are imported in their scope. If you are mixing regular and
//!   mimicking types and are getting name collisions in macros, you can glob
//!   import `awint::awi::*` or `awint::dag::*` in the same scope as the macro
//!   (or add an extra block scope around the macro to glob import in), which
//!   should fix the errors.
//! - There are generation counters on the `PState`s that are enabled when debug
//!   assertions are on
//! - In long running programs that are generating a lot of separate DAGs, you
//!   should use things such as `starlight::Epoch`s for each one, so that thread
//!   local data is cleaned up

#![deny(unsafe_op_in_unsafe_fn)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::comparison_chain)]
#![cfg_attr(feature = "try_support", feature(try_trait_v2))]
#![cfg_attr(feature = "try_support", feature(try_trait_v2_residual))]
#![cfg_attr(feature = "try_support", feature(never_type))]

mod common;
pub mod mimick;
pub use awint_ext::awint_internals::{location, Location};
pub use awint_macro_internals::triple_arena;
#[cfg(feature = "debug")]
pub use awint_macro_internals::triple_arena_render;
pub use common::{
    epoch, ConcatFieldsType, ConcatType, DummyDefault, EAwi, EvalResult, Lineage, Op, PState,
};
// export needed by the macros
#[doc(hidden)]
pub use mimick::assertion::{internal_assert, internal_assert_eq, internal_assert_ne, stringify};
pub use mimick::primitive;
pub use smallvec;

/// All mimicking items
pub mod dag {
    pub use awint_ext::bw;

    pub use crate::{
        mimick::{
            assert, assert_eq, assert_ne, Awi, Bits, ExtAwi, InlAwi, Option,
            Option::{None, Some},
            Result,
            Result::{Err, Ok},
        },
        primitive::*,
    };
}
