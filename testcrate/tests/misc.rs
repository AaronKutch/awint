use awint::prelude::*;
use rand_xoshiro::{rand_core::SeedableRng, Xoshiro128StarStar};

/// [Bits::lut] needs its own test because of its special requirements
#[test]
fn lut_and_field() {
    let mut rng = Xoshiro128StarStar::seed_from_u64(0);
    #[cfg(not(miri))]
    let (out_bw_max, pow_max) = (258, 8);
    #[cfg(miri)]
    let (out_bw_max, pow_max) = (68, 3);
    let mut awi_tmp0 = ExtAwi::zero(bw(out_bw_max * (1 << pow_max)));
    let mut awi_tmp1 = ExtAwi::zero(bw(out_bw_max * (1 << pow_max)));
    let tmp0 = awi_tmp0.const_as_mut();
    let tmp1 = awi_tmp1.const_as_mut();
    for out_bw in 1..out_bw_max {
        let mut awi_out = ExtAwi::zero(bw(out_bw));
        for pow in 1..pow_max {
            let mul = 1 << pow;
            let mut awi_lut = ExtAwi::zero(bw(out_bw * mul));
            awi_lut.rand_assign_using(&mut rng).unwrap();
            let mut awi_inx = ExtAwi::zero(bw(pow));
            let out = awi_out.const_as_mut();
            let lut = awi_lut.const_as_ref();
            let inx = awi_inx.const_as_mut();
            for i in 0..mul {
                inx.usize_assign(i);
                out.lut(lut, inx).unwrap();
                tmp0.zero_resize_assign(out);
                tmp1.field_from(lut, i * out.bw(), out.bw()).unwrap();
                assert_eq!(tmp0, tmp1);
            }
        }
    }
}

/// Test [Bits::lut] and [Bits::lut_set]
#[test]
fn lut_and_lut_set() {
    let mut rng = Xoshiro128StarStar::seed_from_u64(0);
    #[cfg(not(miri))]
    let (entry_bw_max, pow_max) = (258, 8);
    #[cfg(miri)]
    let (entry_bw_max, pow_max) = (68, 3);
    for entry_bw in 1..entry_bw_max {
        let mut awi_entry = ExtAwi::zero(bw(entry_bw));
        let entry = awi_entry.const_as_mut();
        let mut awi_entry_copy = ExtAwi::zero(entry.nzbw());
        let entry_copy = awi_entry_copy.const_as_mut();
        let mut awi_entry_old = ExtAwi::zero(entry.nzbw());
        let entry_old = awi_entry_old.const_as_mut();
        for pow in 1..pow_max {
            let mul = 1 << pow;
            let mut awi_lut = ExtAwi::zero(bw(entry_bw * mul));
            let lut = awi_lut.const_as_mut();
            lut.rand_assign_using(&mut rng).unwrap();
            let mut awi_lut_copy = ExtAwi::zero(lut.nzbw());
            let lut_copy = awi_lut_copy.const_as_mut();
            lut_copy.copy_assign(lut).unwrap();
            let mut awi_inx = ExtAwi::zero(bw(pow));
            let inx = awi_inx.const_as_mut();
            for _ in 0..mul {
                inx.rand_assign_using(&mut rng).unwrap();
                entry.rand_assign_using(&mut rng).unwrap();
                entry_copy.copy_assign(entry).unwrap();
                // before `lut_set`, copy the old entry
                entry_old.lut(lut, inx).unwrap();
                // set new value
                lut.lut_set(entry, inx).unwrap();
                // get the value that was set
                entry.lut(lut, inx).unwrap();
                assert_eq!(entry, entry_copy);
                // restore to original state and make sure nothing else was overwritten
                lut.lut_set(entry_old, inx).unwrap();
                assert_eq!(lut, lut_copy);
            }
        }
    }
}

#[test]
fn funnel() {
    let mut rng = Xoshiro128StarStar::seed_from_u64(0);
    #[cfg(miri)]
    let max_pow = 7;
    #[cfg(not(miri))]
    let max_pow = 10;
    for pow in 1..max_pow {
        let mut awi_shift = ExtAwi::zero(bw(pow));
        let mut awi_lhs = ExtAwi::zero(bw(1 << pow));
        let mut awi_rhs = ExtAwi::zero(bw(2 * awi_lhs.bw()));
        let mut awi_alt0 = ExtAwi::zero(bw(awi_rhs.bw()));
        let mut awi_alt1 = ExtAwi::zero(bw(awi_lhs.bw()));
        awi_rhs.rand_assign_using(&mut rng).unwrap();
        let shift = awi_shift.const_as_mut();
        let lhs = awi_lhs.const_as_mut();
        let rhs = awi_rhs.const_as_mut();
        let alt0 = awi_alt0.const_as_mut();
        let alt1 = awi_alt1.const_as_mut();
        for s in 0..lhs.bw() {
            alt0.copy_assign(rhs).unwrap();
            alt0.lshr_assign(s).unwrap();
            alt1.zero_resize_assign(alt0);
            shift.usize_assign(s);
            lhs.funnel(rhs, shift).unwrap();
            assert_eq!(lhs, alt1);
        }
    }
}

#[test]
fn awint_internals_test() {
    let mut rng = &mut Xoshiro128StarStar::seed_from_u64(0);
    let mut lhs = inlawi!(0u128);
    let mut rhs = inlawi!(0u128);
    let mut add = inlawi!(0u128);
    lhs.rand_assign_using(&mut rng).unwrap();
    rhs.rand_assign_using(&mut rng).unwrap();
    add.rand_assign_using(&mut rng).unwrap();
    let (lo, hi) =
        awint_internals::widening_mul_add_u128(lhs.to_u128(), rhs.to_u128(), add.to_u128());
    let mut tmp0 = extawi!(0u128);
    let mut tmp1 = extawi!(0u128);
    tmp0.u128_assign(lo);
    tmp1.u128_assign(hi);
    let lhs = inlawi!(zero: ..,lhs;..256).unwrap();
    let rhs = inlawi!(zero: ..,rhs;..256).unwrap();
    let mut add = inlawi!(zero: ..,add;..256).unwrap();
    add.mul_add_assign(&lhs, &rhs).unwrap();
    assert_eq!(&extawi!(tmp1, tmp0)[..], &add[..]);
}

#[test]
fn from_primitive() {
    assert_eq!(InlAwi::from_u8(u8::MAX), inlawi!(umax: ..8));
    assert_eq!(InlAwi::from_u16(u16::MAX), inlawi!(umax: ..16));
    assert_eq!(InlAwi::from_u32(u32::MAX), inlawi!(umax: ..32));
    assert_eq!(InlAwi::from_u64(u64::MAX), inlawi!(umax: ..64));
    assert_eq!(InlAwi::from_u128(u128::MAX), inlawi!(umax: ..128));
    assert_eq!(InlAwi::from_i8(i8::MAX), inlawi!(imax: ..8));
    assert_eq!(InlAwi::from_i16(i16::MAX), inlawi!(imax: ..16));
    assert_eq!(InlAwi::from_i32(i32::MAX), inlawi!(imax: ..32));
    assert_eq!(InlAwi::from_i64(i64::MAX), inlawi!(imax: ..64));
    assert_eq!(InlAwi::from_i128(i128::MAX), inlawi!(imax: ..128));
}
