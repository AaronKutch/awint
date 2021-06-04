//! # DAG functionality for `awint`
//!
//! **NOTE**: this is extremely WIP
//!
//! Requires `alloc`
//!
//! Outside of the core functionality which will be useful for big integer
//! arithmetic in constrained environments, there is a secondary goal with this
//! system of `awint` crates to create a new kind of RTL description library
//! that is not a DSL but is rather plain Rust code that can be run normally.
//! This `awint_dag` crate, supplies a `Bits` struct similar to
//! `awint_core::Bits`, except that it has purely lazy execution, creating a DAG
//! recording the order in which different `Bits` operations are applied. A
//! function with a signature containing entirely `Bits` references can have a
//! macro applied to it, which will run the function body with the lazy version
//! of `Bits` and calculate a DAG equivalent to the function. The function can
//! be called like normal and can have the typical compiler optimizations
//! applied, while the DAG can be inspected for more complicated things.

#![no_std]
extern crate alloc;
mod awi;
mod bits;
mod op;
pub mod primitive;

use alloc::rc::Rc;
use core::num::NonZeroUsize;

pub use awi::{ExtAwi, InlAwi};
pub use bits::Bits;
pub use op::*;

/// A trait for a state lineage in a Directed Acyclic Graph of `Bits`
/// operations. Every `Bits` operation has mutable and/or immutable references
/// to different `Bits`. The state of the mutable `Bits` are updated by
/// allocating an `Rc<Op>` node with pointers to the states of all `Bits`
/// involved, then setting the current `op_mut` state to that new node. This
/// results in the DAG being structured with a `Lineage` for every Arbitrary
/// width integer, `Bits` or DAG primitive.
pub trait Lineage {
    /// Returns the bitwidth of `self` as a `NonZeroUsize`
    fn nzbw(&self) -> NonZeroUsize;

    /// Returns the bitwidth of `self` as a `usize`
    fn bw(&self) -> usize {
        self.nzbw().get()
    }

    /// Returns a clone of the latest `Op` that calculated `self`
    fn op(&self) -> Rc<Op>;

    /// Returns a mutable reference to the latest `Op` that calculated `self`
    fn op_mut(&mut self) -> &mut Rc<Op>;

    /// Update the latest `Op` of `self` with `op`
    fn update(&mut self, op: Op) {
        *self.op_mut() = Rc::new(op);
    }
}
