use std::num::NonZeroUsize;

use awint::{bw, cc, Bits, ExtAwi, FP};
use rand_xoshiro::{
    rand_core::{RngCore, SeedableRng},
    Xoshiro128StarStar,
};

use crate::fuzz::fuzz_step;

// there are currently no FP functions that deal with the digit level, so I
// don't think we need 258
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
        panic!(
            "lhs and rhs are not equal when they should be:\nlhs:{:?} rhs:{:?}",
            lhs, rhs
        );
    }
}

/*
/// Checks for nonequality
#[track_caller]
fn ne(lhs: &FP<ExtAwi>, rhs: &FP<ExtAwi>) {
    if lhs.const_eq(rhs).unwrap_or_else(|| {
        panic!(
            "mismatched bitwidths: lhs.bw(): {} rhs.bw(): {}",
            lhs.bw(),
            rhs.bw()
        )
    }) {
        panic!(
            "lhs and rhs are equal when they should not be:\nlhs:{:?} rhs:{:?}",
            lhs, rhs
        );
    }
}
*/

/// Follows the same patterns as `identities.rs` and `multi_bw.rs` and combines
/// them together
fn fp_identities_inner(
    x0bw0: &FP<ExtAwi>,
    //x1bw0: &FP<ExtAwi>,
    //x2bw0: &FP<ExtAwi>,
    mut x3bw0: &mut FP<ExtAwi>,
    x4bw0: &mut FP<ExtAwi>,
    //x5bw0: &mut FP<ExtAwi>,
    x0bw1: &FP<ExtAwi>,
    //x1bw1: &FP<ExtAwi>,
    mut x2bw1: &mut FP<ExtAwi>,
    x3bw1: &mut FP<ExtAwi>,
    //x0bw2: &FP<ExtAwi>,
    //x1bw2: &mut FP<ExtAwi>,
    mut pad0: &mut Bits,
    /*pad1: &mut Bits,
     *pad2: &mut Bits, */
) -> Option<()> {
    // used by truncation alignment
    let align0 = (MAX_FP - x0bw0.fp()) as usize;
    let align1 = (MAX_FP - x0bw1.fp()) as usize;

    // truncation
    cc!(x0bw0; x3bw0)?;
    x3bw0.neg_assign(x0bw0.is_negative());
    cc!(zero: .., x3bw0, ..align0; pad0).unwrap();
    cc!(zero: pad0; .., x2bw1, ..align1)?;
    x2bw1.neg_assign(x0bw1.signed() && x0bw0.is_negative());
    cc!(x0bw0; x3bw0)?;
    FP::truncate_assign(x3bw1, x3bw0);
    // make sure arg not mutated
    eq(x3bw0, x0bw0);
    eq(x2bw1, x3bw1);

    // overflowing truncation
    cc!(x0bw0; x3bw0)?;
    FP::otruncate_assign(x2bw1, x3bw0);
    FP::truncate_assign(x3bw1, x3bw0);
    // assert equal to original truncation
    eq(x2bw1, x3bw1);
    // restart
    cc!(x0bw0; x3bw0)?;
    let o = FP::otruncate_assign(x2bw1, x3bw0);
    // make sure not mutated
    eq(x3bw0, x0bw0);
    x3bw0.neg_assign(x0bw0.is_negative());
    // find if low and high bits get cut off
    cc!(zero: .., x3bw0, ..align0; pad0).unwrap();
    if !pad0.is_zero() {
        let mut target_bounds = FP::rel_sb(x2bw1);
        target_bounds.0 += MAX_FP;
        target_bounds.1 += MAX_FP;
        let lsnb = pad0.tz() as isize;
        assert!(o.0 == ((lsnb < target_bounds.0) || (lsnb > target_bounds.1)));
        let msnb = (pad0.bw() - pad0.lz() - 1) as isize;
        let extra = x2bw1.is_negative() != x0bw0.is_negative();
        assert!(o.1 == ((msnb < target_bounds.0) || (msnb > target_bounds.1) || extra));
    } else {
        assert!(!(o.0 || o.1));
    }

    // test transitive preservation of numerical value
    cc!(x0bw0; x3bw0)?;
    let o0 = FP::otruncate_assign(x2bw1, x3bw0);
    let o1 = FP::otruncate_assign(x4bw0, x2bw1);
    if !(o0.0 || o0.1 || o1.0 || o1.1) {
        eq(x3bw0, x4bw0);
    }

    Some(())
}

fn rand_bool(rng: &mut Xoshiro128StarStar) -> bool {
    (rng.next_u32() & 1) == 0
}

fn rand_bw(rng: &mut Xoshiro128StarStar) -> NonZeroUsize {
    bw(((rng.next_u32() % (MAX_FP as u32)) + 1) as usize)
}

fn rand_fp(rng: &mut Xoshiro128StarStar) -> isize {
    // tricky bug: if we cast directly to an `isize`, it would always be positive on
    // platforms with an `isize` larger than 32 bits.
    (rng.next_u32() as i32 as isize) % MAX_FP
}

/// For testing operations with multiple bitwidths
pub fn fp_identities(seed: u64) -> Option<()> {
    // the seed makes sure that any repeats explore new space
    let rng = &mut Xoshiro128StarStar::seed_from_u64(seed);

    let s0 = rand_bool(rng);
    let bw0 = rand_bw(rng);
    let fp0 = rand_fp(rng);
    let s1 = rand_bool(rng);
    let bw1 = rand_bw(rng);
    let fp1 = rand_fp(rng);
    //let s2 = rand_bool(rng);
    //let bw2 = rand_bw(rng);
    //let fp2 = rand_fp(rng);

    let mut x0bw0 = FP::new(s0, ExtAwi::zero(bw0), fp0).unwrap();
    let mut x1bw0 = x0bw0.clone();
    let mut x3bw0 = x0bw0.clone();
    let mut x4bw0 = x0bw0.clone();
    let mut x0bw1 = FP::new(s1, ExtAwi::zero(bw1), fp1).unwrap();
    let mut x1bw1 = x0bw1.clone();
    let mut x2bw1 = x0bw1.clone();
    let mut x3bw1 = x0bw1.clone();
    //let mut x1bw2 = FP::new(s2, ExtAwi::zero(bw2), fp2).unwrap();

    let mut pad0 = ExtAwi::zero(bw((MAX_FP as usize) * 3));
    //let mut pad1 = pad0.clone();
    //let mut pad2 = pad0.clone();

    for _ in 0..16 {
        fuzz_step(rng, &mut x0bw0, &mut x1bw0);
        fuzz_step(rng, &mut x0bw1, &mut x1bw1);
        //fuzz_step(rng, &mut x0bw2, &mut x1bw2);
        //let b = (rng.next_u32() & 1) == 0;

        fp_identities_inner(
            &x0bw0, &mut x3bw0, &mut x4bw0, &x0bw1, &mut x2bw1, &mut x3bw1, &mut pad0,
        )?;
    }
    Some(())
}
