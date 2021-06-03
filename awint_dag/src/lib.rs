//! # DAG functionality for `awint`
//!
//! **NOTE**: this is extremely WIP
//!
//! Requires a global allocator and atomics support

mod bits;
mod op;
pub mod primitive;

use std::{num::NonZeroUsize, rc::Rc};

pub use bits::Bits;
pub use op::*;

/// A trait for a state lineage in a Directed Acyclic Graph of `Bits`
/// operations. Every `Bits` operation has mutable and/or immutable references
/// to different `Bits`. The state of the mutable `Bits` are updated by creating
/// an `Op` node with `TriPtr`s to the states of all `Bits` involved, then
/// inserting this node into the arena and setting the current state to the new
/// `TriPtr` that the insertion returned. This results in the DAG being
/// structured with a `Lineage` for every `Bits` or DAG primitive.
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
