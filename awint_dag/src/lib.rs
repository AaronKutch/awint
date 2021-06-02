//! # DAG functionality for `awint`
//!
//! **NOTE**: this is extremely WIP
//!
//! Requires a global allocator and atomics support

pub mod bits;
pub(crate) mod op;
//mod state_machine;
pub mod primitive;

use std::num::NonZeroUsize;

pub use bits::Bits;
pub use op::*;
use triple_arena::{Arena, TriPtr};

/// A trait for a state lineage in a Directed Acyclic Graph of `Bits`
/// operations. Every `Bits` operation has mutable and/or immutable references
/// to different `Bits`. The state of the mutable `Bits` are updated by creating
/// an `Op` node with `TriPtr`s to the states of all `Bits` involved, then
/// inserting this node into the arena and setting the current state to the new
/// `TriPtr` that the insertion returned. This results in the DAG being
/// structured with a `Lineage` for every `Bits` or DAG primitive.
pub trait Lineage {
    /// Returns the latest state of `self`
    fn state(&self) -> TriPtr;

    /// Returns a reference to an arena containing all `Op`s performed on `self`
    fn ops(&self) -> &Arena<Op>;

    /// Returns the bitwidth of `self` as a `NonZeroUsize`
    fn nzbw(&self) -> NonZeroUsize;

    /// Returns the bitwidth of `self` as a `usize`
    fn bw(&self) -> usize {
        self.nzbw().get()
    }
}
