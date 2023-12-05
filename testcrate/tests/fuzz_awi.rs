#![feature(const_option)]
#![feature(const_mut_refs)]

mod fuzz;
use core::cmp;

use awint::{bw, extawi, inlawi, Bits, ExtAwi, InlAwi};

const N: u32 = if cfg!(miri) {
    32
} else if cfg!(debug_assertions) {
    10_000
} else {
    1_000_000
};

macro_rules! test_extawi {
    ($($name:ident, $n:expr, $seed:expr, $w:expr);*;) => {
        $(
            #[test]
            fn $name() {
                let mut x0 = extawi!(zero: ..$w);
                let mut x1 = extawi!(zero: ..$w);
                let mut x2 = extawi!(zero: ..$w);
                let mut x3 = extawi!(zero: ..$w);
                let mut x4 = extawi!(zero: ..$w);
                let mut x5 = extawi!(zero: ..$w);
                fuzz::identities($n, $seed,
                    [&mut x0, &mut x1, &mut x2, &mut x3, &mut x4, &mut x5]
                ).unwrap();
                // prevent certain crazy false negatives from happening
                assert_eq!(x0.bw(), $w);
                assert_eq!(x1.bw(), $w);
                assert_eq!(x2.bw(), $w);
                assert_eq!(x3.bw(), $w);
                assert_eq!(x4.bw(), $w);
                assert_eq!(x5.bw(), $w);
            }
        )*
    };
}

// uses prime numbers, half way points, `Bits` without extra bits, and large
// values
test_extawi!(
    ext1, N, 0, 1;
    ext2, N, 0, 2;
    ext31, N, 0, 31;
    ext62, N, 0, 62;
    ext63, N, 0, 63;
    ext64, N, 0, 64;
    ext65, N, 0, 65;
    ext97, N, 0, 97;
    ext128, N, 0, 128;
    ext150, N, 0, 150;
    ext192, N, 0, 192;
    ext224, N, 0, 224;
    ext255, N, 0, 255;
    ext256, N, 0, 256;
    ext257, N, 0, 257;
);

#[cfg(not(miri))]
test_extawi!(ext2048, N, 0, 2048;);

// prime number over 32 * 64
#[cfg(not(miri))]
test_extawi!(ext2053, N, 0, 2053;);

macro_rules! test_inlawi {
    ($($name:ident, $n:expr, $seed:expr, $len:expr);*;) => {
        $(
            #[test]
            fn $name() {
                let mut x0 = inlawi!(zero: ..$len);
                let mut x1 = inlawi!(zero: ..$len);
                let mut x2 = inlawi!(zero: ..$len);
                let mut x3 = inlawi!(zero: ..$len);
                let mut x4 = inlawi!(zero: ..$len);
                let mut x5 = inlawi!(zero: ..$len);
                fuzz::identities($n, $seed,
                    [&mut x0, &mut x1, &mut x2, &mut x3, &mut x4, &mut x5]
                );
                // prevent certain crazy false negatives from happening
                InlAwi::assert_invariants(&x0);
                InlAwi::assert_invariants(&x1);
                InlAwi::assert_invariants(&x2);
                InlAwi::assert_invariants(&x3);
                InlAwi::assert_invariants(&x4);
                InlAwi::assert_invariants(&x5);
            }
        )*
    };
}

// since some of these bitwidths are duplicates, use a different seed to cover
// more space
test_inlawi!(
    inl63, N, 1, 63;
    inl64, N, 1, 64;
    inl65, N, 1, 65;
);

#[cfg(not(miri))]
test_inlawi!(inl2053, N, 1, 2053;);

#[test]
fn multi_bw() {
    for seed in 0..cmp::max(N / 4, 16) {
        fuzz::multi_bw(seed as u64).unwrap();
    }
}

#[test]
fn one_run() {
    #[cfg(miri)]
    let n = 258;
    #[cfg(not(miri))]
    let n = 9000;
    for bw_i in 1..=n {
        let w = bw(bw_i);
        let array: [&mut Bits; 4] = [
            &mut ExtAwi::zero(w),
            &mut ExtAwi::zero(w),
            &mut ExtAwi::zero(w),
            &mut ExtAwi::zero(w),
        ];
        fuzz::one_run(array).unwrap();
    }
}

// no unsafe code being used
#[cfg(not(miri))]
#[test]
fn fp_identities() {
    for seed in 0..cmp::max(N / 4, 16) {
        fuzz::fp_identities(seed as u64).unwrap();
    }
}

// no unsafe code being used
#[cfg(not(miri))]
#[test]
fn fp_string() {
    for seed in 0..cmp::max(N / 4, 16) {
        fuzz::fp_string(seed as u64).unwrap();
    }
}
