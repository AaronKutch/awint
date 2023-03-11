use core::cmp;

use awint::{
    awint_internals::{Digit, BITS, USIZE_BITS},
    Bits, ExtAwi,
};
use rand_xoshiro::{
    rand_core::{RngCore, SeedableRng},
    Xoshiro128StarStar,
};

use crate::fuzz::{eq, fuzz_step, ne};

macro_rules! primitive_conversion {
    (
        $x0:ident,
        $x2:ident,
        $x3:ident,
        $x4:ident,
        $($fn_:ident, $fn_to:ident, $bits:expr);*;
    ) => {
        $(
            let tmp = $x0.$fn_to();
            $x2.$fn_(tmp);
            if $bits < $x3.bw() {
                // mask the bits to the size of the integer
                $x3.copy_($x0)?;
                $x3.range_and_(0..$bits).unwrap();
            } else {
                $x3.copy_($x0)?;
            }
            #[allow(unused_comparisons)]
            if tmp < 0 && $bits < $x0.bw() {
                // Sign extend
                $x4.umax_();
                $x4.shl_($bits).unwrap();
                $x3.or_($x4)?;
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
    d0: Digit,
) -> Option<()> {
    // `.unwrap()` is used on operations that fail in ways other than nonequal
    // bitwidth, because it is otherwise a pain to figure out what line the error
    // originates from.

    let w = x0.bw();

    // bytes check
    for b in x2
        .as_mut_bytes_full_width_nonportable()
        .iter_mut()
        .zip(x0.as_bytes_full_width_nonportable())
    {
        *b.0 = *b.1;
    }
    x2.clear_unused_bits();
    eq(x0, x2);

    // identity and inversion
    x2.copy_(x0)?;
    eq(x0, x2);
    x2.not_();
    ne(x0, x2);
    x2.not_();
    eq(x0, x2);

    // De Morgan's
    x2.copy_(x0)?;
    x3.copy_(x1)?;
    x4.copy_(x0)?;
    x5.copy_(x1)?;
    x2.and_(x3)?;
    x2.not_();
    x4.not_();
    x5.not_();
    x4.or_(x5)?;
    eq(x2, x4);

    // XOR negation
    x2.copy_(x0)?;
    x3.copy_(x1)?;
    x4.copy_(x0)?;
    x5.copy_(x1)?;
    x2.not_();
    x2.xor_(x3)?;
    x5.not_();
    x4.xor_(x5)?;
    eq(x2, x4);

    // Increments
    x2.copy_(x0)?;
    let oflow = x2.inc_(true);
    if oflow {
        assert!(x0.is_umax());
    } else {
        assert!(!x0.is_umax());
    }
    ne(x0, x2);
    x3.copy_(x0)?;
    let oflow = x3.dec_(false);
    if oflow {
        assert!(!x0.is_zero());
    } else {
        assert!(x0.is_zero());
    }
    ne(x0, x3);
    x3.inc_(true);
    eq(x0, x3);

    // Negation
    x2.copy_(x0)?;
    x2.neg_(true);
    x3.copy_(x0)?;
    x3.not_();
    x3.inc_(true);
    eq(x2, x3);

    // Absolute value
    x2.copy_(x0)?;
    x2.abs_();
    if x2.msb() {
        assert!(x0.is_imin());
        assert!(x2.is_imin());
    }
    if x2.msb() != x0.msb() {
        x3.copy_(x0)?;
        x3.neg_(true);
        eq(x3, x2);
    }

    // -(x0 + -x1) == (-x0 + x1)
    x2.copy_(x0)?;
    x3.copy_(x1)?;
    x4.copy_(x0)?;
    x5.copy_(x1)?;
    x2.sub_(x3)?;
    x2.neg_(true);
    x4.rsb_(x5)?;
    eq(x2, x4);

    // logical shift and rotation
    x2.copy_(x0)?;
    x3.copy_(x0)?;
    x4.copy_(x0)?;
    if s0 != 0 {
        x2.shl_(s0).unwrap();
        x3.lshr_(w - s0).unwrap();
        x2.or_(x3)?;
    }
    x4.rotl_(s0).unwrap();
    eq(x2, x4);
    x4.rotr_(s0).unwrap();
    eq(x0, x4);

    // masking
    x2.umax_();
    x2.shl_(s0).unwrap();
    if s1 != 0 {
        x3.umax_();
        x3.lshr_(w - s1).unwrap();
    } else {
        x3.zero_();
    }
    // mask off 0..s1
    x2.and_(x3)?;
    // mask off s0..bw, total mask should be s0..s1
    x2.and_(x0)?;
    x4.copy_(x0)?;
    x4.range_and_(s0..s1).unwrap();
    eq(x2, x4);

    // digit or assign
    x2.copy_(x0)?;
    x3.copy_(x0)?;
    x4.copy_(x1)?;
    x4.range_and_(cmp::min(w, s0)..cmp::min(w, s0 + BITS))
        .unwrap();
    x2.or_(x4)?;
    x4.lshr_(s0).unwrap();
    let digit = x4.to_digit();
    x3.digit_or_(digit, s0);
    eq(x2, x3);

    // arithmetic shift
    x2.copy_(x0)?;
    x3.copy_(x0)?;
    x2.ashr_(s0).unwrap();
    x3.lshr_(s0).unwrap();
    if x0.msb() {
        if s0 != 0 {
            x4.umax_();
            x4.shl_(w - s0).unwrap();
        } else {
            x4.zero_();
        }
        x3.or_(x4)?;
    }
    eq(x3, x2);

    // count ones
    x2.copy_(x0)?;
    x3.copy_(x0)?;
    x2.range_and_(0..s0).unwrap();
    x3.range_and_(s0..w).unwrap();
    assert_eq!(x0.count_ones(), x2.count_ones() + x3.count_ones());

    // leading and trailing zeros
    if x0.lz() + x0.tz() >= w {
        assert!(x0.is_zero());
        assert_eq!(x0.count_ones(), 0);
        assert_eq!(x0.lz(), w);
        assert_eq!(x0.tz(), w);
    } else {
        assert!(x0.count_ones() <= (w - x0.lz() - x0.tz()));
    }

    // bit get and set
    x2.copy_(x0)?;
    x3.copy_(x0)?;
    let b = x2.get(s0)?;
    x2.set(s0, !b)?;
    x3.xor_(x2)?;
    assert_eq!(x3.count_ones(), 1);
    assert_eq!(x3.tz(), s0);

    // mux_
    x2.copy_(x0)?;
    x2.mux_(x1, false)?;
    assert_eq!(x2, x0);
    x2.mux_(x1, true)?;
    assert_eq!(x2, x1);

    // reversal
    x2.copy_(x0)?;
    let lz = x2.lz();
    let tz = x2.tz();
    x2.rev_();
    assert_eq!(x2.tz(), lz);
    assert_eq!(x2.lz(), tz);
    x2.rev_();
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
    x2.zero_();
    assert!(x0.ilt(x2)? == x0.msb());

    // Summation and Comparison
    let cin = (s0 & 1) != 0;
    let (uof, iof) = x2.cin_sum_(cin, x0, x1)?;
    x3.copy_(x0)?;
    x3.add_(x1)?;
    x3.inc_(cin);
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
    x4.copy_(x0)?;
    x5.copy_(x1)?;
    if !(x4.is_imin() || x5.is_imin()) {
        x4.neg_(x0.msb());
        x5.neg_(x1.msb());
        let uof = x3.cin_sum_(cin, x4, x5)?.0;
        if iof {
            assert!(uof || x3.msb());
        } else {
            assert!(!uof);
            if (x0.msb() == x1.msb()) && x3.msb() {
                // There is a corner case where the sum would overflow through signed
                // minimum to signed maximum, except `cin` can push it back.
                assert!(!x3.is_imax());
            }
        }
    }

    // primitive conversion
    let tmp = x0.to_bool();
    x2.bool_(tmp);
    x3.uone_();
    x3.and_(x0)?;
    eq(x2, x3);
    primitive_conversion!(
        x0, x2, x3, x4,
        u8_, to_u8, 8;
        u16_, to_u16, 16;
        u32_, to_u32, 32;
        u64_, to_u64, 64;
        u128_, to_u128, 128;
        usize_, to_usize, USIZE_BITS;
        i8_, to_i8, 8;
        i16_, to_i16, 16;
        i32_, to_i32, 32;
        i64_, to_i64, 64;
        i128_, to_i128, 128;
        isize_, to_isize, USIZE_BITS;
        digit_, to_digit, BITS;
    );

    // multiplication and left shift
    x2.uone_();
    x2.shl_(s0)?;
    x4.zero_();
    x4.mul_add_(x0, x2)?;
    x3.copy_(x0)?;
    x3.shl_(s0)?;
    // (x3 << s) == (x3 * (1 << s))
    eq(x3, x4);

    // negation and multiplication
    x2.digit_(d0);
    x2.mul_add_(x0, x1)?;
    x3.copy_(x0)?;
    x4.copy_(x1)?;
    x3.neg_(true);
    x4.neg_(true);
    x5.digit_(d0);
    x5.mul_add_(x3, x4)?;
    eq(x2, x5);

    // digit multiplication and division
    // duo:x0 div:x1,x3 quo:x4 rem:x5
    let div = x1.to_digit();
    if div != 0 {
        x3.digit_(div);
        let rem = x4.digit_udivide_(x0, div)?;
        x5.digit_(rem);
        let oflow = x4.digit_cin_mul_(0, div);
        assert_eq!(oflow, 0);
        x4.add_(x5)?;
        // `rem < div` and `(quo * div) + rem == duo`
        assert!(x5.ult(x3)? && x4.const_eq(x0)?);

        // compare the two digit divisions
        x2.copy_(x0)?;
        x2.digit_udivide_inplace_(div)?;
        x3.digit_udivide_(x0, div)?;
        eq(x2, x3);
    }

    // compare digit multiplications
    x2.copy_(x0)?;
    let rhs = x0.to_digit();
    x2.digit_cin_mul_(0, rhs);
    x3.copy_(x0)?;
    x4.copy_(x0)?;
    x3.digit_mul_add_(x4, rhs)?;

    // alternate multiplication
    x2.copy_(x0)?;
    x2.mul_(x1, x3)?;
    x4.zero_();
    x4.mul_add_(x0, x1)?;
    eq(x2, x4);

    // unsigned division and logical right shift
    x2.uone_();
    x2.shl_(s0)?;
    Bits::udivide(x4, x5, x0, x2)?;
    x3.copy_(x0)?;
    x3.lshr_(s0)?;
    eq(x3, x4);
    x3.copy_(x0)?;
    x3.range_and_(0..s0)?;
    eq(x3, x5);
    // The remainder is handled below, and the signed versions are handled in
    // `identities`

    // full multiplication and division
    if !x1.is_zero() {
        Bits::udivide(x2, x3, x0, x1)?;
        assert!(x3.ult(x1)?);
        x4.copy_(x3)?;
        x4.mul_add_(x2, x1)?;
        eq(x4, x0);
    } else {
        assert!(Bits::udivide(x2, x3, x0, x1).is_none());
        x4.zero_();
        x4.mul_add_(x2, x1)?;
        assert!(x4.is_zero())
    }
    // the signed version is handled in `identities`

    // const string serialization
    if w <= 257 {
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
            if !string.is_empty() {
                assert!(string[0] != b'0');
            }
        }
        x2.bytes_radix_(sign, &string, radix, x3, x4).unwrap();
        eq(x0, x2);
    }

    Some(())
}

/// probes the sides of byte boundaries, and scales exponentially so that the
/// O(x^4) edge case tester doesn't explode too quickly. e.x. fuzz_lengths(2048)
/// gives `[0, 1, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128, 255, 256, 511, 512,
/// 1023, 1024, 1535, 1536, 2046, 2047]`.
#[allow(dead_code)]
fn fuzz_lengths(w: usize) -> Vec<usize> {
    if w < 4 {
        return (0..w).collect()
    }
    let mut v = vec![0, 1];
    let mut x = 8;
    while x < (w / 2) {
        v.push(x - 1);
        v.push(x);
        x *= 2;
    }
    while x < (w - 2) {
        v.push(x - 1);
        v.push(x);
        x = x + (x / 2);
    }
    // crucial for `imin` cases
    v.push(w - 2);
    v.push(w - 1);
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
            $x.umax_();
            $x.lshr_($fuzz_lengths[i0]).unwrap();
            for i1 in i0..$fuzz_lengths.len() {
                $x2.umax_();
                $x2.shl_($fuzz_lengths[i1 - i0]).unwrap();
                $x.and_($x2).unwrap();
                $inner
            }
        }
    };
}

/// Throws everything together for an identities party. Different operations are
/// interleaved with each other, so that the chance of false positives is
/// greatly reduced.
pub fn identities(iters: u32, seed: u64, tmp: [&mut Bits; 6]) -> Option<()> {
    let [x0, x1, x2, x3, x4, x5] = tmp;
    let w = x0.bw();
    let mut rng = Xoshiro128StarStar::seed_from_u64(seed + (w as u64));

    // edge case fuzzing
    #[cfg(not(debug_assertions))]
    {
        let fl = fuzz_lengths(w);
        edge_cases!(fl, x0, x2, {
            edge_cases!(fl, x1, x3, {
                let s0 = (rng.next_u32() as usize) % w;
                let s1 = (rng.next_u32() as usize) % w;
                let d0 = (u128::from(rng.next_u64()) | (u128::from(rng.next_u64()) << 64)) as Digit;
                identities_inner(&mut rng, x0, x1, x2, x3, x4, x5, s0, s1, d0)?;
            })
        })
    }

    // random fuzzing
    for _ in 0..iters {
        fuzz_step(&mut rng, x0, x2);
        fuzz_step(&mut rng, x1, x2);
        let s0 = (rng.next_u32() as usize) % w;
        let s1 = (rng.next_u32() as usize) % w;
        let d0 = (u128::from(rng.next_u64()) | (u128::from(rng.next_u64()) << 64)) as Digit;
        identities_inner(&mut rng, x0, x1, x2, x3, x4, x5, s0, s1, d0)?;

        // these are handled here because of the requirement that x0 and x1 are mutable

        // signed division and arithmetic right shift
        x2.uone_();
        x2.shl_(s0)?;
        Bits::idivide(x4, x5, x0, x2)?;
        x3.copy_(x0)?;
        x3.ashr_(s0)?;
        if x5.msb() {
            // arithmetic right shift is floored, so correct for that
            x4.dec_(false);
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
                    x4.copy_(x3)?;
                    x5.copy_(x1)?;
                    x4.abs_();
                    x5.abs_();
                    assert!(x4.ilt(x5)?);
                }
            }
            x4.copy_(x3)?;
            x4.mul_add_(x2, x1)?;
            eq(x4, x0);
        } else {
            assert!(Bits::idivide(x2, x3, x0, x1).is_none());
        }
    }
    Some(())
}
