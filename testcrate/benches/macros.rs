#![feature(test)]

extern crate test;
use awint::{awi, cc, extawi, inlawi, Awi, Bits, ExtAwi, InlAwi};
use rand_xoshiro::{
    rand_core::{RngCore, SeedableRng},
    Xoshiro128StarStar,
};
use test::Bencher;

#[bench]
fn macro_cc(bencher: &mut Bencher) {
    let mut rng = Xoshiro128StarStar::seed_from_u64(0);
    let mut a = inlawi!(0u128);
    let mut b = inlawi!(0u128);
    bencher.iter(|| {
        let r = (rng.next_u32() % 128) as usize;
        a.rand_(&mut rng).unwrap();
        b.rand_(&mut rng).unwrap();
        cc!(imax: .., a[r..], b[..r]; ..256).unwrap()
    })
}

#[bench]
fn macro_inlawi(bencher: &mut Bencher) {
    let mut rng = Xoshiro128StarStar::seed_from_u64(0);
    let mut a = inlawi!(0u128);
    let mut b = inlawi!(0u128);
    bencher.iter(|| {
        let r = (rng.next_u32() % 128) as usize;
        a.rand_(&mut rng).unwrap();
        b.rand_(&mut rng).unwrap();
        inlawi!(imax: .., a[r..], b[..r]; ..256).unwrap()
    })
}

#[bench]
fn macro_extawi(bencher: &mut Bencher) {
    let mut rng = Xoshiro128StarStar::seed_from_u64(0);
    let mut a = inlawi!(0u128);
    let mut b = inlawi!(0u128);
    bencher.iter(|| {
        let r = (rng.next_u32() % 128) as usize;
        a.rand_(&mut rng).unwrap();
        b.rand_(&mut rng).unwrap();
        extawi!(imax: .., a[r..], b[..r]; ..256).unwrap()
    })
}

#[bench]
fn macro_awi(bencher: &mut Bencher) {
    let mut rng = Xoshiro128StarStar::seed_from_u64(0);
    let mut a = inlawi!(0u128);
    let mut b = inlawi!(0u128);
    bencher.iter(|| {
        let r = (rng.next_u32() % 128) as usize;
        a.rand_(&mut rng).unwrap();
        b.rand_(&mut rng).unwrap();
        awi!(imax: .., a[r..], b[..r]; ..256).unwrap()
    })
}
