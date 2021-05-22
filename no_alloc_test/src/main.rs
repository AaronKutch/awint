#![no_std]
#![no_main]

extern crate panic_halt;

use awint::prelude::*;
use rand_xoshiro::{rand_core::SeedableRng, Xoroshiro128StarStar};
use riscv_minimal_rt::entry;

#[entry]
fn main() -> ! {
    // test that the procedural macros can still use allocation at compile time
    // without the dependency leaking into runtime
    let mut awi0 = inlawi!(12345i20);
    let awi1 = inlawi!(54321i20);
    let x0 = awi0.const_as_mut();
    let x1 = awi1.const_as_ref();
    x0.add_assign(x1).unwrap();
    assert!(x0.is_zero());
    let mut rng = Xoroshiro128StarStar::seed_from_u64(0);
    x0.rand_assign_using(&mut rng).unwrap();

    panic!("main is not allowed to return")
}
