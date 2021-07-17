mod awi;
mod bits;
mod ops;
pub mod primitive;

use std::{num::NonZeroUsize, rc::Rc};

pub use awi::{ExtAwi, InlAwi};
pub use bits::Bits;

use crate::Op;

/// The mimicking structs have extra information that the lowering logic needs
/// but that can't be exposed in their public interfaces. This trait exposes
/// extra functions on mimicking structs.
pub trait Lineage {
    fn new(bw: NonZeroUsize, op: Op, ops: Vec<Rc<State>>) -> Self;

    fn state(&self) -> Rc<State>;
}

/// Represents a single state that `mimick::Bits` is in at one point in time.
/// The operands point to other `State`s. `Bits`, `InlAwi`, and `ExtAwi` use
/// `Rc` pointers to `States`, so that they can change their state without
/// borrowing issues or mutating `States` (which could be used as operands by
/// other `States`).
#[derive(Debug, Hash, PartialEq, Eq)]
pub struct State {
    /// Bitwidth
    pub nzbw: NonZeroUsize,
    /// Operation
    pub op: Op,
    /// Operands
    pub ops: Vec<Rc<Self>>,
}

impl State {
    pub fn new(nzbw: NonZeroUsize, op: Op, ops: Vec<Rc<State>>) -> Rc<Self> {
        Rc::new(Self { nzbw, op, ops })
    }
}

/// `Lineage` but for known bitwidth contexts
pub trait ConstBwLineage {
    fn new(op: Op, ops: Vec<Rc<State>>) -> Self;

    /// This function is here to avoid implementing `const_nzbw` as a direct
    /// member function on the primitives
    fn hidden_const_nzbw() -> NonZeroUsize;

    fn hidden_const_bw() -> usize {
        Self::hidden_const_nzbw().get()
    }

    fn state(&self) -> Rc<State>;
}
