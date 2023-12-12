use std::num::NonZeroUsize;

use awint_ext::{
    awint_internals::{Location, USIZE_BITS},
    bw, Awi,
};

use crate::{
    common::Op,
    epoch::{get_nzbw_from_current_epoch, get_op_from_current_epoch, new_pstate_for_current_epoch},
    triple_arena::ptr_struct,
};

#[cfg(any(debug_assertions, feature = "gen_counter_for_pstate"))]
ptr_struct!(PState);

#[cfg(not(any(debug_assertions, feature = "gen_counter_for_pstate")))]
ptr_struct!(PState());

impl PState {
    /// Enters a new `State` from the given components into the thread local
    /// arena and registers it for the current `EpochCallback`.
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
        if let Op::Literal(lit) = self.get_op() {
            if lit.bw() == USIZE_BITS {
                return Some(lit.to_usize())
            }
        }
        None
    }

    pub fn try_get_as_awi(&self) -> Option<Awi> {
        if let Op::Literal(lit) = self.get_op() {
            Some(lit)
        } else {
            None
        }
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

impl DummyDefault for Awi {
    fn default() -> Self {
        Awi::zero(bw(1))
    }
}

impl DummyDefault for PState {
    fn default() -> Self {
        Default::default()
    }
}
