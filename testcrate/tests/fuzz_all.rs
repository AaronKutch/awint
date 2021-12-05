#![feature(const_option)]
#![feature(const_fn_fn_ptr_basics)]
#![feature(const_mut_refs)]

mod fuzz;
use core::cmp;

use awint::{bw, inlawi_zero, ExtAwi, InlAwi};

const N: u32 = if cfg!(miri) {
    32
} else if cfg!(debug_assertions) {
    10_000
} else {
    1_000_000
};

macro_rules! test_extawi {
    ($($name:ident, $n:expr, $seed:expr, $bw:expr);*;) => {
        $(
            #[test]
            fn $name() {
                let bw = bw($bw);
                let array = [
                    &mut ExtAwi::zero(bw)[..],
                    &mut ExtAwi::zero(bw)[..],
                    &mut ExtAwi::zero(bw)[..],
                    &mut ExtAwi::zero(bw)[..],
                    &mut ExtAwi::zero(bw)[..],
                    &mut ExtAwi::zero(bw)[..],
                ];
                fuzz::identities($n, $seed, array).unwrap();
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
                fuzz::identities($n, $seed,
                    [
                        inlawi_zero!($len).const_as_mut(),
                        inlawi_zero!($len).const_as_mut(),
                        inlawi_zero!($len).const_as_mut(),
                        inlawi_zero!($len).const_as_mut(),
                        inlawi_zero!($len).const_as_mut(),
                        inlawi_zero!($len).const_as_mut(),
                    ]
                );
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
        let bw = bw(bw_i);
        let array = [
            &mut ExtAwi::zero(bw)[..],
            &mut ExtAwi::zero(bw)[..],
            &mut ExtAwi::zero(bw)[..],
            &mut ExtAwi::zero(bw)[..],
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
