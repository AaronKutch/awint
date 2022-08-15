#![feature(const_mut_refs)]
#![feature(const_option)]
#![feature(const_trait_impl)]
#![allow(clippy::reversed_empty_ranges)]

use awint::prelude::{bw, cc, inlawi, inlawi_ty, Bits, InlAwi};

const fn check_invariants(x: &Bits) {
    if x.extra() != 0 && (x.last() & (usize::MAX << x.extra())) != 0 {
        panic!("unused bits are set");
    }
}

/// Checks for equality and that invariants are being kept
const fn eq(lhs: &Bits, rhs: &Bits) {
    check_invariants(lhs);
    check_invariants(rhs);
    if !lhs.const_eq(rhs).unwrap() {
        panic!("lhs and rhs are not equal when they should be")
    }
}

/// The purpose of this test is to supply some actual test values to some
/// functions to make sure `fuzz.rs` isn't running into false positives.
#[test]
const fn consts() {
    let mut awi0: inlawi_ty!(256) = InlAwi::zero();
    let mut awi1: inlawi_ty!(256) = InlAwi::zero();
    //let mut awi2 = InlAwi::<5>::zero();
    let x: &mut Bits = awi0.const_as_mut();
    let y: &mut Bits = awi1.const_as_mut();
    //let z: &mut Bits = awi2.const_as_mut();
    x.u128_assign(123456789);
    y.u128_assign(9876543211);
    x.add_assign(y).unwrap();
    y.u128_assign(10000000000);
    eq(x, y);

    let a1337: inlawi_ty!(12) = inlawi!(1337u12);
    let mut b1337: inlawi_ty!(12) = inlawi!(010100111001);
    let c_100: inlawi_ty!(12) = inlawi!(100i12);
    let d1437: inlawi_ty!(12) = inlawi!(1437u12);
    eq(a1337.const_as_ref(), b1337.const_as_ref());
    let sum = b1337.const_as_mut();
    sum.add_assign(c_100.const_as_ref()).unwrap();
    eq(sum, d1437.const_as_ref());
    let e1337: inlawi_ty!(12) = inlawi!(0101, 0011, 1001);
    eq(a1337.const_as_ref(), e1337.const_as_ref());

    let y3 = inlawi!(0xba9u12);
    let y2 = inlawi!(0x876u12);
    let y1 = inlawi!(0x543u12);
    let y0 = inlawi!(0x210u12);

    let mut z2 = inlawi!(0u16);
    let mut z1 = inlawi!(0u16);
    let mut z0 = inlawi!(0u16);
    let r0 = 0;
    let r1 = 12;

    cc!(
        y3, y2[r0..r1], y1, y0;
        z2, z1, z0;
        ..48;
    )
    .unwrap();

    eq(z2.const_as_ref(), inlawi!(0xba98u16).const_as_ref());
    eq(z1.const_as_ref(), inlawi!(0x7654u16).const_as_ref());
    eq(z0.const_as_ref(), inlawi!(0x3210u16).const_as_ref());
}

#[test]
#[should_panic]
const fn bw_panics() {
    let _ = bw(0);
}

macro_rules! test_nonequal_bw {
    (
        $x0:ident, $x1:ident;
        $($fn_unary:ident)*;
        $($fn_unary_shift:ident)*;
        $($fn_unary_literal:ident)*;
        $($fn_binary:ident)*
    ) => {
        $(
            let _ = $x0.$fn_unary(); // Just checking that the function exists and is constant
        )*
        $(
            assert!($x0.$fn_unary_shift($x0.bw() - 1).is_some());
            assert!($x0.$fn_unary_shift($x0.bw()).is_none());
        )*
        $(
            $x0.$fn_unary_literal(1);
        )*
        $(
            assert!($x0.$fn_binary($x1).is_none());
        )*
    }
}

/// This test checks that all appropriate functions on `Bits` exist, are const,
/// and checks `None` return cases.
#[test]
const fn bits_functions() {
    // these macros also test the corresponding `InlAwi` functions
    let mut awi0 = inlawi!(zero: ..128);
    let mut awi1 = inlawi!(umax: ..192);
    let mut awi2 = inlawi!(imax: ..192);
    let mut awi3 = inlawi!(imin: ..192);
    let mut awi4 = inlawi!(uone: ..192);
    let x0 = awi0.const_as_mut();
    let x1 = awi1.const_as_mut();
    let x2 = awi2.const_as_mut();
    let x3 = awi3.const_as_mut();
    let x4 = awi4.const_as_mut();

    // test the inlawi macros first
    assert!(x0.is_zero());
    assert!(x1.is_umax());
    assert!(x2.is_imax());
    assert!(x3.is_imin());
    assert!(x4.is_uone());

    // miscellanious functions that won't work with the macro

    assert!(x0.range_and_assign(0..128).is_some());
    assert!(x0.range_and_assign(127..0).is_some());
    assert!(x0.range_and_assign(128..128).is_some());
    assert!(x0.range_and_assign(129..129).is_none());
    // we want to test this reversed range specifically
    assert!(x0.range_and_assign(129..128).is_none());
    assert!(x0.range_and_assign(0..129).is_none());

    assert!(x0.field(0, x1, 0, 128).is_some());
    assert!(x0.field(0, x1, 64, 128).is_some());
    assert!(x0.field(0, x1, 0, 129).is_none());
    assert!(x0.field(1, x1, 0, 128).is_none());
    assert!(x1.field(0, x0, 0, 128).is_some());
    assert!(x1.field(64, x0, 0, 128).is_some());
    assert!(x1.field(0, x0, 0, 129).is_none());
    assert!(x1.field(0, x0, 1, 128).is_none());
    assert!(x0.field(128, x1, 192, 0).is_some());
    assert!(x0.field(129, x1, 192, 0).is_none());
    assert!(x0.field(128, x1, 193, 0).is_none());
    assert!(x0.field_to(0, x1, 128).is_some());
    assert!(x0.field_to(1, x1, 128).is_none());
    assert!(x0.field_to(0, x1, 129).is_none());
    assert!(x0.field_to(128, x1, 0).is_some());
    assert!(x1.field_to(0, x0, 128).is_some());
    assert!(x1.field_to(64, x0, 128).is_some());
    assert!(x1.field_to(0, x0, 129).is_none());
    assert!(x1.field_to(192, x0, 0).is_some());
    assert!(x0.field_from(x1, 0, 128).is_some());
    assert!(x0.field_from(x1, 64, 128).is_some());
    assert!(x0.field_from(x1, 65, 128).is_none());
    assert!(x0.field_from(x1, 0, 129).is_none());
    assert!(x0.field_from(x1, 192, 0).is_some());
    assert!(x1.field_from(x0, 0, 128).is_some());
    assert!(x1.field_from(x0, 65, 128).is_none());
    assert!(x1.field_from(x0, 64, 129).is_none());
    assert!(x1.field_from(x0, 128, 0).is_some());
    assert!(x0.field_width(x1, 0).is_some());
    assert!(x0.field_width(x1, 128).is_some());
    assert!(x0.field_width(x1, 129).is_none());
    assert!(x1.field_width(x0, 0).is_some());
    assert!(x1.field_width(x0, 128).is_some());
    assert!(x1.field_width(x0, 129).is_none());
    assert!(x0.field_bit(0, x1, 0).is_some());
    assert!(x0.field_bit(127, x1, 191).is_some());
    assert!(x0.field_bit(128, x1, 191).is_none());
    assert!(x0.field_bit(127, x1, 192).is_none());

    assert!(x0.lut(x1, x3).is_none());
    assert!(x0.funnel(x1, x3).is_none());

    x0.short_cin_mul(0, 0);

    assert!(x0.mul_add_assign(x1, x2).is_none());
    assert!(x1.mul_add_assign(x0, x2).is_none());
    assert!(x2.mul_add_assign(x1, x0).is_none());
    assert!(x0.mul_assign(x1, x2).is_none());
    assert!(x1.mul_assign(x0, x2).is_none());
    assert!(x2.mul_assign(x1, x0).is_none());

    x0.arb_umul_add_assign(x1, x2);
    x0.arb_imul_add_assign(x1, x2);

    x0.bool_assign(true);

    x0.inc_assign(false);
    x0.dec_assign(true);
    x0.neg_assign(false);
    assert!(x0.neg_add_assign(false, x1).is_none());
    assert!(x0.cin_sum_assign(false, x1, x2).is_none());

    x0.usize_or_assign(123, 60);

    // division by zero and differing size
    x1.umax_assign();
    x2.umax_assign();
    x3.umax_assign();
    x4.zero_assign();
    assert!(Bits::udivide(x1, x2, x3, x4).is_none());
    x0.umax_assign();
    x1.umax_assign();
    x2.umax_assign();
    x3.umax_assign();
    assert!(Bits::udivide(x0, x1, x2, x3).is_none());
    x1.umax_assign();
    x2.umax_assign();
    x3.umax_assign();
    x4.zero_assign();
    assert!(Bits::idivide(x1, x2, x3, x4).is_none());
    x0.umax_assign();
    x1.umax_assign();
    x2.umax_assign();
    x3.umax_assign();
    assert!(Bits::idivide(x0, x1, x2, x3).is_none());
    x1.umax_assign();
    assert!(x4.short_udivide_assign(x1, 0).is_none());
    x0.umax_assign();
    assert!(x4.short_udivide_assign(x0, 1).is_none());
    assert!(x4.short_udivide_inplace_assign(0).is_none());

    assert!(x0.get(128).is_none());
    assert!(x0.set(128, false).is_none());

    assert!(x0.mux(x1, x2, false).is_none());
    assert!(x2.mux(x0, x1, false).is_none());
    assert!(x2.mux(x1, x0, false).is_none());

    // TODO test all const serialization

    test_nonequal_bw!(
        x0, x1
        ;// functions with signature `fn({ , &, &mut} self) -> ...`
        nzbw
        bw
        len
        unused
        extra
        first
        first_mut
        last
        last_mut
        clear_unused_bits
        as_slice
        as_mut_slice
        zero_assign
        umax_assign
        imax_assign
        imin_assign
        uone_assign
        not_assign
        is_zero
        is_umax
        is_imin
        is_uone
        lsb
        msb
        lz
        tz
        count_ones
        rev_assign
        to_usize
        to_isize
        to_u8
        to_i8
        to_u16
        to_i16
        to_u32
        to_i32
        to_u64
        to_i64
        to_u128
        to_i128
        to_bool
        abs_assign
        ;
        shl_assign
        lshr_assign
        ashr_assign
        rotl_assign
        rotr_assign
        ;// functions with signature `fn({ , &, &mut} self, rhs: integer) -> ...`
        usize_assign
        isize_assign
        u8_assign
        i8_assign
        u16_assign
        i16_assign
        u32_assign
        i32_assign
        u64_assign
        i64_assign
        u128_assign
        i128_assign
        ;// functions with signature `fn({ , &, &mut} self, rhs: { , &, &mut} Self) -> Option<...>`
        copy_assign
        or_assign
        and_assign
        xor_assign
        const_eq
        const_ne
        ult
        ule
        ugt
        uge
        ilt
        ile
        igt
        ige
        add_assign
        sub_assign
        rsb_assign
    );
}
