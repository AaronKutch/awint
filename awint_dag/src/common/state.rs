use std::{cell::RefCell, num::NonZeroUsize};

use awint_ext::{awint_internals::Location, bw};

use super::DummyDefault;
use crate::{
    common::Op,
    dag,
    triple_arena::{ptr_struct, Arena, Ptr},
    Lineage,
};

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
    pub visit: u64,
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

// TODO if we commit to stacklike epoch management then we could use something
// different than a general `Arena`

thread_local!(
    /// Contains the actual `State`s. Note that [PState]
    /// has additional methods to automatically manage this.
    pub static STATE_ARENA: RefCell<Arena<PState, State>> = RefCell::new(Arena::new())
);
thread_local!(
    /// The current visitation generation for algorithms, use `next_state_visit_gen`
    pub static STATE_VISIT_GEN: RefCell<u64> = RefCell::new(0)
);
thread_local!(
    /// The Epoch stack, with each layer having the generation, a free list pointer, and assertions
    pub static EPOCH_STACK: RefCell<Vec<(u64, Option<PState>, Assertions)>>
        = RefCell::new(vec![(0, None, Assertions::new())])
);
thread_local!(
    /// The current Epoch generation
    pub static EPOCH_GEN: RefCell<u64> = RefCell::new(0)
);

/// Gets a new visitation generation for algorithms on [STATE_ARENA]
pub fn next_state_visit_gen() -> u64 {
    STATE_VISIT_GEN.with(|f| {
        let gen = f.borrow().checked_add(1).unwrap();
        *f.borrow_mut() = gen;
        gen
    })
}

/// Registers `bit` to the assertions of the current epoch
#[track_caller]
pub fn register_assertion_bit(bit: dag::bool, location: Location) {
    // need a new bit to attach location data to
    let new_bit =
        dag::bool::from_state(PState::new(bw(1), Op::Copy([bit.state()]), Some(location)));
    EPOCH_STACK.with(|f| {
        f.borrow_mut().last_mut().unwrap().2.bits.push(new_bit);
    });
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

/// Manages the lifetimes and assertions of `State`s created by mimicking types.
///
/// During the lifetime of a `StateEpoch` struct, all thread local `State`s
/// created will be kept until the struct is dropped, in which case the capacity
/// for those states are reclaimed and their `PState`s invalidated. If mimick
/// types are using states from before the lifetime, variables taken only by
/// reference will retain their validity, but variables operated on by mutable
/// reference during the lifetime will probably be invalidated and cause panics
/// if used after the lifetime.
///
/// Additionally, assertion bits from [crate::mimick::assert],
/// [crate::mimick::assert_eq], [crate::mimick::Option::unwrap], etc are
/// associated with the newest level `StateEpoch` alive at the time they are
/// created. Use [StateEpoch::assertions] to acquire these.
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
            f.borrow_mut()
                .push((this_epoch_gen, None, Assertions::new()));
        });
        Self { this_epoch_gen }
    }

    /// Returns the generation of `self`, which is different for every
    /// `StateEpoch`
    pub fn gen(&self) -> u64 {
        self.this_epoch_gen
    }

    /// Returns the latest state, if any, associated with this Epoch
    pub fn latest_state(&self) -> Option<PState> {
        let mut res = None;
        EPOCH_STACK.with(|f| {
            for (i, layer) in f.borrow().iter().enumerate().rev() {
                if layer.0 == self.gen() {
                    res = layer.1;
                    break
                }
                if i == 0 {
                    // shouldn't be reachable even with leaks
                    unreachable!();
                }
            }
        });
        res
    }

    /// Gets the states associated with this Epoch (not including states from
    /// when sub-epochs are alive or from before this Epoch was created)
    pub fn states(&self) -> Vec<PState> {
        let mut res = vec![];
        EPOCH_STACK.with(|f| {
            for (i, layer) in f.borrow().iter().enumerate().rev() {
                if layer.0 == self.gen() {
                    let mut current_state = layer.1;
                    STATE_ARENA.with(|f| {
                        let a = f.borrow();
                        while let Some(p) = current_state {
                            res.push(p);
                            current_state = a[p].prev_in_epoch;
                        }
                    });
                    break
                }
                if i == 0 {
                    // shouldn't be reachable even with leaks
                    unreachable!();
                }
            }
        });
        res
    }

    /// Gets the assertions associated with this Epoch (not including assertions
    /// from when sub-epochs are alive or from before the this Epoch was
    /// created)
    pub fn assertions(&self) -> Assertions {
        let mut res = Assertions::new();
        EPOCH_STACK.with(|f| {
            for (i, layer) in f.borrow().iter().enumerate().rev() {
                if layer.0 == self.gen() {
                    res = layer.2.clone();
                    break
                }
                if i == 0 {
                    // shouldn't be reachable even with leaks
                    unreachable!();
                }
            }
        });
        res
    }
}

impl Drop for StateEpoch {
    fn drop(&mut self) {
        EPOCH_STACK.with(|f| {
            let mut epoch_stack = f.borrow_mut();
            let top_gen = epoch_stack.last().unwrap().0;
            if top_gen == self.gen() {
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
                    self.gen(),
                    top_gen
                );
            }
        });
    }
}

impl PState {
    /// Enters a new `State` from the given components into the thread local
    /// arena and registers it for the current `StateEpoch`. Returns a `PState`
    /// `Ptr` to it that will only be invalidated when the current `StateEpoch`
    /// is dropped.
    pub fn new(nzbw: NonZeroUsize, op: Op<PState>, location: Option<Location>) -> Self {
        STATE_ARENA.with(|f| {
            f.borrow_mut().insert_with(|p_this| State {
                nzbw,
                op,
                location,
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

    /// Gets the `State` pointed to by `self` from the thread local arena
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

impl Default for StateEpoch {
    fn default() -> Self {
        Self::new()
    }
}
