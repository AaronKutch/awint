use std::{num::NonZeroUsize, rc::Rc};

use crate::common::State;

/// The mimicking structs have extra information that the lowering logic needs
/// but that can't be exposed in their public interfaces. This trait exposes
/// extra functions on mimicking structs.
pub trait Lineage {
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
