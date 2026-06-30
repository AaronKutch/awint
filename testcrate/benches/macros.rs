#![feature(test)]

extern crate test;
use awint::{Awi, Bits, ExtAwi, InlAwi, awi, cc, extawi, inlawi};
use rand_xoshiro::{
    Xoshiro128StarStar,
    rand_core::{RngCore, SeedableRng},
};
use test::Bencher;

#[bench]
fn macro_cc(bencher: &mut Bencher) {
    let mut rng = Xoshiro128StarStar::seed_from_u64(0);
    let mut a = inlawi!(0u128);
    let mut b = inlawi!(0u128);
    bencher.iter(|| {
        let r = (rng.next_u32() % 128) as usize;
        a.rand_(&mut rng);
        b.rand_(&mut rng);
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
        a.rand_(&mut rng);
        b.rand_(&mut rng);
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
        a.rand_(&mut rng);
        b.rand_(&mut rng);
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
        a.rand_(&mut rng);
        b.rand_(&mut rng);
        awi!(imax: .., a[r..], b[..r]; ..256).unwrap()
    })
}
