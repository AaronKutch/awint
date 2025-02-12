//! Access to special epoch structs and functions. Most users should instead use
//! a downstream epoch management construct such as the `Epoch` from the
//! `starlight` crate.

// TODO
#![allow(renamed_and_removed_lints)]
#![allow(clippy::thread_local_initializer_can_be_made_const)]

use std::{
    cell::{Cell, RefCell},
    marker::PhantomData,
    num::{NonZeroU64, NonZeroUsize},
    rc::Rc,
};

use awint_ext::awint_internals::Location;

use crate::{dag, Op, PState};

/// A set of callback functions called by the mimicking types as they are
/// created and operated on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EpochCallback {
    /// Called when new state should be created with the given bitwidth,
    /// `Op<PState>`, and optional location. Should return a `PState` pointing
    /// to the new state.
    pub new_pstate: fn(NonZeroUsize, Op<PState>, Option<Location>) -> PState,
    /// Called when an existing `dag::bool` should be registered as an assertion
    /// bit. Is also attached with location information.
    pub register_assertion_bit: fn(dag::bool, Location),
    /// Should return the bitwidth of the state corresponding to the `PState`.
    pub get_nzbw: fn(PState) -> NonZeroUsize,
    /// Should return the `Option<PState>` that the `PState` was created for.
    pub get_op: fn(PState) -> Op<PState>,
}

/// The current callback used for when nothing is on the epoch stack
#[doc(hidden)]
pub fn _unregistered_callback() -> EpochCallback {
    fn panic0() -> ! {
        panic!(
            "attempted to use mimicking types from `awint_dag` when no `EpochCallback` is \
             registered (there should be an active epoch management struct such as \
             `starlight::Epoch`)"
        );
    }
    fn panic1(_: NonZeroUsize, _: Op<PState>, _: Option<Location>) -> PState {
        panic0()
    }
    fn panic2(_: dag::bool, _: Location) {
        panic0()
    }
    fn panic3(_: PState) -> NonZeroUsize {
        panic0()
    }
    fn panic4(_: PState) -> Op<PState> {
        panic0()
    }
    EpochCallback {
        new_pstate: panic1,
        register_assertion_bit: panic2,
        get_nzbw: panic3,
        get_op: panic4,
    }
}

thread_local!(
    /// The current Epoch generation, used for insuring that Epoch lifetimes are
    /// stacklike.
    static EPOCH_GEN: Cell<NonZeroU64> = Cell::new(NonZeroU64::new(2).unwrap());

    /// The Epoch callback stack. Includes the associated epoch generation
    /// number
    static EPOCH_STACK: RefCell<Vec<(NonZeroU64, EpochCallback)>> = RefCell::new(vec![]);

    /// This should always be a clone of the last element on the `EPOCH_STACK`,
    /// or if the stack is empty it should be `unregistered_callback()`. This is
    /// a simple `Cell<EpochCallback>` for performance.
    static CURRENT_CALLBACK: Cell<EpochCallback> = Cell::new(_unregistered_callback());
);

/// Used by epoch handler structs to be able to call `pop_off_epoch_stack` when
/// they are done.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EpochKey {
    generation: NonZeroU64,
    _no_send_or_sync: PhantomData<fn() -> Rc<()>>,
}

impl EpochCallback {
    /// Pushes this epoch callback onto the epoch stack, registering it to
    /// recieve callbacks from the mimicking types. Returns an `EpochKey`, which
    /// has a method [EpochKey::pop_off_epoch_stack] to deregister the epoch.
    pub fn push_on_epoch_stack(self) -> EpochKey {
        let generation: NonZeroU64 = EPOCH_GEN.with(|g| {
            let generation = g.get();
            g.set(
                NonZeroU64::new(generation.get().wrapping_add(1))
                    .expect("epoch generation counter overflow"),
            );
            generation
        });
        EPOCH_STACK.with(|v| {
            v.borrow_mut().push((generation, self));
        });
        // TODO #25
        //CURRENT_CALLBACK.replace(self);
        CURRENT_CALLBACK.with(|x| {
            x.replace(self);
        });
        EpochKey {
            generation,
            _no_send_or_sync: PhantomData,
        }
    }
}

impl EpochKey {
    /// Pops the epoch callback corresponding to `self` off the epoch stack,
    /// deregistering it. The new last epoch callback is reregistered in its
    /// place (or a panicking callback is registered if the stack is empty).
    ///
    /// # Errors
    ///
    /// If an epoch was pushed on after the one referred to by `self` was, and
    /// if that epoch has not been popped off and deregistered, then the stack
    /// invariant is violated and this function returns a tuple of the `self`
    /// generation and the generation that has not been dropped yet. Users
    /// should probably use this to panic.
    pub fn pop_off_epoch_stack(self) -> Result<(), (NonZeroU64, NonZeroU64)> {
        EPOCH_STACK.with(|v| {
            let mut epoch_stack = v.borrow_mut();
            let (top_gen, _) = epoch_stack.last().unwrap();
            if self.generation != *top_gen {
                return Err((self.generation, *top_gen));
            }
            epoch_stack.pop().unwrap();
            if let Some((_, callback)) = epoch_stack.last() {
                // TODO #25
                //CURRENT_CALLBACK.replace(*callback);
                CURRENT_CALLBACK.with(|x| {
                    x.replace(*callback);
                });
            } else {
                // TODO #25
                //CURRENT_CALLBACK.replace(_unregistered_callback());
                CURRENT_CALLBACK.with(|x| {
                    x.replace(_unregistered_callback());
                });
            }
            Ok(())
        })
    }

    /// Returns an `EpochKey` that is always invalid
    pub fn invalid() -> Self {
        Self {
            generation: NonZeroU64::new(1).unwrap(),
            _no_send_or_sync: PhantomData,
        }
    }

    /// Returns the generation of `self`
    pub fn generation(&self) -> NonZeroU64 {
        self.generation
    }
}

impl Default for EpochKey {
    fn default() -> Self {
        Self::invalid()
    }
}

/// Uses the callback of the current epoch to create a new `PState`
///
/// # Panics
///
/// If there is no epoch currently registered
pub fn new_pstate_for_current_epoch(
    nzbw: NonZeroUsize,
    op: Op<PState>,
    location: Option<Location>,
) -> PState {
    CURRENT_CALLBACK.with(|callback| (callback.get().new_pstate)(nzbw, op, location))
}

/// Uses the callback of the current epoch to register an assertion bit
///
/// # Panics
///
/// If there is no epoch currently registered
pub fn register_assertion_bit_for_current_epoch(bit: dag::bool, location: Location) {
    CURRENT_CALLBACK.with(|callback| (callback.get().register_assertion_bit)(bit, location))
}

/// Uses the callback of the current epoch to get the bitwidth of a state
///
/// # Panics
///
/// If `p_state` is invalid or if there is no epoch currently registered
pub fn get_nzbw_from_current_epoch(p_state: PState) -> NonZeroUsize {
    CURRENT_CALLBACK.with(|callback| (callback.get().get_nzbw)(p_state))
}

/// Uses the callback of the current epoch to get the operation of a state
///
/// # Panics
///
/// If `p_state` is invalid or if there is no epoch currently registered
pub fn get_op_from_current_epoch(p_state: PState) -> Op<PState> {
    CURRENT_CALLBACK.with(|callback| (callback.get().get_op)(p_state))
}

// used in debugging and testing
#[doc(hidden)]
pub fn _get_epoch_gen() -> NonZeroU64 {
    // TODO #25
    //EPOCH_GEN.get()
    EPOCH_GEN.with(|x| x.get())
}
#[doc(hidden)]
pub fn _get_epoch_stack() -> Vec<(NonZeroU64, EpochCallback)> {
    EPOCH_STACK.with(|v| v.borrow().clone())
}
#[doc(hidden)]
pub fn _get_epoch_callback() -> EpochCallback {
    // TODO #25
    //CURRENT_CALLBACK.get()
    CURRENT_CALLBACK.with(|x| x.get())
}
