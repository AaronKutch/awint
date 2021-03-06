//! # DAG functionality for `awint`
//!
//! **NOTE**: this is extremely WIP
//!
//! Requires `std`
//!
//! Outside of the core functionality which will be useful for big integer
//! arithmetic in constrained environments, there is a secondary goal with this
//! system of `awint` crates to create a new kind of RTL description library
//! that is not a DSL but is rather plain Rust code that can be run normally.
//! This `awint_dag` crate supplies a "mimicking" `Bits` struct similar to
//! `awint_core::Bits`, except that it has purely lazy execution, creating a DAG
//! recording the order in which different `Bits` operations are applied. A
//! function with a signature containing entirely `Bits` references can have a
//! macro applied to it, which will run the function body with the lazy version
//! of `Bits` and calculate a DAG equivalent to the function. The function can
//! be called like normal and can have the typical compiler optimizations
//! applied, while the DAG can be inspected for more complicated things.

#![deny(unsafe_op_in_unsafe_fn)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::comparison_chain)]

pub mod common;
pub mod lowering;
pub mod mimick;
pub use mimick::primitive;

pub mod prelude {
    pub use crate::mimick::{Bits, ExtAwi, InlAwi};
}
