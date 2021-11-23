use awint::Bits;
use rand_xoshiro::{rand_core::RngCore, Xoshiro128StarStar};
mod fp;
mod identities;
mod multi_bw;
mod one_run;

#[track_caller]
const fn check_invariants(x: &Bits) {
    if x.extra() != 0 && (x.last() & (usize::MAX << x.extra())) != 0 {
        panic!("unused bits are set");
    }
}

/// Checks for equality and that invariants are being kept
#[track_caller]
fn eq(lhs: &Bits, rhs: &Bits) {
    check_invariants(lhs);
    check_invariants(rhs);
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

/// Checks for nonequality and that invariants are being kept
#[track_caller]
fn ne(lhs: &Bits, rhs: &Bits) {
    check_invariants(lhs);
    check_invariants(rhs);
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

pub fn fuzz_step(rng: &mut Xoshiro128StarStar, x: &mut Bits, tmp: &mut Bits) {
    let r0 = (rng.next_u32() as usize) % x.bw();
    let r1 = (rng.next_u32() as usize) % x.bw();
    tmp.umax_assign();
    *tmp <<= r0;
    tmp.rotl_assign(r1);
    match rng.next_u32() % 4 {
        0 => *x |= &tmp,
        1 => *x &= &tmp,
        _ => *x ^= &tmp,
    }
}

pub const BITS: usize = usize::BITS as usize;
pub use fp::fp_identities;
pub use identities::identities;
pub use multi_bw::multi_bw;
pub use one_run::one_run;
