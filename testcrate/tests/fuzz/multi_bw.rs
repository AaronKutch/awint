use core::cmp;
use std::cmp::min;

use awint::{bw, cc, Bits, ExtAwi, InlAwi};
use rand_xoshiro::{
    rand_core::{RngCore, SeedableRng},
    Xoshiro128StarStar,
};

use crate::fuzz::{eq, fuzz_step, ne};

fn multi_bw_inner(
    #[allow(unused_variables)] // for miri
    rng: &mut Xoshiro128StarStar,
    x0bw0: &Bits,
    x1bw0: &mut Bits,
    x2bw0: &mut Bits,
    x3bw0: &mut Bits,
    x0bw1: &Bits,
    x1bw1: &mut Bits,
    x2bw1: &mut Bits,
    x0bw2: &Bits,
    x1bw2: &mut Bits,
    b: bool,
) -> Option<()> {
    let bw0 = x0bw0.bw();
    let bw1 = x0bw1.bw();

    // basic resize assign
    x1bw1.resize_(x0bw0, b);
    x1bw0.resize_(x1bw1, b);
    if !x0bw0.const_eq(x1bw0)? {
        assert!(bw1 < bw0);
    }
    if bw0 <= bw1 {
        // only truncation should be lossy
        eq(x0bw0, x1bw0);
    } else if b {
        x2bw0.umax_();
        x2bw0.shl_(bw1)?;
        x2bw0.or_(x0bw0)?;
        eq(x1bw0, x2bw0);
    } else {
        x2bw0.copy_(x0bw0)?;
        x2bw0.range_and_(0..bw1)?;
        eq(x1bw0, x2bw0);
    }

    // zero resize assign
    let o0 = x1bw1.zero_resize_(x0bw0);
    let o1 = x1bw0.zero_resize_(x1bw1);
    // the overflow should only occur the first time if it does
    assert!(!o1);
    if o0 {
        assert!(bw1 < bw0);
        ne(x0bw0, x1bw0)
    } else {
        eq(x0bw0, x1bw0)
    }

    // sign resize assign
    let o0 = x1bw1.sign_resize_(x0bw0);
    let o1 = x1bw0.sign_resize_(x1bw1);
    assert!(!o1);
    if o0 {
        assert!(bw1 < bw0);
        ne(x0bw0, x1bw0)
    } else {
        eq(x0bw0, x1bw0)
    }

    // bitfields
    let width = (rng.next_u32() as usize) % (cmp::min(bw0, bw1) + 1);
    let from = if bw0 - width == 0 {
        0
    } else {
        (rng.next_u32() as usize) % (bw0 - width)
    };
    let to = if bw1 - width == 0 {
        0
    } else {
        (rng.next_u32() as usize) % (bw1 - width)
    };
    // set x2bw1 to what x1bw1 will be without the copied field
    x1bw1.copy_(x0bw1)?;
    x1bw1.range_and_(0..to).unwrap();
    x2bw1.copy_(x0bw1)?;
    x2bw1.range_and_((to + width)..bw1).unwrap();
    x2bw1.or_(x1bw1)?;
    // set x1bw1 to what the x1bw1 will be with only the copied field
    x1bw0.copy_(x0bw0)?;
    x1bw0.range_and_(from..(from + width)).unwrap();
    x1bw0.lshr_(from).unwrap();
    x1bw1.zero_resize_(x1bw0);
    x1bw1.shl_(to).unwrap();
    // combine
    x2bw1.or_(x1bw1).unwrap();
    // x1bw1 is done being used as a temporary
    x1bw1.copy_(x0bw1)?;
    x1bw1.field(to, x0bw0, from, width).unwrap();
    eq(x1bw1, x2bw1);
    x1bw1.field(0, x0bw0, from, width).unwrap();
    x2bw1.field_from(x0bw0, from, width).unwrap();
    eq(x1bw1, x2bw1);
    x1bw1.field(to, x0bw0, 0, width).unwrap();
    x2bw1.field_to(to, x0bw0, width).unwrap();
    eq(x1bw1, x2bw1);
    x1bw1.field(0, x0bw0, 0, width).unwrap();
    x2bw1.field_width(x0bw0, width).unwrap();
    eq(x1bw1, x2bw1);
    x1bw1.field(to, x0bw0, from, 1).unwrap();
    x2bw1.field_bit(to, x0bw0, from).unwrap();
    eq(x1bw1, x2bw1);

    // repeat_
    let mut to = 0;
    loop {
        if to > bw1 {
            break
        }
        x1bw1.field_to(to, x0bw0, min(bw0, bw1 - to)).unwrap();
        to += bw0;
    }
    x2bw1.repeat_(x0bw0);
    eq(x1bw1, x2bw1);

    // arbitrary width multiplication
    x1bw0.copy_(x0bw0)?;
    x2bw0.zero_resize_(x0bw1);
    x3bw0.zero_resize_(x0bw2);
    x1bw0.mul_add_(x2bw0, x3bw0)?;
    x2bw0.copy_(x0bw0)?;
    x2bw0.arb_umul_add_(x0bw1, x0bw2);
    eq(x1bw0, x2bw0);
    // signed version
    x1bw0.copy_(x0bw0)?;
    x2bw0.sign_resize_(x0bw1);
    x3bw0.sign_resize_(x0bw2);
    x1bw0.mul_add_(x2bw0, x3bw0)?;
    x2bw0.copy_(x0bw0)?;
    x1bw1.copy_(x0bw1)?;
    x1bw2.copy_(x0bw2)?;
    x2bw0.arb_imul_add_(x1bw1, x1bw2);
    // make sure it did not mutate these arguments
    eq(x1bw1, x0bw1);
    eq(x1bw2, x0bw2);
    eq(x1bw0, x2bw0);

    // no unsafe code being used in these functions, disabling because it is too
    // slow
    #[cfg(not(miri))]
    {
        // testing `ExtAwi::from_bytes_radix`
        let radix = ((rng.next_u32() % 35) + 2) as u8;
        let tmp_rng = rng.next_u32();
        let sign = if (tmp_rng & 0b1) != 0 {
            None
        } else {
            Some(x0bw0.msb())
        };
        let min_chars = if (tmp_rng & 0b10) != 0 {
            rng.next_u32() as usize % 258
        } else {
            0
        };
        let string = ExtAwi::bits_to_string_radix(
            x0bw0,
            sign.is_some(),
            radix,
            (tmp_rng & 0b100) != 0,
            min_chars,
        )
        .unwrap();
        match ExtAwi::from_str_radix(sign, &string, radix, bw(bw1)) {
            Ok(awi) => {
                if sign.is_none() {
                    x1bw1.zero_resize_(x0bw0);
                } else {
                    x1bw1.sign_resize_(x0bw0);
                }
                eq(x1bw1, &awi);
            }
            Err(e) => {
                // this should be the only error we will encounter
                assert!(matches!(e, awint::SerdeError::Overflow));
                assert!(bw1 < bw0);
            }
        }
    }

    Some(())
}

/// For testing operations with multiple bitwidths
pub fn multi_bw(seed: u64) -> Option<()> {
    // the seed makes sure that any repeats explore new space
    let rng = &mut Xoshiro128StarStar::seed_from_u64(seed);

    let bw0 = bw(((rng.next_u32() % 258) + 1) as usize);
    let bw1 = bw(((rng.next_u32() % 258) + 1) as usize);
    let bw2 = bw(((rng.next_u32() % 258) + 1) as usize);
    let mut awi0_0 = ExtAwi::zero(bw0);
    let mut awi1_0 = ExtAwi::zero(bw0);
    let mut awi2_0 = ExtAwi::zero(bw0);
    let mut awi3_0 = ExtAwi::zero(bw0);
    let mut awi0_1 = ExtAwi::zero(bw1);
    let mut awi1_1 = ExtAwi::zero(bw1);
    let mut awi2_1 = ExtAwi::zero(bw1);
    let mut awi0_2 = ExtAwi::zero(bw2);
    let mut awi1_2 = ExtAwi::zero(bw2);
    // only the `x0_`s are being randomized, the `x1_`s however are needed for fuzz
    // step temporaries
    let x0bw0 = awi0_0.const_as_mut();
    let x1bw0 = awi1_0.const_as_mut();
    let x2bw0 = awi2_0.const_as_mut();
    let x3bw0 = awi3_0.const_as_mut();
    let x0bw1 = awi0_1.const_as_mut();
    let x1bw1 = awi1_1.const_as_mut();
    let x2bw1 = awi2_1.const_as_mut();
    let x0bw2 = awi0_2.const_as_mut();
    let x1bw2 = awi1_2.const_as_mut();

    for _ in 0..16 {
        fuzz_step(rng, x0bw0, x1bw0);
        fuzz_step(rng, x0bw1, x1bw1);
        fuzz_step(rng, x0bw2, x1bw2);
        let b = (rng.next_u32() & 1) == 0;

        multi_bw_inner(
            rng, x0bw0, x1bw0, x2bw0, x3bw0, x0bw1, x1bw1, x2bw1, x0bw2, x1bw2, b,
        )?;
    }
    Some(())
}

#[test]
pub fn u8_slice() {
    let mut rng = &mut Xoshiro128StarStar::seed_from_u64(0);
    let mut array = [0u8; 65];
    for _ in 0..10 {
        if (rng.next_u32() & 1) != 0 {
            array.fill(rng.next_u32() as u8);
        }
        let w = ((rng.next_u32() as usize) % 258) + 1;
        let mut x0 = ExtAwi::zero(bw(w));
        let mut x1 = ExtAwi::zero(bw(w));
        let inx = (rng.next_u32() as usize) % ((2 * (258 / 8)) + 1);
        x0.u8_slice_(&array[..inx]);
        assert!(x0.sig() <= (inx * 8));
        for (i, byte) in array.iter().take(inx).enumerate() {
            let r0 = cmp::min(i * 8, w);
            let r1 = cmp::min(r0 + 8, w);
            cc!(InlAwi::from_u8(*byte); .., x1[r0..r1]; ..8).unwrap();
        }
        assert_eq!(x0, x1);
        x0.rand_(&mut rng);
        x0.to_u8_slice(&mut array[..inx]);
        let mut byte = InlAwi::from_u8(0);
        for (i, item) in array.iter().take(inx).enumerate() {
            let r0 = cmp::min(i * 8, w);
            let r1 = cmp::min(r0 + 8, w);
            // automatically tests for zeros
            cc!(zero: .., x0[r0..r1]; byte; ..8).unwrap();
            assert_eq!(*item, byte.to_u8());
        }
    }
}
