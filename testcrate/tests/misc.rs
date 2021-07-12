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
            awi_lut[..].rand_assign_using(&mut rng).unwrap();
            let mut awi_inx = ExtAwi::zero(bw(pow));
            let out = awi_out.const_as_mut();
            let lut = awi_lut.const_as_ref();
            let inx = awi_inx.const_as_mut();
            for i in 0..mul {
                inx.usize_assign(i);
                out.lut(lut, inx).unwrap();
                tmp0.zero_resize_assign(out);
                tmp1.field(0, lut, i * out.bw(), out.bw()).unwrap();
                assert_eq!(tmp0, tmp1);
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
        awi_rhs[..].rand_assign_using(&mut rng).unwrap();
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
