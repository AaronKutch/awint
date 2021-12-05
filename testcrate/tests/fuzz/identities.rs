use core::cmp;

use awint::{Bits, ExtAwi};
use rand_xoshiro::{
    rand_core::{RngCore, SeedableRng},
    Xoshiro128StarStar,
};

use crate::fuzz::{eq, fuzz_step, ne, BITS};

macro_rules! primitive_conversion {
    (
        $x0:ident,
        $x2:ident,
        $x3:ident,
        $x4:ident,
        $($fn_assign:ident, $fn_to:ident, $bits:expr);*;
    ) => {
        $(
            let tmp = $x0.$fn_to();
            $x2.$fn_assign(tmp);
            if $bits < $x3.bw() {
                // mask the bits to the size of the integer
                $x3.copy_assign($x0)?;
                $x3.range_and_assign(0..$bits).unwrap();
            } else {
                $x3.copy_assign($x0)?;
            }
            #[allow(unused_comparisons)]
            if tmp < 0 && $bits < $x0.bw() {
                // Sign extend
                $x4.umax_assign();
                $x4.shl_assign($bits).unwrap();
                $x3.or_assign($x4)?;
            }
            eq($x2, $x3);
        )*
    }
}

/// This inner function has `x0` and `x1` as `&Bits`, which eliminates the
/// chance of accidentally overwriting them and causing false positives.
fn identities_inner(
    rng: &mut Xoshiro128StarStar,
    x0: &Bits,
    x1: &Bits,
    x2: &mut Bits,
    x3: &mut Bits,
    x4: &mut Bits,
    x5: &mut Bits,
    s0: usize,
    s1: usize,
) -> Option<()> {
    // `.unwrap()` is used on operations that fail in ways other than nonequal
    // bitwidth, because it is otherwise a pain to figure out what line the error
    // originates from.

    let bw = x0.bw();

    // bytes check
    for b in x2.as_mut_bytes().iter_mut().zip(x0.as_bytes()) {
        *b.0 = *b.1;
    }
    eq(x0, x2);

    // identity and inversion
    x2.copy_assign(x0)?;
    eq(x0, x2);
    x2.not_assign();
    ne(x0, x2);
    x2.not_assign();
    eq(x0, x2);

    // De Morgan's
    x2.copy_assign(x0)?;
    x3.copy_assign(x1)?;
    x4.copy_assign(x0)?;
    x5.copy_assign(x1)?;
    x2.and_assign(x3);
    x2.not_assign();
    x4.not_assign();
    x5.not_assign();
    x4.or_assign(x5);
    eq(x2, x4);

    // XOR negation
    x2.copy_assign(x0)?;
    x3.copy_assign(x1)?;
    x4.copy_assign(x0)?;
    x5.copy_assign(x1)?;
    x2.not_assign();
    x2.xor_assign(x3)?;
    x5.not_assign();
    x4.xor_assign(x5)?;
    eq(x2, x4);

    // Increments
    x2.copy_assign(x0)?;
    let oflow = x2.inc_assign(true);
    if oflow {
        assert!(x0.is_umax());
    } else {
        assert!(!x0.is_umax());
    }
    ne(x0, x2);
    x3.copy_assign(x0)?;
    let oflow = x3.dec_assign(false);
    if oflow {
        assert!(!x0.is_zero());
    } else {
        assert!(x0.is_zero());
    }
    ne(x0, x3);
    x3.inc_assign(true);
    eq(x0, x3);

    // Negation
    x2.copy_assign(x0)?;
    x2.neg_assign(true);
    x3.copy_assign(x0)?;
    x3.not_assign();
    x3.inc_assign(true);
    eq(x2, x3);

    // Absolute value
    x2.copy_assign(x0)?;
    x2.abs_assign();
    if x2.msb() {
        assert!(x0.is_imin());
        assert!(x2.is_imin());
    }
    if x2.msb() != x0.msb() {
        x3.copy_assign(x0)?;
        x3.neg_assign(true);
        eq(x3, x2);
    }

    // -(x0 + -x1) == (-x0 + x1)
    x2.copy_assign(x0)?;
    x3.copy_assign(x1)?;
    x4.copy_assign(x0)?;
    x5.copy_assign(x1)?;
    x2.sub_assign(x3)?;
    x2.neg_assign(true);
    x4.rsb_assign(x5)?;
    eq(x2, x4);

    // logical shift and rotation
    x2.copy_assign(x0)?;
    x3.copy_assign(x0)?;
    x4.copy_assign(x0)?;
    if s0 != 0 {
        x2.shl_assign(s0).unwrap();
        x3.lshr_assign(bw - s0).unwrap();
        x2.or_assign(x3)?;
    }
    x4.rotl_assign(s0).unwrap();
    eq(x2, x4);
    x4.rotr_assign(s0).unwrap();
    eq(x0, x4);

    // masking
    x2.umax_assign();
    x2.shl_assign(s0).unwrap();
    if s1 != 0 {
        x3.umax_assign();
        x3.lshr_assign(bw - s1).unwrap();
    } else {
        x3.zero_assign();
    }
    // mask off 0..s1
    x2.and_assign(x3)?;
    // mask off s0..bw, total mask should be s0..s1
    x2.and_assign(x0)?;
    x4.copy_assign(x0)?;
    x4.range_and_assign(s0..s1).unwrap();
    eq(x2, x4);

    // usize or assign
    x2.copy_assign(x0)?;
    x3.copy_assign(x0)?;
    x4.copy_assign(x1)?;
    x4.range_and_assign(cmp::min(bw, s0)..cmp::min(bw, s0 + BITS))
        .unwrap();
    x2.or_assign(x4)?;
    x4.lshr_assign(s0).unwrap();
    let digit = x4.to_usize();
    x3.usize_or_assign(digit, s0);
    eq(x2, x3);

    // arithmetic shift
    x2.copy_assign(x0)?;
    x3.copy_assign(x0)?;
    x2.ashr_assign(s0).unwrap();
    x3.lshr_assign(s0).unwrap();
    if x0.msb() {
        if s0 != 0 {
            x4.umax_assign();
            x4.shl_assign(bw - s0).unwrap();
        } else {
            x4.zero_assign();
        }
        x3.or_assign(x4)?;
    }
    eq(x3, x2);

    // count ones
    x2.copy_assign(x0)?;
    x3.copy_assign(x0)?;
    x2.range_and_assign(0..s0).unwrap();
    x3.range_and_assign(s0..bw).unwrap();
    assert_eq!(x0.count_ones(), x2.count_ones() + x3.count_ones());

    // leading and trailing zeros
    if x0.lz() + x0.tz() >= bw {
        assert!(x0.is_zero());
        assert_eq!(x0.count_ones(), 0);
        assert_eq!(x0.lz(), bw);
        assert_eq!(x0.tz(), bw);
    } else {
        assert!(x0.count_ones() <= (bw - x0.lz() - x0.tz()));
    }

    // reversal
    x2.copy_assign(x0)?;
    let lz = x2.lz();
    let tz = x2.tz();
    x2.rev_assign();
    assert_eq!(x2.tz(), lz);
    assert_eq!(x2.lz(), tz);
    x2.rev_assign();
    eq(x0, x2);

    // comparison
    if x0.const_eq(x1)? {
        assert!(!x0.const_ne(x1)?);
        assert!(x0.ule(x1)?);
        assert!(x0.uge(x1)?);
        assert!(!x0.ult(x1)?);
        assert!(!x0.ugt(x1)?);
    }
    if x0.ult(x1)? {
        assert!(x0.const_ne(x1)?);
        assert!(!x0.const_eq(x1)?);
        assert!(x0.ule(x1)?);
        assert!(!x0.uge(x1)?);
        assert!(!x0.ugt(x1)?);
    }
    if x0.ugt(x1)? {
        assert!(x0.const_ne(x1)?);
        assert!(!x0.const_eq(x1)?);
        assert!(!x0.ule(x1)?);
        assert!(x0.uge(x1)?);
        assert!(x0.ugt(x1)?);
    }
    if x0.ilt(x1)? {
        assert!(x0.const_ne(x1)?);
        assert!(!x0.const_eq(x1)?);
        assert!(x0.ile(x1)?);
        assert!(!x0.ige(x1)?);
        assert!(!x0.igt(x1)?);
    }
    if x0.igt(x1)? {
        assert!(x0.const_ne(x1)?);
        assert!(!x0.const_eq(x1)?);
        assert!(!x0.ile(x1)?);
        assert!(x0.ige(x1)?);
        assert!(x0.igt(x1)?);
    }
    x2.zero_assign();
    assert!(x0.ilt(x2)? == x0.msb());

    // Summation and Comparison
    let cin = (s0 & 1) != 0;
    let (uof, iof) = x2.cin_sum_assign(cin, x0, x1)?;
    x3.copy_assign(x0)?;
    x3.add_assign(x1)?;
    x3.inc_assign(cin);
    eq(x2, x3);
    if uof {
        // using `ule` instead of `ult` because `cin` can make the sum wrap around all
        // the way
        assert!(x2.ule(x0)?);
        assert!(x2.ule(x1)?);
    } else {
        assert!(!x2.ult(x0)?);
        assert!(!x2.ult(x1)?);
    }
    x4.copy_assign(x0)?;
    x5.copy_assign(x1)?;
    if !(x4.is_imin() || x5.is_imin()) {
        x4.neg_assign(x0.msb());
        x5.neg_assign(x1.msb());
        let uof = x3.cin_sum_assign(cin, x4, x5)?.0;
        if iof {
            assert!(uof || x3.msb());
        } else {
            assert!(!uof);
            if x0.msb() == x1.msb() {
                if x3.msb() {
                    // There is a corner case where the sum would overflow through signed
                    // minimum to signed maximum, except `cin` can push it back.
                    assert!(!x3.is_imax());
                }
            }
        }
    }

    // primitive conversion
    let tmp = x0.to_bool();
    x2.bool_assign(tmp);
    x3.uone_assign();
    x3.and_assign(x0)?;
    eq(x2, x3);
    primitive_conversion!(
        x0, x2, x3, x4,
        u8_assign, to_u8, 8;
        u16_assign, to_u16, 16;
        u32_assign, to_u32, 32;
        u64_assign, to_u64, 64;
        u128_assign, to_u128, 128;
        usize_assign, to_usize, BITS;
        i8_assign, to_i8, 8;
        i16_assign, to_i16, 16;
        i32_assign, to_i32, 32;
        i64_assign, to_i64, 64;
        i128_assign, to_i128, 128;
        isize_assign, to_isize, BITS;
    );

    // multiplication and left shift
    x2.uone_assign();
    x2.shl_assign(s0);
    x4.zero_assign();
    x4.mul_add_assign(x0, x2)?;
    x3.copy_assign(x0)?;
    x3.shl_assign(s0);
    // (x3 << s) == (x3 * (1 << s))
    eq(x3, x4);

    // negation and multiplication
    x2.usize_assign(s0);
    x2.mul_add_assign(x0, x1)?;
    x3.copy_assign(x0)?;
    x4.copy_assign(x1)?;
    x3.neg_assign(true);
    x4.neg_assign(true);
    x5.usize_assign(s0);
    x5.mul_add_assign(x3, x4)?;
    eq(x2, x5);

    // short multiplication and division
    // duo:x0 div:x1,x3 quo:x4 rem:x5
    let div = x1.to_usize();
    if div != 0 {
        x3.usize_assign(div);
        let rem = x4.short_udivide_assign(x0, div)?;
        x5.usize_assign(rem);
        let oflow = x4.short_cin_mul(0, div);
        assert_eq!(oflow, 0);
        x4.add_assign(x5)?;
        // `rem < div` and `(quo * div) + rem == duo`
        assert!(x5.ult(x3)? && x4.const_eq(x0)?);

        // compare the two short divisions
        x2.copy_assign(x0)?;
        x2.short_udivide_inplace_assign(div)?;
        x3.short_udivide_assign(x0, div)?;
        eq(x2, x3);
    }

    // compare short multiplications
    x2.copy_assign(x0)?;
    let rhs = x0.to_usize();
    x2.short_cin_mul(0, rhs);
    x3.copy_assign(x0)?;
    x4.copy_assign(x0)?;
    x3.short_mul_add_assign(x4, rhs)?;

    // alternate multiplication
    x2.copy_assign(x0)?;
    x2.mul_assign(x1, x3)?;
    x4.zero_assign();
    x4.mul_add_assign(x0, x1)?;
    eq(x2, x4);

    // unsigned division and logical right shift
    x2.uone_assign();
    x2.shl_assign(s0)?;
    Bits::udivide(x4, x5, x0, x2)?;
    x3.copy_assign(x0)?;
    x3.lshr_assign(s0)?;
    eq(x3, x4);
    x3.copy_assign(x0)?;
    x3.range_and_assign(0..s0)?;
    eq(x3, x5);
    // The remainder is handled below, and the signed versions are handled in
    // `identities`

    // full multiplication and division
    if !x1.is_zero() {
        Bits::udivide(x2, x3, x0, x1)?;
        assert!(x3.ult(x1)?);
        x4.copy_assign(x3)?;
        x4.mul_add_assign(x2, x1)?;
        eq(x4, x0);
    } else {
        assert!(Bits::udivide(x2, x3, x0, x1).is_none());
        x4.zero_assign();
        x4.mul_add_assign(x2, x1)?;
        assert!(x4.is_zero())
    }
    // the signed version is handled in `identities`

    // const string serialization
    if bw <= 257 {
        let radix = ((rng.next_u32() % 35) + 2) as u8;
        let tmp_rng = rng.next_u32();
        let sign = if (tmp_rng & 0b1) != 0 {
            None
        } else {
            Some(x0.msb())
        };
        let min_chars = if (tmp_rng & 0b10) != 0 {
            rng.next_u32() as usize % 258
        } else {
            0
        };
        let string =
            ExtAwi::bits_to_vec_radix(x0, sign.is_some(), radix, (tmp_rng & 0b100) != 0, min_chars)
                .unwrap();

        assert!(min_chars <= string.len());
        if min_chars < string.len() {
            // make sure there are no leading zeros
            if string.len() != 0 {
                assert!(string[0] != b'0');
            }
        }
        x2.bytes_radix_assign(sign, &string, radix, x3, x4).unwrap();
        eq(x0, x2);
    }

    Some(())
}

/// probes the sides of byte boundaries, and scales exponentially so that the
/// O(x^4) edge case tester doesn't explode too quickly. e.x. fuzz_lengths(2048)
/// gives `[0, 1, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128, 255, 256, 511, 512,
/// 1023, 1024, 1535, 1536, 2046, 2047]`.
#[allow(dead_code)]
fn fuzz_lengths(bw: usize) -> Vec<usize> {
    if bw < 4 {
        return (0..bw).collect()
    }
    let mut v = vec![0, 1];
    let mut x = 8;
    while x < (bw / 2) {
        v.push(x - 1);
        v.push(x);
        x *= 2;
    }
    while x < (bw - 2) {
        v.push(x - 1);
        v.push(x);
        x = x + (x / 2);
    }
    // crucial for `imin` cases
    v.push(bw - 2);
    v.push(bw - 1);
    v
}

// the result of this applied to one 16 bit $x looks like
// 1111111111111111
// 1111111111111110
// 1111111111111100
// 1111111110000000
// 1111111100000000
// 1110000000000000
// 1100000000000000
// 1000000000000000
// 111111111111111
// 111111111111110
// 111111111111100
// 111111110000000
// 111111100000000
// 110000000000000
// 100000000000000
// 11111111111111
// 11111111111110
// 11111111111100
// 11111110000000
// 11111100000000
// 10000000000000
// 111111111
// 111111110
// 111111100
// 110000000
// 100000000
// 11111111
// 11111110
// 11111100
// 10000000
// 111
// 110
// 100
// 11
// 10
// 1
#[allow(unused_macros)]
macro_rules! edge_cases {
    ($fuzz_lengths:ident, $x:ident, $x2:ident, $inner:block) => {
        for i0 in 0..$fuzz_lengths.len() {
            $x.umax_assign();
            $x.lshr_assign($fuzz_lengths[i0]).unwrap();
            for i1 in i0..$fuzz_lengths.len() {
                $x2.umax_assign();
                $x2.shl_assign($fuzz_lengths[i1 - i0]);
                $x.and_assign($x2).unwrap();
                $inner
            }
        }
    };
}

/// Throws everything together for an identities party. Different operations are
/// interleaved with each other, so that the chance of false positives is
/// greatly reduced.
pub fn identities(iters: u32, seed: u64, tmp: [&mut Bits; 6]) -> Option<()> {
    let [mut x0, mut x1, mut x2, x3, x4, x5] = tmp;
    let bw = x0.bw();
    let mut rng = Xoshiro128StarStar::seed_from_u64(seed + (bw as u64));

    // edge case fuzzing
    #[cfg(not(debug_assertions))]
    {
        let fl = fuzz_lengths(bw);
        edge_cases!(fl, x0, x2, {
            edge_cases!(fl, x1, x3, {
                let s0 = (rng.next_u32() as usize) % bw;
                let s1 = (rng.next_u32() as usize) % bw;
                identities_inner(&mut rng, x0, x1, x2, x3, x4, x5, s0, s1)?;
            })
        })
    }

    // random fuzzing
    for _ in 0..iters {
        fuzz_step(&mut rng, &mut x0, &mut x2);
        fuzz_step(&mut rng, &mut x1, &mut x2);
        let s0 = (rng.next_u32() as usize) % bw;
        let s1 = (rng.next_u32() as usize) % bw;

        identities_inner(&mut rng, x0, x1, x2, x3, x4, x5, s0, s1)?;

        // these are handled here because of the requirement that x0 and x1 are mutable

        // signed division and arithmetic right shift
        x2.uone_assign();
        x2.shl_assign(s0)?;
        Bits::idivide(x4, x5, x0, x2)?;
        x3.copy_assign(x0)?;
        x3.ashr_assign(s0)?;
        if x5.msb() {
            // arithmetic right shift is floored, so correct for that
            x4.dec_assign(false);
        }
        if !x2.is_imin() {
            eq(x3, x4);
        }

        // full signed multiplication and division
        if !x1.is_zero() {
            Bits::idivide(x2, x3, x0, x1)?;
            // check that the signed remainder is correct
            if !x3.is_zero() {
                assert_eq!(x0.msb(), x3.msb());
                if x1.is_imin() {
                    assert!(!x3.is_imin());
                } else {
                    x4.copy_assign(x3)?;
                    x5.copy_assign(x1)?;
                    x4.abs_assign();
                    x5.abs_assign();
                    assert!(x4.ilt(x5)?);
                }
            }
            x4.copy_assign(x3)?;
            x4.mul_add_assign(x2, x1)?;
            eq(x4, x0);
        } else {
            assert!(Bits::idivide(x2, x3, x0, x1).is_none());
        }
    }
    Some(())
}
