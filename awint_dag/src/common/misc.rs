use std::num::NonZeroUsize;

use awint_ext::{
    awint_internals::{Location, USIZE_BITS},
    bw, ExtAwi,
};

use crate::{
    common::Op,
    epoch::{get_nzbw_from_current_epoch, get_op_from_current_epoch, new_pstate_for_current_epoch},
    triple_arena::ptr_struct,
};

#[cfg(debug_assertions)]
ptr_struct!(PState; PNode);

#[cfg(not(debug_assertions))]
ptr_struct!(PState(); PNode());

ptr_struct!(PNote);

impl DummyDefault for PNode {
    fn default() -> Self {
        Default::default()
    }
}

impl PState {
    /// Enters a new `State` from the given components into the thread local
    /// arena and registers it for the current `StateEpoch`. Returns a `PState`
    /// `Ptr` to it that will only be invalidated when the current `StateEpoch`
    /// is dropped.
    pub fn new(nzbw: NonZeroUsize, op: Op<PState>, location: Option<Location>) -> Self {
        new_pstate_for_current_epoch(nzbw, op, location)
    }

    pub fn get_nzbw(&self) -> NonZeroUsize {
        get_nzbw_from_current_epoch(*self)
    }

    pub fn get_op(&self) -> Op<PState> {
        get_op_from_current_epoch(*self)
    }

    pub fn try_get_as_usize(&self) -> Option<usize> {
        if let Op::Literal(ref lit) = self.get_op() {
            if lit.bw() == USIZE_BITS {
                return Some(lit.to_usize())
            }
        }
        None
    }
}

/// A trait for mimicking structs that allows access to the internal state
pub trait Lineage {
    fn state_nzbw(&self) -> NonZeroUsize {
        self.state().get_nzbw()
    }

    /// Get a reference to the `State` of `self`
    fn state(&self) -> PState;
}

/// Some types can't implement `Default`, this is a `Default`-like trait for
/// placeholder values
#[doc(hidden)]
pub trait DummyDefault {
    fn default() -> Self;
}

impl DummyDefault for NonZeroUsize {
    fn default() -> Self {
        bw(1)
    }
}

impl DummyDefault for ExtAwi {
    fn default() -> Self {
        ExtAwi::zero(bw(1))
    }
}

impl DummyDefault for PState {
    fn default() -> Self {
        Default::default()
    }
}

#[derive(Debug, Clone)]
pub struct Assertions {
    pub bits: Vec<crate::dag::bool>,
}

impl Assertions {
    pub fn new() -> Self {
        Self { bits: vec![] }
    }

    pub fn states(&self) -> impl Iterator<Item = PState> + '_ {
        self.bits.iter().map(|bit| bit.state())
    }
}

impl Default for Assertions {
    fn default() -> Self {
        Self::new()
    }
}
