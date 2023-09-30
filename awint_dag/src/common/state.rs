use std::num::{NonZeroU64, NonZeroUsize};

use awint_ext::awint_internals::{Location, USIZE_BITS};

use super::{
    epoch::{get_nzbw_from_current_epoch, get_op_from_current_epoch, new_pstate_for_current_epoch},
    DummyDefault,
};
use crate::{common::Op, dag, triple_arena::ptr_struct, Lineage};

#[cfg(debug_assertions)]
ptr_struct!(PState; PNode);

#[cfg(not(debug_assertions))]
ptr_struct!(PState(); PNode());

impl DummyDefault for PNode {
    fn default() -> Self {
        Default::default()
    }
}

/// Represents a single state that `mimick::Bits` is in at one point in time.
/// The operands point to other `State`s. `Bits`, `InlAwi`, and `ExtAwi` use
/// `Ptr`s to `States` in a thread local arena, so that they can change their
/// state without borrowing issues or mutating `States` (which could be used as
/// operands by other `States`).
#[derive(Debug, Clone)]
pub struct State {
    /// Bitwidth
    pub nzbw: NonZeroUsize,
    /// Operation
    pub op: Op<PState>,
    /// Location where this state is derived from
    pub location: Option<Location>,
    /// Used to avoid needing hashmaps
    pub node_map: PNode,
    /// Used in algorithms for DFS tracking and to allow multiple DAG
    /// constructions from same nodes
    pub visit: NonZeroU64,
    pub prev_in_epoch: Option<PState>,
}

#[derive(Debug, Clone)]
pub struct Assertions {
    pub bits: Vec<dag::bool>,
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
