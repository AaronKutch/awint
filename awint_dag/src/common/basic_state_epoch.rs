/// An epoch management struct used for tests and examples.
use std::{
    cell::RefCell,
    mem,
    num::{NonZeroU64, NonZeroUsize},
    thread::panicking,
};

use awint_ext::{awint_internals::Location, bw};

use super::{
    epoch::{EpochCallback, EpochKey},
    Assertions,
};
use crate::{
    common::Op,
    dag,
    triple_arena::{Arena, Ptr},
    Lineage, PNode, PState,
};

/// Represents a single state that `mimick::Bits` is in at one point in time.
/// The operands point to other `State`s. `Bits` and `*Awi` use `Ptr`s to
/// `States` in a thread local arena, so that they can change their
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

#[derive(Default)]
struct EpochData {
    key: EpochKey,
    assertions: Assertions,
    prev_in_epoch: Option<PState>,
}

struct TopEpochData {
    /// Contains the actual `State`s
    arena: Arena<PState, State>,
    /// The top level `EpochData`
    data: EpochData,
    /// Visit number
    visit: NonZeroU64,
    /// If the top level is active
    active: bool,
}

impl TopEpochData {
    pub fn new() -> Self {
        Self {
            arena: Arena::new(),
            data: EpochData::default(),
            visit: NonZeroU64::new(2).unwrap(),
            active: false,
        }
    }
}

thread_local!(
    /// The `TopEpochData`. We have this separate from `EPOCH_DATA_STACK` in the
    /// first place to minimize the assembly needed to access the data.
    static EPOCH_DATA_TOP: RefCell<TopEpochData> = RefCell::new(TopEpochData::new());

    /// Stores data for epochs lower than the current one
    static EPOCH_DATA_STACK: RefCell<Vec<EpochData>> = RefCell::new(vec![]);
);

#[doc(hidden)]
pub fn _callback() -> EpochCallback {
    fn new_pstate(nzbw: NonZeroUsize, op: Op<PState>, location: Option<Location>) -> PState {
        EPOCH_DATA_TOP.with(|top| {
            let mut top = top.borrow_mut();
            let visit = top.visit;
            let prev_in_epoch = top.data.prev_in_epoch;
            let mut outer_p_this = None;
            top.arena.insert_with(|p_this| {
                outer_p_this = Some(p_this);
                State {
                    nzbw,
                    op,
                    location,
                    node_map: Ptr::invalid(),
                    visit,
                    prev_in_epoch,
                }
            });
            top.data.prev_in_epoch = outer_p_this;
            outer_p_this.unwrap()
        })
    }
    fn register_assertion_bit(bit: dag::bool, location: Location) {
        EPOCH_DATA_TOP.with(|top| {
            let mut top = top.borrow_mut();
            let visit = top.visit;
            let prev_in_epoch = top.data.prev_in_epoch;
            let mut outer_p_this = None;
            top.arena.insert_with(|p_this| {
                outer_p_this = Some(p_this);
                State {
                    nzbw: bw(1),
                    op: Op::Assert([bit.state()]),
                    location: Some(location),
                    node_map: Ptr::invalid(),
                    visit,
                    prev_in_epoch,
                }
            });
            top.data.prev_in_epoch = outer_p_this;
            top.data.assertions.bits.push(bit);
        });
    }
    fn get_nzbw(p_state: PState) -> NonZeroUsize {
        EPOCH_DATA_TOP.with(|top| {
            let top = top.borrow();
            top.arena.get(p_state).unwrap().nzbw
        })
    }
    fn get_op(p_state: PState) -> Op<PState> {
        EPOCH_DATA_TOP.with(|top| {
            let top = top.borrow();
            top.arena.get(p_state).unwrap().op.clone()
        })
    }
    EpochCallback {
        new_pstate,
        register_assertion_bit,
        get_nzbw,
        get_op,
    }
}

/// Manages the lifetimes and assertions of `State`s created by mimicking types.
///
/// During the lifetime of a `StateEpoch` struct, all thread local `State`s
/// created will be kept until the struct is dropped, in which case the capacity
/// for those states are reclaimed and their `PState`s are invalidated.
///
/// Additionally, assertion bits from [crate::mimick::assert],
/// [crate::mimick::assert_eq], [crate::mimick::Option::unwrap], etc are
/// associated with the top level `StateEpoch` alive at the time they are
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
///
/// Using `mem::forget` or similar on a `StateEpoch` will leak `State`s and
/// cause them to not be cleaned up, and will also likely cause panics because
/// of the stack requirement.
#[derive(Debug)]
pub struct StateEpoch {
    key: EpochKey,
}

impl Drop for StateEpoch {
    fn drop(&mut self) {
        // prevent invoking recursive panics and a buffer overrun
        if !panicking() {
            // unregister callback
            self.key.pop_off_epoch_stack();
            EPOCH_DATA_TOP.with(|top| {
                let mut top = top.borrow_mut();
                // remove all the states associated with this epoch
                let mut last_state = top.data.prev_in_epoch;
                while let Some(p_state) = last_state {
                    let state = top.arena.remove(p_state).unwrap();
                    last_state = state.prev_in_epoch;
                }
                // move the top of the stack to the new top
                let new_top = EPOCH_DATA_STACK.with(|stack| {
                    let mut stack = stack.borrow_mut();
                    stack.pop()
                });
                if let Some(new_data) = new_top {
                    top.data = new_data;
                } else {
                    top.active = false;
                    top.data = EpochData::default();
                    // if there is considerable capacity, clear it (else we do not want to incur
                    // allocations for rapid state epoch creation)
                    if top.arena.capacity() > 64 {
                        top.arena.clear_and_shrink();
                    }
                }
            });
        }
    }
}

impl StateEpoch {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let key = _callback().push_on_epoch_stack();
        EPOCH_DATA_TOP.with(|top| {
            let mut top = top.borrow_mut();
            if top.active {
                // move old top to the stack
                EPOCH_DATA_STACK.with(|stack| {
                    let mut stack = stack.borrow_mut();
                    let new_top = EpochData {
                        key,
                        ..Default::default()
                    };
                    let old_top = mem::replace(&mut top.data, new_top);
                    stack.push(old_top);
                })
            } else {
                top.active = true;
                top.data.key = key;
                // do not have to do anything else, defaults are set at the
                // beginning and during dropping
            }
        });
        Self { key }
    }

    /// Gets the assertions associated with this Epoch (not including assertions
    /// from when sub-epochs are alive or from before the this Epoch was
    /// created)
    pub fn assertions(&self) -> Assertions {
        let mut res = Assertions::new();
        let mut found = false;
        EPOCH_DATA_TOP.with(|top| {
            let top = top.borrow();
            if top.data.key == self.key {
                res = top.data.assertions.clone();
                found = true;
            }
        });
        if !found {
            EPOCH_DATA_STACK.with(|stack| {
                let stack = stack.borrow();
                for (i, layer) in stack.iter().enumerate().rev() {
                    if layer.key == self.key {
                        res = layer.assertions.clone();
                        break
                    }
                    if i == 0 {
                        // shouldn't be reachable even with leaks
                        unreachable!();
                    }
                }
            });
        }
        res
    }

    pub fn next_visit_gen(&self) -> NonZeroU64 {
        EPOCH_DATA_TOP.with(|top| {
            let mut top = top.borrow_mut();
            let next =
                NonZeroU64::new(top.visit.get().wrapping_add(1)).expect("visit gen overflow");
            top.visit = next;
            next
        })
    }

    pub fn latest_state(&self) -> Option<PState> {
        let mut res = None;
        let mut found = false;
        EPOCH_DATA_TOP.with(|top| {
            let top = top.borrow();
            if top.data.key == self.key {
                res = top.data.prev_in_epoch;
                found = true;
            }
        });
        if !found {
            EPOCH_DATA_STACK.with(|stack| {
                let stack = stack.borrow();
                for (i, layer) in stack.iter().enumerate().rev() {
                    if layer.key == self.key {
                        res = layer.prev_in_epoch;
                        break
                    }
                    if i == 0 {
                        unreachable!();
                    }
                }
            });
        }
        res
    }

    pub fn get_mut_state<F: FnMut(&mut State)>(&self, p_state: PState, mut f: F) {
        EPOCH_DATA_TOP.with(|top| {
            let mut top = top.borrow_mut();
            let state = top.arena.get_mut(p_state).unwrap();
            f(state)
        })
    }

    /// Note: this borrows `EPOCH_DATA_TOP`
    pub fn pstate_to_pnode(&self, p_state: PState) -> PNode {
        EPOCH_DATA_TOP.with(|top| {
            let top = top.borrow_mut();
            top.arena.get(p_state).unwrap().node_map
        })
    }
}

// used in debugging and tests
#[doc(hidden)]
pub fn _get_epoch_data_arena<F: FnMut(&Arena<PState, State>)>(mut f: F) {
    EPOCH_DATA_TOP.with(|top| {
        let top = top.borrow_mut();
        f(&top.arena)
    })
}
#[doc(hidden)]
pub fn _get_top_assertions() -> Assertions {
    EPOCH_DATA_TOP.with(|top| {
        let top = top.borrow_mut();
        top.data.assertions.clone()
    })
}
