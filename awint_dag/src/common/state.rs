use std::{fmt, num::NonZeroUsize, rc::Rc};

use crate::common::Op;

/// Represents a single state that `mimick::Bits` is in at one point in time.
/// The operands point to other `State`s. `Bits`, `InlAwi`, and `ExtAwi` use
/// `Rc` pointers to `States`, so that they can change their state without
/// borrowing issues or mutating `States` (which could be used as operands by
/// other `States`).
#[derive(Hash, Default, PartialEq, Eq)]
pub struct State {
    /// Bitwidth
    pub nzbw: Option<NonZeroUsize>,
    /// Operation
    pub op: Op<RcState>,
}

/// Abstracts around the reference counting mechanism. Defines equality using
/// `Rc::ptr_eq`.
#[allow(clippy::derive_hash_xor_eq)] // If `ptr_eq` is true, the `Hash` defined on `Rc` also agrees
#[derive(Hash, Default, Eq)]
pub struct RcState {
    state: Rc<State>,
}

impl Clone for RcState {
    fn clone(&self) -> Self {
        Self {
            state: Rc::clone(&self.state),
        }
    }
}

impl PartialEq for RcState {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.state, &other.state)
    }
}

impl RcState {
    pub fn new(nzbw: Option<NonZeroUsize>, op: Op<RcState>) -> RcState {
        RcState {
            state: Rc::new(State { nzbw, op }),
        }
    }

    pub fn nzbw(&self) -> Option<NonZeroUsize> {
        self.state.nzbw
    }

    pub fn op(&self) -> &Op<RcState> {
        &self.state.op
    }
}

impl fmt::Debug for RcState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.state)
    }
}

impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // do not include `ops` field, if the `Rc`s are in a web it results in
        // exponential growth
        f.debug_struct("State")
            .field("nzbw", &self.nzbw)
            .field("op", &self.op.operation_name())
            .finish()
    }
}
