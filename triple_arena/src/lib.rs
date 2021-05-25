#![feature(const_option)]
#![no_std]

extern crate alloc;
use core::sync::atomic::AtomicI64;

mod triarena;
mod triptr;

pub use triarena::Arena;
pub use triptr::TriPtr;

pub mod prelude {
    pub use crate::{Arena, TriPtr};
}

/// For generating unique Arena IDs. Starts at 2 so that `NonZeroU64` for enum
/// optimization and a guaranteed invalid id can exist.
static GLOBAL_ARENA_ID: AtomicI64 = AtomicI64::new(2);

/// For generating unique invalid `TriPtr`s
static GLOBAL_UNIQUE_INVALID_GEN: AtomicI64 = AtomicI64::new(2);
