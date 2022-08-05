use std::{cell::RefCell, fmt, num::NonZeroUsize};

use triple_arena::{ptr_struct, Arena};

use crate::common::Op;

/// Represents a single state that `mimick::Bits` is in at one point in time.
/// The operands point to other `State`s. `Bits`, `InlAwi`, and `ExtAwi` use
/// `Rc` pointers to `States`, so that they can change their state without
/// borrowing issues or mutating `States` (which could be used as operands by
/// other `States`).
#[derive(Hash, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// Bitwidth
    pub nzbw: Option<NonZeroUsize>,
    /// Operation
    pub op: Op<PState>,
}

ptr_struct!(PState);
thread_local!(static STATE_ARENA: RefCell<Arena<PState, State>> = RefCell::new(Arena::new()));

pub fn get_state(p_state: PState) -> State {
    STATE_ARENA.with(|f| f.borrow().get(p_state).unwrap().clone())
}

pub fn new_state(new_state: State) -> PState {
    STATE_ARENA.with(|f| f.borrow_mut().insert(new_state))
}

pub fn new_state_with(nzbw: Option<NonZeroUsize>, op: Op<PState>) -> PState {
    STATE_ARENA.with(|f| f.borrow_mut().insert(State { nzbw, op }))
}

/// Abstracts around the reference counting mechanism. Defines equality using
/// pointer equality.
/*#[allow(clippy::derive_hash_xor_eq)] // If `ptr_eq` is true, the `Hash` also agrees
#[derive(Hash, Default, PartialEq, Eq, Clone)]
pub struct PState {
    p_state: PState,
}*/

/*impl Clone for PState {
    fn clone(&self) -> Self {
        STATE_ARENA.with(|f| f.borrow_mut().get_mut(self.p_state).unwrap().rc += 1);
        Self { p_state: self.p_state }
    }
}*/

/*impl Drop for PState {
    fn drop(&mut self) {
        STATE_ARENA.with(|f| {
            let rc = f.borrow_mut().get_mut(self.p_state).unwrap_or_else(|| panic!("could not find state while dropping PState")).rc;
            f.borrow_mut().get_mut(self.p_state).unwrap().rc = rc.checked_sub(1).unwrap_or_else(|| panic!("decremented zero rc while dropping PState"));
        });
    }
}*/

/*impl PState {
    pub fn new(nzbw: Option<NonZeroUsize>, op: Op<PState>) -> PState {
        PState {
            p_state: STATE_ARENA.with(|f| f.borrow_mut().insert(State { nzbw, op })),
        }
    }

    /// If other things are needed, `no_rc_inc_state` should be used instead to
    /// avoid incurring extra lookups
    pub fn nzbw(&self) -> Option<NonZeroUsize> {
        let res = STATE_ARENA.with(|f| f.borrow().get(self.p_state).unwrap().nzbw);
        res
    }

    /// Gets the `State` without incrementing the reference count
    pub fn no_rc_inc_state(&self) -> State {
        let res = STATE_ARENA.with(|f| {
            let borrow = f.borrow();
            let state = borrow.get(self.p_state).unwrap();
            State {
                nzbw: state.nzbw,
                op: state.op,
            }
        });
        res
    }
}

impl fmt::Debug for PState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.p_state)
    }
}*/

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
