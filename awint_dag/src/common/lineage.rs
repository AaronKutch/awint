use std::num::NonZeroUsize;

use super::state::{get_state, PState};

/// The mimicking structs have extra information that the lowering logic needs
/// but that can't be exposed in their public interfaces. This trait exposes
/// extra functions on mimicking structs.
pub trait Lineage {
    fn state_nzbw(&self) -> NonZeroUsize {
        get_state(self.state()).unwrap().nzbw
    }

    /// Get a reference to the `State` of `self`
    fn state(&self) -> PState;
}
