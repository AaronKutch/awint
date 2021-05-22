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
    /// use awint::{InlAwi, inlawi, inlawi_zero};
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
        let result = rng.try_fill_bytes(self.as_mut_bytes());
        self.clear_unused_bits();
        result
    }
}
