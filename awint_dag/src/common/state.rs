use std::{cell::RefCell, num::NonZeroUsize};

use triple_arena::{ptr_struct, Arena, Ptr};

use crate::common::Op;

ptr_struct!(PState);
ptr_struct!(PNode);

thread_local!(static STATE_ARENA: RefCell<Arena<PState, State>> = RefCell::new(Arena::new()));
thread_local!(static STATE_VISIT_GEN: RefCell<u64> = RefCell::new(0));

/// Enters a new `State` from the given components into the thread local arena
pub fn new_state_with(nzbw: NonZeroUsize, op: Op<PState>) -> PState {
    STATE_ARENA.with(|f| {
        f.borrow_mut().insert(State {
            nzbw,
            op,
            node_map: Ptr::invalid(),
            visit: STATE_VISIT_GEN.with(|f| *f.borrow()),
        })
    })
}

/// Gets state pointed to by `p_state` from the thread local arena
pub fn get_state(p_state: PState) -> Option<State> {
    STATE_ARENA.with(|f| f.borrow().get(p_state).cloned())
}

pub fn next_state_visit_gen() -> u64 {
    STATE_VISIT_GEN.with(|f| {
        let gen = f.borrow().checked_add(1).unwrap();
        *f.borrow_mut() = gen;
        gen
    })
}

pub fn set_state_node_map(p_state: PState, visit: u64, node_map: PNode) -> Option<()> {
    STATE_ARENA.with(|f| {
        if let Some(state) = f.borrow_mut().get_mut(p_state) {
            state.node_map = node_map;
            state.visit = visit;
            Some(())
        } else {
            None
        }
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
    /// Used to avoid needing hashmaps
    pub node_map: PNode,
    /// Used in algorithms for DFS tracking and to allow multiple DAG
    /// constructions from same nodes
    pub visit: u64,
}
