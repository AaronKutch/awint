use core::{num::NonZeroU64, sync::atomic::Ordering};

use crate::GLOBAL_UNIQUE_INVALID_GEN;

/// An Arena Pointer that can distinguish among 3 dimensions: arenas, indexes
/// into an arena, and different generations of elements in the same index.
///
/// Note: `TriPtr`s contain `NonZeroU64`s which allow certain enum optimizations
/// to be applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TriPtr {
    pub(crate) arena_id: NonZeroU64,
    /// Generation of the index when this internal allocation was made
    pub(crate) gen: NonZeroU64,
    /// index into the arena
    pub(crate) index: usize,
}

impl TriPtr {
    /// Creates a `TriPtr` that is guaranteed to be invalid
    pub const fn invalid() -> TriPtr {
        TriPtr {
            arena_id: NonZeroU64::new(1).unwrap(), // `GLOBAL_ARENA_ID` starts at 2
            gen: NonZeroU64::new(1).unwrap(),      // `GLOBAL_UNIQUE_INVALID_GEN` starts at 2
            index: 0,
        }
    }

    /// Creates a `TriPtr` that is guaranteed to be invalid and unequal to any
    /// other `TriPtr`. Note: this function makes atomic fetches.
    pub fn unique_invalid() -> TriPtr {
        let new_gen = GLOBAL_UNIQUE_INVALID_GEN.fetch_add(1, Ordering::Relaxed);
        if new_gen <= 0 {
            panic!("GLOBAL_UNIQUE_INVALID_GEN overflow");
        }
        TriPtr {
            arena_id: NonZeroU64::new(1).unwrap(), // `GLOBAL_ARENA_ID` starts at 2
            gen: NonZeroU64::new(new_gen as u64).unwrap(),
            index: 0,
        }
    }

    /// Returns the id of the arena that this `TriPtr` was created from
    pub fn arena_id(&self) -> NonZeroU64 {
        self.arena_id
    }
}
