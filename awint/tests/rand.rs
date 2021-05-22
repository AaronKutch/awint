#![cfg(feature = "rand_support")]

use awint::{inlawi, inlawi_zero, InlAwi};
use rand_xoshiro::{rand_core::SeedableRng, Xoshiro128StarStar};

#[test]
fn rand() {
    // note: mirror changes of this example to the doctest for the
    // `rand_assign_using` function
    let mut rng = Xoshiro128StarStar::seed_from_u64(0);
    let mut awi = inlawi_zero!(100);
    awi.const_as_mut().rand_assign_using(&mut rng).unwrap();
    assert_eq!(awi, inlawi!(0x5ab77d3629a089d75dec9045du100));
    awi.const_as_mut().rand_assign_using(&mut rng).unwrap();
    assert_eq!(awi, inlawi!(0x4c25a514060dea0565c95a8dau100));
}
