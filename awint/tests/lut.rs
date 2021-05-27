#![cfg(feature = "rand_support")]

use awint::prelude::*;
use rand_xoshiro::{rand_core::SeedableRng, Xoshiro128StarStar};

/// [Bits::lut] needs its own test because of its special requirements
#[test]
fn lut() {
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
