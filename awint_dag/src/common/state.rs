use std::{num::NonZeroUsize, rc::Rc};

use crate::common::Op;

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
