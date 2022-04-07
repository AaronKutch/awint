use awint_internals::*;

use crate::Bits;

/// `rand_support` functions
impl Bits {
    // this is tested by `awint_test/tests/rand.rs`

    /// Randomly-assigns `self` using a `rand_core::RngCore` random number
    /// generator. This works by calling `RngCore::try_fill_bytes` on
    /// `self.as_mut_bytes`, clearing unused bits, and returning the result.
    ///
    /// ```
    /// // Example using the `rand_xoshiro` crate.
    /// use awint::{Bits, InlAwi, inlawi, inlawi_zero};
    /// use rand_xoshiro::{rand_core::SeedableRng, Xoshiro128StarStar};
    ///
    /// let mut rng = Xoshiro128StarStar::seed_from_u64(0);
    /// let mut awi = inlawi_zero!(100);
    /// awi.const_as_mut().rand_assign_using(&mut rng).unwrap();
    /// assert_eq!(awi, inlawi!(0x5ab77d3629a089d75dec9045du100));
    /// awi.const_as_mut().rand_assign_using(&mut rng).unwrap();
    /// assert_eq!(awi, inlawi!(0x4c25a514060dea0565c95a8dau100));
    /// ```
    pub fn rand_assign_using<R>(&mut self, rng: &mut R) -> Result<(), rand_core::Error>
    where
        R: rand_core::RngCore,
    {
        // We really want to use `try_fill_bytes` without an intermediate buffer.

        // Here we make it portable with respect to length by emulating a byte sized
        // unused bits scheme. On big endian systems this will set some unused bytes,
        // but this will be fixed below.
        let size_in_u8 = if (self.bw() % 8) == 0 {
            self.bw() / 8
        } else {
            (self.bw() / 8) + 1
        };
        let bytes = &mut self.as_mut_bytes_full_width_nonportable()[..size_in_u8];
        let result = rng.try_fill_bytes(bytes);
        // this is a no-op on little endian, but on big endian this fixes byte order in
        // regular digits and rotates out unused bytes
        const_for!(i in {0..self.len()} {
            self.as_mut_slice()[i] = usize::from_le(self.as_mut_slice()[i]);
        });
        // clean up unused bits in last byte
        self.clear_unused_bits();
        result
    }
}
