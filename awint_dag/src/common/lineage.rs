use std::num::NonZeroUsize;

use crate::PState;

/// A trait for mimicking structs that allows access to the internal state
pub trait Lineage {
    fn state_nzbw(&self) -> NonZeroUsize {
        self.state().get_nzbw()
    }

    /// Get a reference to the `State` of `self`
    fn state(&self) -> PState;
}
