use std::{num::NonZeroUsize, rc::Rc};

use crate::mimick::Op;

/// A trait for a state lineage in a Directed Acyclic Graph of `Bits`
/// operations. Every `Bits` operation has mutable and/or immutable references
/// to different `Bits`. The state of the mutable `Bits` are updated by
/// allocating an `Rc<Op>` node with pointers to the states of all `Bits`
/// involved, then setting the current `op_mut` state to that new node. This
/// results in the DAG being structured with a `Lineage` for every Arbitrary
/// width integer, `Bits`, or DAG primitive.
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
