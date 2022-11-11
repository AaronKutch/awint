use std::{
    cell::{RefCell, RefMut},
    num::NonZeroUsize,
};

use triple_arena::{ptr_struct, Arena, Ptr};

use crate::common::Op;

#[cfg(debug_assertions)]
ptr_struct!(PState; PNode);

#[cfg(not(debug_assertions))]
ptr_struct!(PState(); PNode());

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
    pub prev_in_epoch: Option<PState>,
}

// TODO if we commit to stacklike epoch management then we could use something
// different than a general `Arena`

thread_local!(pub static STATE_ARENA: RefCell<Arena<PState, State>> = RefCell::new(Arena::new()));
thread_local!(pub static STATE_VISIT_GEN: RefCell<u64> = RefCell::new(0));
thread_local!(
    pub static EPOCH_STACK: RefCell<Vec<(u64, Option<PState>)>> = RefCell::new(vec![(0, None)])
);
thread_local!(pub static EPOCH_GEN: RefCell<u64> = RefCell::new(0));

/// During the lifetime of a `StateEpoch` struct, all thread local `State`s
/// created will be kept until the struct is dropped, in which case the capacity
/// for those states are reclaimed and their `PState`s invalidated. If mimick
/// types are using states from before the lifetime, variables taken only by
/// reference will retain their validity, but variables operated on by mutable
/// reference during the lifetime will probably be invalidated and cause panics
/// if used after the lifetime.
///
/// In most use cases, you should create a `StateEpoch` for the lifetime of a
/// group of mimicking types that are never used after being converted to
/// `OpDag` form or used by a function like `OpDag::add_group` or
/// `OpDag::graft`. Once in `OpDag` form, you do not have to worry about any
/// thread local weirdness.
///
/// # Panics
///
/// The lifetimes of `StateEpoch` structs should be stacklike, such that a
/// `StateEpoch` created during the lifetime of another `StateEpoch` should be
/// dropped before the older `StateEpoch` is dropped, otherwise a panic occurs.
#[derive(Debug)]
pub struct StateEpoch {
    this_epoch_gen: u64,
}

impl StateEpoch {
    pub fn new() -> Self {
        let this_epoch_gen = EPOCH_GEN.with(|f| {
            let gen = f.borrow().checked_add(1).unwrap();
            *f.borrow_mut() = gen;
            gen
        });
        EPOCH_STACK.with(|f| {
            f.borrow_mut().push((this_epoch_gen, None));
        });
        Self { this_epoch_gen }
    }
}

impl Drop for StateEpoch {
    fn drop(&mut self) {
        EPOCH_STACK.with(|f| {
            let mut epoch_stack: RefMut<Vec<(u64, Option<PState>)>> = f.borrow_mut();
            let top_gen = epoch_stack.last().unwrap().0;
            if top_gen == self.this_epoch_gen {
                // remove all the states associated with this epoch
                let mut last_state = epoch_stack.pop().unwrap().1;
                STATE_ARENA.with(|f| {
                    let mut a = f.borrow_mut();
                    while let Some(p_state) = last_state {
                        let state = a.remove(p_state).unwrap();
                        last_state = state.prev_in_epoch;
                    }
                })
            } else {
                panic!(
                    "when trying to drop the `StateEpoch` with generation {}, found that the \
                     `StateEpoch` with generation {} has not been dropped yet",
                    self.this_epoch_gen, top_gen
                );
            }
        });
    }
}

impl PState {
    /// Enters a new `State` from the given components into the thread local
    /// arena and registered for the current `StateEpoch`. Returns a `PState`
    /// referencing that `State`, which will only be removed when the current
    /// `StateEpoch` is dropped.
    pub fn new(nzbw: NonZeroUsize, op: Op<PState>) -> Self {
        STATE_ARENA.with(|f| {
            f.borrow_mut().insert_with(|p_this| State {
                nzbw,
                op,
                node_map: Ptr::invalid(),
                visit: STATE_VISIT_GEN.with(|f| *f.borrow()),
                prev_in_epoch: EPOCH_STACK.with(|f| {
                    let mut stack = f.borrow_mut();
                    // if there was a previous state in this epoch, record it for later chain
                    // freeing
                    let prev_in_epoch = stack.last().unwrap().1;
                    stack.last_mut().unwrap().1 = Some(p_this);
                    prev_in_epoch
                }),
            })
        })
    }

    /// Gets `State` pointed to by `self` from the thread local arena
    pub fn get_state(&self) -> Option<State> {
        STATE_ARENA.with(|f| f.borrow().get(*self).cloned())
    }

    pub fn get_nzbw(&self) -> NonZeroUsize {
        STATE_ARENA.with(|f| f.borrow().get(*self).unwrap().nzbw)
    }

    /// Set the auxiliary `visit` and `node_map` fields on the `State` pointed
    /// to by `self.
    pub fn set_state_aux(&self, visit: u64, node_map: PNode) -> Option<()> {
        STATE_ARENA.with(|f| {
            if let Some(state) = f.borrow_mut().get_mut(*self) {
                state.node_map = node_map;
                state.visit = visit;
                Some(())
            } else {
                None
            }
        })
    }
}

/// Gets a new visit generation
pub fn next_state_visit_gen() -> u64 {
    STATE_VISIT_GEN.with(|f| {
        let gen = f.borrow().checked_add(1).unwrap();
        *f.borrow_mut() = gen;
        gen
    })
}

/// Calls `clear_and_shrink` on the thread local state arena. Panics if there
/// are active `StateEpoch`s.
pub fn clear_thread_local_state() {
    EPOCH_STACK.with(|f| {
        if f.borrow().len() != 1 {
            panic!("called `clear_thread_local_state` when not all `StateEpoch`s are dropped")
        }
    });
    // use `clear_and_shrink` because this function will be called to reduce
    // absolute resource usage
    STATE_ARENA.with(|f| f.borrow_mut().clear_and_shrink())
}

impl Default for StateEpoch {
    fn default() -> Self {
        Self::new()
    }
}
