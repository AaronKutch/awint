mod bits;
mod inlawi;
#[cfg(feature = "serde_support")]
mod serde;
#[cfg(feature = "star_rng_support")]
mod star_rng;

pub use bits::Bits;
pub use inlawi::InlAwi;

#[cfg(feature = "const_support")]
mod const_traits;
#[cfg(not(feature = "const_support"))]
mod traits;
#[cfg(feature = "const_support")]
pub use const_traits::{AsBits, AsMutBits};
#[cfg(not(feature = "const_support"))]
pub use traits::{AsBits, AsMutBits};
