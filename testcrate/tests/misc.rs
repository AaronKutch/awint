use awint::{
    awi::*,
    awint_internals::{Digit, BITS, USIZE_BITS},
};
use rand_xoshiro::{
    rand_core::{RngCore, SeedableRng},
    Xoshiro128StarStar,
};

/// [Bits::lut_] needs its own test because of its special requirements
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
            awi_lut.rand_(&mut rng);
            let mut awi_inx = ExtAwi::zero(bw(pow));
            let out = awi_out.const_as_mut();
            let lut = awi_lut.as_ref();
            let inx = awi_inx.as_mut();
            for i in 0..mul {
                inx.usize_(i);
                out.lut_(lut, inx).unwrap();
                tmp0.zero_resize_(out);
                tmp1.field_from(lut, i * out.bw(), out.bw()).unwrap();
                assert_eq!(tmp0, tmp1);
            }
        }
    }
}

/// Test [Bits::lut_] and [Bits::lut_set]
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
            lut.rand_(&mut rng);
            let mut awi_lut_copy = ExtAwi::zero(lut.nzbw());
            let lut_copy = awi_lut_copy.const_as_mut();
            lut_copy.copy_(lut).unwrap();
            let mut awi_inx = ExtAwi::zero(bw(pow));
            let inx = awi_inx.const_as_mut();
            for _ in 0..mul {
                inx.rand_(&mut rng);
                entry.rand_(&mut rng);
                entry_copy.copy_(entry).unwrap();
                // before `lut_set`, copy the old entry
                entry_old.lut_(lut, inx).unwrap();
                // set new value
                lut.lut_set(entry, inx).unwrap();
                // get the value that was set
                entry.lut_(lut, inx).unwrap();
                assert_eq!(entry, entry_copy);
                // restore to original state and make sure nothing else was overwritten
                lut.lut_set(entry_old, inx).unwrap();
                assert_eq!(lut, lut_copy);
            }
        }
    }
}

#[test]
fn funnel_() {
    let mut rng = Xoshiro128StarStar::seed_from_u64(0);
    #[cfg(miri)]
    let max_pow = 7;
    #[cfg(not(miri))]
    let max_pow = 10; // note this also is over BITS when `u8_digits` is active
    for pow in 1..max_pow {
        let mut awi_shift = ExtAwi::zero(bw(pow));
        let mut awi_lhs = ExtAwi::zero(bw(1 << pow));
        let mut awi_rhs = ExtAwi::zero(bw(2 * awi_lhs.bw()));
        let mut awi_alt0 = ExtAwi::zero(bw(awi_rhs.bw()));
        let mut awi_alt1 = ExtAwi::zero(bw(awi_lhs.bw()));
        awi_rhs.rand_(&mut rng);
        let shift = awi_shift.const_as_mut();
        let lhs = awi_lhs.const_as_mut();
        let rhs = awi_rhs.const_as_mut();
        let alt0 = awi_alt0.const_as_mut();
        let alt1 = awi_alt1.const_as_mut();
        for s in 0..lhs.bw() {
            alt0.copy_(rhs).unwrap();
            alt0.lshr_(s).unwrap();
            alt1.zero_resize_(alt0);
            shift.usize_(s);
            lhs.funnel_(rhs, shift).unwrap();
            assert_eq!(lhs, alt1);
        }
    }
}

macro_rules! test_unstable_from_u8_slice {
    ($buf:ident, $($ty:expr)*) => {
        $(
            let x: inlawi_ty!($ty) = InlAwi::unstable_from_u8_slice($buf);
            InlAwi::assert_invariants(&x);
        )*
    };
}

#[test]
fn awint_internals_test() {
    let mut rng = &mut Xoshiro128StarStar::seed_from_u64(0);
    let mut lhs = inlawi!(0u128);
    let mut rhs = inlawi!(0u128);
    let mut add = inlawi!(0u128);
    lhs.rand_(&mut rng);
    rhs.rand_(&mut rng);
    add.rand_(&mut rng);
    let (lo, hi) =
        awint::awint_internals::widening_mul_add_u128(lhs.to_u128(), rhs.to_u128(), add.to_u128());
    let mut tmp0 = extawi!(0u128);
    let mut tmp1 = extawi!(0u128);
    tmp0.u128_(lo);
    tmp1.u128_(hi);
    let lhs = inlawi!(zero: ..,lhs;..256).unwrap();
    let rhs = inlawi!(zero: ..,rhs;..256).unwrap();
    let mut add = inlawi!(zero: ..,add;..256).unwrap();
    add.mul_add_(&lhs, &rhs).unwrap();
    assert_eq!(&extawi!(tmp1, tmp0)[..], &add[..]);

    let mut buf = [0u8; 68];
    for x in &mut buf {
        *x = rng.next_u32() as u8;
    }
    for i in 0..buf.len() {
        // test `unstable_from_u8_slice` directly because the macros won't test some
        // cases
        let buf = &buf[0..i];
        test_unstable_from_u8_slice!(buf, 1 7 8 9 15 16 17 31 32 33 63 64 65 127 128 129 258);
    }
}

#[test]
fn from_primitive() {
    assert_eq!(InlAwi::from_bool(true), inlawi!(umax: ..1));
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
    assert_eq!(InlAwi::from(true), inlawi!(umax: ..1));
    assert_eq!(InlAwi::from(u8::MAX), inlawi!(umax: ..8));
    assert_eq!(InlAwi::from(u16::MAX), inlawi!(umax: ..16));
    assert_eq!(InlAwi::from(u32::MAX), inlawi!(umax: ..32));
    assert_eq!(InlAwi::from(u64::MAX), inlawi!(umax: ..64));
    assert_eq!(InlAwi::from(u128::MAX), inlawi!(umax: ..128));
    assert_eq!(InlAwi::from(i8::MAX), inlawi!(imax: ..8));
    assert_eq!(InlAwi::from(i16::MAX), inlawi!(imax: ..16));
    assert_eq!(InlAwi::from(i32::MAX), inlawi!(imax: ..32));
    assert_eq!(InlAwi::from(i64::MAX), inlawi!(imax: ..64));
    assert_eq!(InlAwi::from(i128::MAX), inlawi!(imax: ..128));

    assert_eq!(InlAwi::from_usize(usize::MAX).bw(), USIZE_BITS);
    assert_eq!(InlAwi::from_isize(isize::MAX).bw(), USIZE_BITS);
    assert_eq!(InlAwi::from_digit(Digit::MAX).bw(), BITS);
    assert_eq!(InlAwi::from(usize::MAX).bw(), USIZE_BITS);
    assert_eq!(InlAwi::from(isize::MAX).bw(), USIZE_BITS);
    assert_eq!(InlAwi::from(Digit::MAX).bw(), BITS);
    assert_eq!(InlAwi::from_usize(usize::MAX).to_usize(), usize::MAX);
    assert_eq!(InlAwi::from_isize(isize::MAX).to_isize(), isize::MAX);
    assert_eq!(InlAwi::from_digit(Digit::MAX).to_digit(), Digit::MAX);
    assert_eq!(InlAwi::from(usize::MAX).to_usize(), usize::MAX);
    assert_eq!(InlAwi::from(isize::MAX).to_isize(), isize::MAX);
    assert_eq!(InlAwi::from(Digit::MAX).to_digit(), Digit::MAX);

    assert_eq!(ExtAwi::from_bool(true), extawi!(umax: ..1));
    assert_eq!(ExtAwi::from_u8(u8::MAX), extawi!(umax: ..8));
    assert_eq!(ExtAwi::from_u16(u16::MAX), extawi!(umax: ..16));
    assert_eq!(ExtAwi::from_u32(u32::MAX), extawi!(umax: ..32));
    assert_eq!(ExtAwi::from_u64(u64::MAX), extawi!(umax: ..64));
    assert_eq!(ExtAwi::from_u128(u128::MAX), extawi!(umax: ..128));
    assert_eq!(
        ExtAwi::from_usize(usize::MAX),
        extawi!(umax: ..USIZE_BITS).unwrap()
    );
    assert_eq!(ExtAwi::from_i8(i8::MAX), extawi!(imax: ..8));
    assert_eq!(ExtAwi::from_i16(i16::MAX), extawi!(imax: ..16));
    assert_eq!(ExtAwi::from_i32(i32::MAX), extawi!(imax: ..32));
    assert_eq!(ExtAwi::from_i64(i64::MAX), extawi!(imax: ..64));
    assert_eq!(ExtAwi::from_i128(i128::MAX), extawi!(imax: ..128));
    assert_eq!(
        ExtAwi::from_isize(isize::MAX),
        extawi!(imax: ..(isize::BITS as usize)).unwrap()
    );
    assert_eq!(
        ExtAwi::from_digit(Digit::MAX),
        extawi!(umax: ..BITS).unwrap()
    );
    assert_eq!(ExtAwi::from(true), extawi!(umax: ..1));
    assert_eq!(ExtAwi::from(u8::MAX), extawi!(umax: ..8));
    assert_eq!(ExtAwi::from(u16::MAX), extawi!(umax: ..16));
    assert_eq!(ExtAwi::from(u32::MAX), extawi!(umax: ..32));
    assert_eq!(ExtAwi::from(u64::MAX), extawi!(umax: ..64));
    assert_eq!(ExtAwi::from(u128::MAX), extawi!(umax: ..128));
    assert_eq!(
        ExtAwi::from(usize::MAX),
        extawi!(umax: ..USIZE_BITS).unwrap()
    );
    assert_eq!(ExtAwi::from(i8::MAX), extawi!(imax: ..8));
    assert_eq!(ExtAwi::from(i16::MAX), extawi!(imax: ..16));
    assert_eq!(ExtAwi::from(i32::MAX), extawi!(imax: ..32));
    assert_eq!(ExtAwi::from(i64::MAX), extawi!(imax: ..64));
    assert_eq!(ExtAwi::from(i128::MAX), extawi!(imax: ..128));
    assert_eq!(
        ExtAwi::from(isize::MAX),
        extawi!(imax: ..(isize::BITS as usize)).unwrap()
    );
    assert_eq!(ExtAwi::from(Digit::MAX), extawi!(umax: ..BITS).unwrap());

    assert_eq!(Awi::from_bool(true), awi!(umax: ..1));
    assert_eq!(Awi::from_u8(u8::MAX), awi!(umax: ..8));
    assert_eq!(Awi::from_u16(u16::MAX), awi!(umax: ..16));
    assert_eq!(Awi::from_u32(u32::MAX), awi!(umax: ..32));
    assert_eq!(Awi::from_u64(u64::MAX), awi!(umax: ..64));
    assert_eq!(Awi::from_u128(u128::MAX), awi!(umax: ..128));
    assert_eq!(
        Awi::from_usize(usize::MAX),
        awi!(umax: ..USIZE_BITS).unwrap()
    );
    assert_eq!(Awi::from_i8(i8::MAX), awi!(imax: ..8));
    assert_eq!(Awi::from_i16(i16::MAX), awi!(imax: ..16));
    assert_eq!(Awi::from_i32(i32::MAX), awi!(imax: ..32));
    assert_eq!(Awi::from_i64(i64::MAX), awi!(imax: ..64));
    assert_eq!(Awi::from_i128(i128::MAX), awi!(imax: ..128));
    assert_eq!(
        Awi::from_isize(isize::MAX),
        awi!(imax: ..(isize::BITS as usize)).unwrap()
    );
    assert_eq!(Awi::from_digit(Digit::MAX), awi!(umax: ..BITS).unwrap());
    assert_eq!(Awi::from(true), awi!(umax: ..1));
    assert_eq!(Awi::from(u8::MAX), awi!(umax: ..8));
    assert_eq!(Awi::from(u16::MAX), awi!(umax: ..16));
    assert_eq!(Awi::from(u32::MAX), awi!(umax: ..32));
    assert_eq!(Awi::from(u64::MAX), awi!(umax: ..64));
    assert_eq!(Awi::from(u128::MAX), awi!(umax: ..128));
    assert_eq!(Awi::from(usize::MAX), awi!(umax: ..USIZE_BITS).unwrap());
    assert_eq!(Awi::from(i8::MAX), awi!(imax: ..8));
    assert_eq!(Awi::from(i16::MAX), awi!(imax: ..16));
    assert_eq!(Awi::from(i32::MAX), awi!(imax: ..32));
    assert_eq!(Awi::from(i64::MAX), awi!(imax: ..64));
    assert_eq!(Awi::from(i128::MAX), awi!(imax: ..128));
    assert_eq!(
        Awi::from(isize::MAX),
        awi!(imax: ..(isize::BITS as usize)).unwrap()
    );
    assert_eq!(Awi::from(Digit::MAX), awi!(umax: ..BITS).unwrap());
}
