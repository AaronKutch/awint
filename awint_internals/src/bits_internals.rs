/// This is intended as a convenient way to hide special internal functions on
/// `Bits`. Originally, functions like `Bits::len` that were short for
/// convenient usage were marked `doc(hidden)` but were still too easy to
/// accidentally use. We put all these special functions behind a trait so that
/// they can't be accidentally used and are documented for developer purposes at
/// the same time.
pub trait BitsInternals {}
