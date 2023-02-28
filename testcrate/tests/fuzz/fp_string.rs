// this is dramatically longer to compute because of allocations, so it gets its
// own file

use std::num::NonZeroUsize;

use awint::{bw, ExtAwi, FP};
use rand_xoshiro::{
    rand_core::{RngCore, SeedableRng},
    Xoshiro128StarStar,
};

use crate::fuzz::fuzz_step;

const MAX_FP: isize = 68;

/// Checks for equality
#[track_caller]
fn eq(lhs: &FP<ExtAwi>, rhs: &FP<ExtAwi>) {
    if !lhs.const_eq(rhs).unwrap_or_else(|| {
        panic!(
            "mismatched bitwidths: lhs.bw(): {} rhs.bw(): {}",
            lhs.bw(),
            rhs.bw()
        )
    }) {
        panic!("lhs and rhs are not equal when they should be:\nlhs:{lhs:?} rhs:{rhs:?}");
    }
}

fn fp_string_inner(rng: &mut Xoshiro128StarStar, x: &FP<ExtAwi>) -> Option<()> {
    // general string conversion
    let radix = ((rng.next_u32() % 35) + 2) as u8;
    let tmp_rng = rng.next_u32();
    let min_fraction_chars = if (tmp_rng & 0b10) == 0 {
        0
    } else {
        rng.next_u32() as usize % 70
    };
    let min_integer_chars = if (tmp_rng & 0b100) == 0 {
        0
    } else {
        rng.next_u32() as usize % 70
    };
    let (integer, fraction) = FP::to_vec_general(
        x,
        radix,
        (tmp_rng & 0b1000) != 0,
        min_integer_chars,
        min_fraction_chars,
        4096,
    )
    .unwrap();
    assert!(min_integer_chars <= integer.len());
    assert!(min_fraction_chars <= fraction.len());
    if min_integer_chars < integer.len() {
        // make sure there are no leading zeros
        if !integer.is_empty() {
            assert!(integer[0] != b'0');
        }
    }
    if min_fraction_chars < fraction.len() {
        // make sure there are no trailing zeros
        if !fraction.is_empty() {
            assert!(fraction[fraction.len() - 1] != b'0');
        }
    }
    let recovered = FP::new(
        x.signed(),
        ExtAwi::from_bytes_general(x.sign(), &integer, &fraction, 0, radix, x.nzbw(), x.fp())
            .unwrap(),
        x.fp(),
    )
    .unwrap();
    eq(x, &recovered);

    Some(())
}

fn rand_bool(rng: &mut Xoshiro128StarStar) -> bool {
    (rng.next_u32() & 1) == 0
}

fn rand_bw(rng: &mut Xoshiro128StarStar) -> NonZeroUsize {
    bw(((rng.next_u32() % (MAX_FP as u32)) + 1) as usize)
}

fn rand_fp(rng: &mut Xoshiro128StarStar) -> isize {
    (rng.next_u32() as isize) % MAX_FP
}

pub fn fp_string(seed: u64) -> Option<()> {
    // the seed makes sure that any repeats explore new space
    let rng = &mut Xoshiro128StarStar::seed_from_u64(seed);

    let s0 = rand_bool(rng);
    let bw0 = rand_bw(rng);
    let fp0 = rand_fp(rng);

    let mut x0bw0 = FP::new(s0, ExtAwi::zero(bw0), fp0).unwrap();
    let mut x1bw0 = x0bw0.clone();

    for _ in 0..16 {
        fuzz_step(rng, &mut x0bw0, &mut x1bw0);

        fp_string_inner(rng, &x0bw0)?;
    }
    Some(())
}
