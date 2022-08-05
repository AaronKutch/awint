use std::{cell::RefCell, num::NonZeroUsize};

use triple_arena::{ptr_struct, Arena};

use crate::common::Op;

ptr_struct!(PState);
thread_local!(static STATE_ARENA: RefCell<Arena<PState, State>> = RefCell::new(Arena::new()));

/// Enters a new `State` into the thread local arena
pub fn new_state(new_state: State) -> PState {
    STATE_ARENA.with(|f| f.borrow_mut().insert(new_state))
}

/// Enters a new `State` from the given components into the thread local arena
pub fn new_state_with(nzbw: NonZeroUsize, op: Op<PState>) -> PState {
    STATE_ARENA.with(|f| f.borrow_mut().insert(State { nzbw, op }))
}

/// Gets state pointed to by `p_state` from the thread local arena
pub fn get_state(p_state: PState) -> State {
    STATE_ARENA.with(|f| {
        f.borrow()
            .get(p_state)
            .expect("old `PState`s are probably being used after a call to `clear_state`")
            .clone()
    })
}

/// Clears the thread local state arena of all states and capacity
pub fn clear_thread_local_state() {
    // use `clear_and_shrink` because this function will be called to reduce
    // resource usage
    STATE_ARENA.with(|f| f.borrow_mut().clear_and_shrink())
}

/// Represents a single state that `mimick::Bits` is in at one point in time.
/// The operands point to other `State`s. `Bits`, `InlAwi`, and `ExtAwi` use
/// `Ptr`s to `States` in a thread local arena, so that they can change their
/// state without borrowing issues or mutating `States` (which could be used as
/// operands by other `States`).
#[derive(Hash, Clone, PartialEq, Eq, Debug)]
pub struct State {
    /// Bitwidth
    pub nzbw: NonZeroUsize,
    /// Operation
    pub op: Op<PState>,
}
