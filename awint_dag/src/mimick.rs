mod awi;
mod bits;
mod ops;
pub mod primitive;

use std::{num::NonZeroUsize, rc::Rc};

pub use awi::{ExtAwi, InlAwi};
pub use bits::Bits;

use crate::Op;

/// Represents a single state that `mimick::Bits` is in at one point in time.
/// The operands point to other `State`s. `Bits`, `InlAwi`, and `ExtAwi` use
/// `Rc` pointers to `States`, so that they can change their state without
/// borrowing issues or mutating `States` (which could be used as operands by
/// other `States`).
#[derive(Debug, Hash, PartialEq, Eq)]
pub struct State {
    /// Bitwidth
    pub nzbw: Option<NonZeroUsize>,
    /// Operation
    pub op: Op,
    /// Operands
    pub ops: Vec<Rc<Self>>,
}

impl State {
    pub fn new(nzbw: Option<NonZeroUsize>, op: Op, ops: Vec<Rc<State>>) -> Rc<Self> {
        Rc::new(Self { nzbw, op, ops })
    }

    // Note: there is no `update` function, because we run into borrowing problems
    // when using a previous state to create a new one and replace the current
}

/// The mimicking structs have extra information that the lowering logic needs
/// but that can't be exposed in their public interfaces. This trait exposes
/// extra functions on mimicking structs.
pub trait Lineage {
    fn from_state(state: Rc<State>) -> Self;

    fn state_nzbw(&self) -> Option<NonZeroUsize> {
        self.state().nzbw
    }

    /// If the underlying type has a known constant bitwidth, such as `InlAwi`
    /// or a mimicking primitive
    fn hidden_const_nzbw() -> Option<NonZeroUsize>;

    fn hidden_const_bw() -> Option<usize> {
        Self::hidden_const_nzbw().map(|x| x.get())
    }

    /// Get a reference to the `State` of `self`
    fn state(&self) -> Rc<State>;
}
