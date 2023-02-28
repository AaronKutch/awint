use std::num::NonZeroUsize;

use awint::{
    fp::{F32, F64},
    *,
};
use rand_xoshiro::{
    rand_core::{RngCore, SeedableRng},
    Xoshiro128StarStar,
};

use crate::fuzz::fp::num_eq;

const N: u32 = if cfg!(debug_assertions) {
    10_000
} else {
    1_000_000
};

const MAX_BW_32: usize = 258;
const MAX_FP_32: isize = 258;
const MAX_BW_64: usize = 258;
const MAX_FP_64: isize = 1100;

fn fuzz_awi(rng: &mut Xoshiro128StarStar, w: NonZeroUsize) -> ExtAwi {
    let mut awi = ExtAwi::zero(w);
    let mut mask = ExtAwi::umax(w);
    for _ in 0..8 {
        mask.umax_();
        mask.range_and_(0..((rng.next_u64() as usize) % w.get()))
            .unwrap();
        mask.rotl_((rng.next_u64() as usize) % w.get()).unwrap();
        match rng.next_u32() % 4 {
            0 => awi.or_(&mask).unwrap(),
            1 => awi.and_(&mask).unwrap(),
            _ => awi.xor_(&mask).unwrap(),
        }
    }
    awi
}

#[track_caller]
fn check_f32(f: f32, awi: inlawi_ty!(25), fp: isize) {
    let mut tmp = F32::new(true, awi, fp).unwrap();
    assert_eq!(FP::from_f32(f), tmp);
    assert_eq!(
        FP::try_to_f32(&mut tmp).unwrap(),
        if f.is_finite() { f } else { 0.0 }
    );
}

#[test]
fn fp_f32() {
    check_f32(1.0, inlawi!(0, 1, 0u23), 150 - 127);
    check_f32(-1.0, inlawi!(11, 0u23), 150 - 127);
    check_f32(2.0, inlawi!(0, 1, 0u23), 150 - 128);
    check_f32(0.0, inlawi!(0u25), 150);
    check_f32(f32::INFINITY, inlawi!(0u25), 150 - 255);
    check_f32(f32::NEG_INFINITY, inlawi!(0u25), 150 - 255);
    check_f32(f32::NAN, inlawi!(0u25), 150 - 255);
    check_f32(f32::MIN_POSITIVE, inlawi!(0, 1, 0u23), 150 - 1);
    check_f32(f32::MAX, inlawi!(imax: ..25), 150 - 254);
    assert_eq!(
        FP::try_to_f32(&mut FP::new(true, inlawi!(1), -127).unwrap()).unwrap(),
        -1.7014118e38
    );
    assert!(FP::try_to_f32(&mut FP::new(true, inlawi!(1), -128).unwrap()).is_none());
    // also testing macros this way
    check_f32(
        std::f32::consts::TAU,
        inlawi!(
            ExtAwi::from_str_general(
                Some(false),
                "6",
                "2831853071795862",
                0,
                10,
                bw(25),
                21
            ).unwrap();
            ..25
        )
        .unwrap(),
        21,
    );
    // subnormal
    check_f32(1.337e-39, inlawi!(0, 0, 0xe8f03u23), 150);

    let mut rng = Xoshiro128StarStar::seed_from_u64(0);
    for _ in 0..N {
        let w = NonZeroUsize::new((rng.next_u64() as usize) % MAX_BW_32 + 1).unwrap();
        let awi = fuzz_awi(&mut rng, w);
        let mut fp0 = FP::new(
            (rng.next_u32() & 1) == 0,
            awi,
            (rng.next_u32() as i32 as isize) % MAX_FP_32,
        )
        .unwrap();
        let mut fp1 = fp0.clone();
        let mut fp2 = fp0.clone();
        let mut fp3 = fp0.clone();
        if let Some(f) = FP::try_to_f32(&mut fp0) {
            let mut awif32 = FP::from_f32(f);
            FP::floating_(&mut fp1, &mut awif32).unwrap();
            num_eq(true, false, &fp0, &fp1, &mut fp2, &mut fp3).unwrap();
        } else {
            assert!((fp0.sig() as isize) - fp0.fp() >= 128);
        }
    }
}

#[track_caller]
fn check_f64(f: f64, awi: inlawi_ty!(54), fp: isize) {
    let mut tmp = F64::new(true, awi, fp).unwrap();
    assert_eq!(FP::from_f64(f), tmp);
    assert_eq!(
        FP::try_to_f64(&mut tmp).unwrap(),
        if f.is_finite() { f } else { 0.0 }
    );
}

#[test]
fn fp_f64() {
    check_f64(1.0, inlawi!(0, 1, 0u52), 1075 - 1023);
    check_f64(-1.0, inlawi!(11, 0u52), 1075 - 1023);
    check_f64(2.0, inlawi!(0, 1, 0u52), 1075 - 1024);
    check_f64(0.0, inlawi!(0u54), 1075);
    check_f64(f64::INFINITY, inlawi!(0u54), 1075 - 2047);
    check_f64(f64::NEG_INFINITY, inlawi!(0u54), 1075 - 2047);
    check_f64(f64::NAN, inlawi!(0u54), 1075 - 2047);
    check_f64(f64::MIN_POSITIVE, inlawi!(0, 1, 0u52), 1075 - 1);
    check_f64(f64::MAX, inlawi!(imax: ..54), 1075 - 2046);
    assert_eq!(
        FP::try_to_f64(&mut FP::new(true, inlawi!(1), -1023).unwrap()).unwrap(),
        -8.98846567431158e307
    );
    assert!(FP::try_to_f64(&mut FP::new(true, inlawi!(1), -1024).unwrap()).is_none());
    check_f64(
        std::f64::consts::TAU,
        inlawi!(
            ExtAwi::from_str_general(
                Some(false),
                "6",
                "2831853071795862",
                0,
                10,
                bw(54),
                50
            ).unwrap();
            ..54
        )
        .unwrap(),
        50,
    );
    // subnormal
    check_f64(1.337e-311, inlawi!(0, 0, 0x2761135ac7fu52), 1075);

    let mut rng = Xoshiro128StarStar::seed_from_u64(0);
    for _ in 0..N {
        let w = NonZeroUsize::new((rng.next_u64() as usize) % MAX_BW_64 + 1).unwrap();
        let awi = fuzz_awi(&mut rng, w);
        let mut fp0 = FP::new(
            (rng.next_u32() & 1) == 0,
            awi,
            (rng.next_u32() as i32 as isize) % MAX_FP_64,
        )
        .unwrap();
        let mut fp1 = fp0.clone();
        let mut fp2 = fp0.clone();
        let mut fp3 = fp0.clone();
        if let Some(f) = FP::try_to_f64(&mut fp0) {
            let mut awif64 = FP::from_f64(f);
            FP::floating_(&mut fp1, &mut awif64).unwrap();
            num_eq(true, false, &fp0, &fp1, &mut fp2, &mut fp3).unwrap()
        } else {
            assert!((fp0.sig() as isize) - fp0.fp() >= 1024);
        }
    }
}
