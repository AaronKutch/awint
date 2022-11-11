//! Using combined normal and mimick types to assist in lowering

use std::{cmp::min, mem, num::NonZeroUsize};

use awint_ext::awi;
use awint_internals::BITS;
use awint_macros::*;

use crate::mimick::{Bits, ExtAwi, InlAwi};

// This code here is especially messy because we do not want to get into
// infinite lowering loops. These first few functions need to use manual `get`
// and `set` and only literal macros within loop blocks.

/// Given `inx.bw()` bits, this returns `2^inx.bw()` signals for every possible
/// state of `inx`. The `i`th signal is true only if `inx.to_usize() == i`.
/// `cap` optionally restricts the number of signals. If `cap` is 0, there is
/// one signal line set to true unconditionally.
pub fn selector(inx: &Bits, cap: Option<usize>) -> Vec<inlawi_ty!(1)> {
    let num = cap.unwrap_or_else(|| 1usize << inx.bw());
    if num == 0 {
        // not sure if this should be reachable
        panic!();
    }
    if num == 1 {
        return vec![inlawi!(1)]
    }
    let lb_num = num.next_power_of_two().trailing_zeros() as usize;
    let mut signals = vec![];
    let lut0 = inlawi!(0100);
    let lut1 = inlawi!(1000);
    for i in 0..num {
        let mut signal = inlawi!(1);
        for j in 0..lb_num {
            let mut tmp = inlawi!(00);
            tmp.set(0, inx.get(j).unwrap()).unwrap();
            tmp.set(1, signal.to_bool()).unwrap();
            // depending on the `j`th bit of `i`, keep the signal line true
            if (i & (1 << j)) == 0 {
                signal.lut_assign(&lut0, &tmp).unwrap();
            } else {
                signal.lut_assign(&lut1, &tmp).unwrap();
            }
        }
        signals.push(signal);
    }
    signals
}

pub fn selector_awi(inx: &Bits, cap: Option<usize>) -> ExtAwi {
    let num = cap.unwrap_or_else(|| 1usize << inx.bw());
    if num == 0 {
        // not sure if this should be reachable
        panic!();
    }
    if num == 1 {
        return extawi!(1)
    }
    let lb_num = num.next_power_of_two().trailing_zeros() as usize;
    let mut signals = ExtAwi::zero(NonZeroUsize::new(num).unwrap());
    let lut0 = inlawi!(0100);
    let lut1 = inlawi!(1000);
    for i in 0..num {
        let mut signal = inlawi!(1);
        for j in 0..lb_num {
            let mut tmp = inlawi!(00);
            tmp.set(0, inx.get(j).unwrap()).unwrap();
            tmp.set(1, signal.to_bool()).unwrap();
            // depending on the `j`th bit of `i`, keep the signal line true
            if (i & (1 << j)) == 0 {
                signal.lut_assign(&lut0, &tmp).unwrap();
            } else {
                signal.lut_assign(&lut1, &tmp).unwrap();
            }
        }
        signals.set(i, signal.to_bool()).unwrap();
    }
    signals
}

/// Trailing smear, given the value of `inx` it will set all bits in the vector
/// up to but not including the one indexed by `inx`. This means that
/// `inx.to_usize() == 0` sets no bits, and `inx.to_usize() == num_bits` sets
/// all the bits. Beware of off-by-one errors, if there are `n` bits then there
/// are `n + 1` possible unique smears.
pub fn tsmear_inx(inx: &Bits, num_signals: usize) -> Vec<inlawi_ty!(1)> {
    let next_pow = num_signals.next_power_of_two();
    let mut lb_num = next_pow.trailing_zeros() as usize;
    if next_pow == num_signals {
        // need extra bit to get all `n + 1`
        lb_num += 1;
    }
    let mut signals = vec![];
    let lut_s0 = inlawi!(10010000);
    let lut_and = inlawi!(1000);
    let lut_or = inlawi!(1110);
    for i in 0..num_signals {
        // if `inx < i`
        let mut signal = inlawi!(0);
        // if the prefix up until now is equal
        let mut prefix_equal = inlawi!(1);
        for j in (0..lb_num).rev() {
            // starting with the msb going down
            if (i & (1 << j)) == 0 {
                // update equality, and if the prefix is true and the `j` bit of `inx` is set
                // then the signal is set
                let mut tmp0 = inlawi!(00);
                tmp0.set(0, inx.get(j).unwrap()).unwrap();
                tmp0.set(1, prefix_equal.to_bool()).unwrap();
                let mut tmp1 = inlawi!(00);
                tmp1.lut_assign(&lut_s0, &tmp0).unwrap();
                prefix_equal.set(0, tmp1.get(0).unwrap()).unwrap();

                // or into `signal`
                let mut tmp = inlawi!(00);
                tmp.set(0, tmp1.get(1).unwrap()).unwrap();
                tmp.set(1, signal.to_bool()).unwrap();
                signal.lut_assign(&lut_or, &tmp).unwrap();
            } else {
                // just update equality, the `j`th bit of `i` is 1 and cannot be less than
                // whatever the `inx` bit is
                let mut tmp = inlawi!(00);
                tmp.set(0, inx.get(j).unwrap()).unwrap();
                tmp.set(1, prefix_equal.to_bool()).unwrap();
                prefix_equal.lut_assign(&lut_and, &tmp).unwrap();
            }
        }
        signals.push(signal);
    }
    signals
}

pub fn tsmear_awi(inx: &Bits, num_signals: usize) -> ExtAwi {
    let next_pow = num_signals.next_power_of_two();
    let mut lb_num = next_pow.trailing_zeros() as usize;
    if next_pow == num_signals {
        // need extra bit to get all `n + 1`
        lb_num += 1;
    }
    let mut signals = ExtAwi::zero(NonZeroUsize::new(num_signals).unwrap());
    let lut_s0 = inlawi!(10010000);
    let lut_and = inlawi!(1000);
    let lut_or = inlawi!(1110);
    for i in 0..num_signals {
        // if `inx < i`
        let mut signal = inlawi!(0);
        // if the prefix up until now is equal
        let mut prefix_equal = inlawi!(1);
        for j in (0..lb_num).rev() {
            // starting with the msb going down
            if (i & (1 << j)) == 0 {
                // update equality, and if the prefix is true and the `j` bit of `inx` is set
                // then the signal is set
                let mut tmp0 = inlawi!(00);
                tmp0.set(0, inx.get(j).unwrap()).unwrap();
                tmp0.set(1, prefix_equal.to_bool()).unwrap();
                let mut tmp1 = inlawi!(00);
                tmp1.lut_assign(&lut_s0, &tmp0).unwrap();
                prefix_equal.set(0, tmp1.get(0).unwrap()).unwrap();

                // or into `signal`
                let mut tmp = inlawi!(00);
                tmp.set(0, tmp1.get(1).unwrap()).unwrap();
                tmp.set(1, signal.to_bool()).unwrap();
                signal.lut_assign(&lut_or, &tmp).unwrap();
            } else {
                // just update equality, the `j`th bit of `i` is 1 and cannot be less than
                // whatever the `inx` bit is
                let mut tmp = inlawi!(00);
                tmp.set(0, inx.get(j).unwrap()).unwrap();
                tmp.set(1, prefix_equal.to_bool()).unwrap();
                prefix_equal.lut_assign(&lut_and, &tmp).unwrap();
            }
        }
        signals.set(i, signal.to_bool()).unwrap();
    }
    signals
}

pub fn mux_assign(x0: &Bits, x1: &Bits, inx: &Bits) -> ExtAwi {
    assert_eq!(x0.bw(), x1.bw());
    assert_eq!(inx.bw(), 1);
    let mut out = ExtAwi::zero(x0.nzbw());
    let lut = inlawi!(1100_1010);
    for i in 0..x0.bw() {
        let mut tmp0 = inlawi!(000);
        tmp0.set(0, x0.get(i).unwrap()).unwrap();
        tmp0.set(1, x1.get(i).unwrap()).unwrap();
        tmp0.set(2, inx.to_bool()).unwrap();
        let mut tmp1 = inlawi!(0);
        tmp1.lut_assign(&lut, &tmp0).unwrap();
        out.set(i, tmp1.to_bool()).unwrap();
    }
    out
}

/*
Normalize. Table size explodes really fast if trying
to keep as a single LUT, lets use a meta LUT.

e.x.
i_1 i_0
  0   0 x_0_0 x_1_0
  0   1 x_0_1 x_1_1
  1   0 x_0_2 x_1_2
  1   1 x_0_3 x_1_3
        y_0   y_1
=>
// a signal line for each row
s_0 = (!i_1) && (!i_0)
s_1 = (!i_1) && i_0
y_0 = (s_0 && x_0_0) || (s_1 && x_0_1) || ...
y_1 = (s_0 && x_1_0) || (s_1 && x_1_1) || ...
...
*/
pub fn dynamic_to_static_lut(out: &mut Bits, table: &Bits, inx: &Bits) {
    // if this is broken it breaks a lot of stuff
    assert!(table.bw() == (out.bw().checked_mul(1 << inx.bw()).unwrap()));
    let signals = selector(inx, None);
    let lut = inlawi!(1111_1000);
    for j in 0..out.bw() {
        let mut column = inlawi!(0);
        for (i, signal) in signals.iter().enumerate() {
            let mut tmp = inlawi!(000);
            tmp.set(0, signal.to_bool()).unwrap();
            tmp.set(1, table.get((i * out.bw()) + j).unwrap()).unwrap();
            tmp.set(2, column.to_bool()).unwrap();
            // if the column is set or both the cell and signal are set
            column.lut_assign(&lut, &tmp).unwrap();
        }
        out.set(j, column.to_bool()).unwrap();
    }
}

pub fn dynamic_to_static_get(bits: &Bits, inx: &Bits) -> inlawi_ty!(1) {
    if bits.bw() == 1 {
        return InlAwi::from(bits.to_bool())
    }
    let signals = selector(inx, Some(bits.bw()));
    let lut = inlawi!(1111_1000);
    let mut out = inlawi!(0);
    for (i, signal) in signals.iter().enumerate() {
        let mut tmp = inlawi!(000);
        tmp.set(0, signal.to_bool()).unwrap();
        tmp.set(1, bits.get(i).unwrap()).unwrap();
        tmp.set(2, out.to_bool()).unwrap();
        // horizontally OR the product of the signals and `bits`
        out.lut_assign(&lut, &tmp).unwrap();
    }
    out
}

pub fn dynamic_to_static_set(bits: &Bits, inx: &Bits, bit: &Bits) -> ExtAwi {
    if bits.bw() == 1 {
        return ExtAwi::from(bit)
    }
    let signals = selector(inx, Some(bits.bw()));
    let mut out = ExtAwi::zero(bits.nzbw());
    let lut = inlawi!(1101_1000);
    for (i, signal) in signals.iter().enumerate() {
        let mut tmp0 = inlawi!(000);
        tmp0.set(0, signal.to_bool()).unwrap();
        tmp0.set(1, bit.to_bool()).unwrap();
        tmp0.set(2, bits.get(i).unwrap()).unwrap();
        let mut tmp1 = inlawi!(0);
        // multiplex between using `bits` or the `bit` depending on the signal
        tmp1.lut_assign(&lut, &tmp0).unwrap();
        out.set(i, tmp1.to_bool()).unwrap();
    }
    out
}

pub fn resize(x: &Bits, w: NonZeroUsize, signed: bool) -> ExtAwi {
    let mut out = ExtAwi::zero(w);
    if out.nzbw() == x.nzbw() {
        out.copy_assign(x).unwrap();
    } else if out.nzbw() < x.nzbw() {
        for i in 0..out.bw() {
            out.set(i, x.get(i).unwrap()).unwrap();
        }
    } else {
        for i in 0..x.bw() {
            out.set(i, x.get(i).unwrap()).unwrap();
        }
        if signed {
            for i in x.bw()..out.bw() {
                out.set(i, x.get(x.bw() - 1).unwrap()).unwrap();
            }
        } // else the bits in `out` are automatically zero
    }
    out
}

pub fn resize_cond(x: &Bits, w: NonZeroUsize, signed: &Bits) -> ExtAwi {
    assert_eq!(signed.bw(), 1);
    let mut out = ExtAwi::zero(w);
    if out.nzbw() == x.nzbw() {
        out.copy_assign(x).unwrap();
    } else if out.nzbw() < x.nzbw() {
        for i in 0..out.bw() {
            out.set(i, x.get(i).unwrap()).unwrap();
        }
    } else {
        for i in 0..x.bw() {
            out.set(i, x.get(i).unwrap()).unwrap();
        }
        let signed = signed.to_bool();
        for i in x.bw()..out.bw() {
            out.set(i, signed).unwrap();
        }
    }
    out
}

pub fn static_field(lhs: &Bits, to: usize, rhs: &Bits, from: usize, width: usize) -> ExtAwi {
    //(lhs.bw(), to, rhs.bw(), from, width);
    assert!(
        width <= lhs.bw()
            && width <= rhs.bw()
            && to <= (lhs.bw() - width)
            && from <= (rhs.bw() - width)
    );
    let mut out = ExtAwi::from_bits(lhs);
    for i in 0..width {
        out.set(i + to, rhs.get(i + from).unwrap()).unwrap();
    }
    out
}

pub fn field_width(lhs: &Bits, rhs: &Bits, width: &Bits) -> ExtAwi {
    let mut out = ExtAwi::from_bits(lhs);
    let min_w = min(lhs.bw(), rhs.bw());
    let signals = tsmear_inx(width, min_w);
    let lut = inlawi!(1100_1010);
    for (i, signal) in signals.into_iter().enumerate() {
        // mux_assign betwee `lhs` or `rhs` based on the signal
        let mut tmp0 = inlawi!(000);
        tmp0.set(0, lhs.get(i).unwrap()).unwrap();
        tmp0.set(1, rhs.get(i).unwrap()).unwrap();
        tmp0.set(2, signal.to_bool()).unwrap();
        let mut tmp1 = inlawi!(0);
        tmp1.lut_assign(&lut, &tmp0).unwrap();
        out.set(i, tmp1.to_bool()).unwrap();
    }
    out
}

/// Given the diagonal control lines and input of a crossbar with output width
/// s.t. `input.bw() + out.bw() - 1 = signals.bw()`, returns the output. The
/// `i`th input bit and `j`th output bit are controlled by the `out.bw()
/// - 1 + i - j`th control line. `signal_range` uses a virtual `..` range of the
///   possible signals.
pub fn crossbar(
    output: &mut Bits,
    input: &Bits,
    signals: &[inlawi_ty!(1)],
    signal_range: (usize, usize),
) {
    assert!(signal_range.0 < signal_range.1);
    assert_eq!(signal_range.1 - signal_range.0, signals.len());
    for j in 0..output.bw() {
        // output bar for ORing
        let mut out_bar = inlawi!(0);
        for i in 0..input.bw() {
            let signal_inx = output.bw() - 1 + i - j;
            if (signal_inx >= signal_range.0) && (signal_inx < signal_range.1) {
                let mut inx = inlawi!(000);
                inx.set(0, input.get(i).unwrap()).unwrap();
                inx.set(1, signals[signal_inx - signal_range.0].to_bool())
                    .unwrap();
                inx.set(2, out_bar.to_bool()).unwrap();
                out_bar.lut_assign(&inlawi!(1111_1000), &inx).unwrap();
            }
        }
        output.set(j, out_bar.to_bool()).unwrap();
    }
}

pub fn funnel(x: &Bits, s: &Bits) -> ExtAwi {
    assert_eq!(x.bw() & 1, 0);
    assert_eq!(x.bw() / 2, 1 << s.bw());
    let mut out = ExtAwi::zero(NonZeroUsize::new(x.bw() / 2).unwrap());
    let signals = selector(s, None);
    // select zero should connect the zeroeth crossbars, so the offset is `out.bw()
    // - 1 + 0 - 0`
    let range = (out.bw() - 1, out.bw() - 1 + out.bw());
    crossbar(&mut out, x, &signals, range);
    out
}

pub fn field_from(lhs: &Bits, rhs: &Bits, from: &Bits, width: &Bits) -> ExtAwi {
    assert_eq!(from.bw(), BITS);
    assert_eq!(width.bw(), BITS);
    let mut out = ExtAwi::from_bits(lhs);
    // the `width == 0` case will result in a no-op from the later `field_width`
    // part, so we need to be able to handle just `rhs.bw()` possible shifts for
    // `width == 1` cases. There are `rhs.bw()` output bars needed. `from == 0`
    // should connect the zeroeth crossbars, so the offset is `rhs.bw() - 1 + 0 -
    // 0`. `j` stays zero and we have `0 <= i < rhs.bw()`
    let signals = selector(from, Some(rhs.bw()));
    let range = (rhs.bw() - 1, 2 * rhs.bw() - 1);
    let mut tmp = ExtAwi::zero(rhs.nzbw());
    crossbar(&mut tmp, rhs, &signals, range);
    out.field_width(&tmp, width.to_usize()).unwrap();
    out
}

pub fn shl(x: &Bits, s: &Bits) -> ExtAwi {
    assert_eq!(s.bw(), BITS);
    let mut signals = selector(s, Some(x.bw()));
    signals.reverse();
    let mut out = ExtAwi::zero(x.nzbw());
    crossbar(&mut out, x, &signals, (0, x.bw()));
    out
}

pub fn lshr(x: &Bits, s: &Bits) -> ExtAwi {
    assert_eq!(s.bw(), BITS);
    let signals = selector(s, Some(x.bw()));
    let mut out = ExtAwi::zero(x.nzbw());
    crossbar(&mut out, x, &signals, (x.bw() - 1, 2 * x.bw() - 1));
    out
}

pub fn ashr(x: &Bits, s: &Bits) -> ExtAwi {
    assert_eq!(s.bw(), BITS);
    let signals = selector(s, Some(x.bw()));
    let mut out = ExtAwi::zero(x.nzbw());
    crossbar(&mut out, x, &signals, (x.bw() - 1, 2 * x.bw() - 1));
    // Not sure if there is a better way to do this. If we try to use the crossbar
    // signals in some way, we are guaranteed some kind of > O(1) time thing.

    // get the `lb_num` that `tsmear_inx` uses, it can be `x.bw() - 1` because of
    // the `s < x.bw()` requirement, this single bit of difference is important
    // for powers of two because of the `lb_num += 1` condition it avoids.
    let num = x.bw() - 1;
    let next_pow = num.next_power_of_two();
    let mut lb_num = next_pow.trailing_zeros() as usize;
    if next_pow == num {
        // need extra bit to get all `n + 1`
        lb_num += 1;
    }
    if let Some(w) = NonZeroUsize::new(lb_num) {
        let mut gated_s = ExtAwi::zero(w);
        let lut_and = inlawi!(1000);
        // `gated_s` will be zero if `x.msb()` is zero, in which case `tsmear_inx`
        // produces all zeros to be ORed
        for i in 0..gated_s.bw() {
            let mut tmp0 = inlawi!(00);
            tmp0.set(0, s.get(i).unwrap()).unwrap();
            tmp0.set(1, x.msb()).unwrap();
            let mut tmp1 = inlawi!(0);
            tmp1.lut_assign(&lut_and, &tmp0).unwrap();
            gated_s.set(i, tmp1.to_bool()).unwrap();
        }
        let or_mask = tsmear_awi(&gated_s, num);
        let lut_or = inlawi!(1110);
        for i in 0..or_mask.bw() {
            let out_i = out.bw() - 1 - i;
            let mut tmp0 = inlawi!(00);
            tmp0.set(0, out.get(out_i).unwrap()).unwrap();
            tmp0.set(1, or_mask.get(i).unwrap()).unwrap();
            let mut tmp1 = inlawi!(0);
            tmp1.lut_assign(&lut_or, &tmp0).unwrap();
            out.set(out_i, tmp1.to_bool()).unwrap();
        }
    }

    out
}

pub fn rotl(x: &Bits, s: &Bits) -> ExtAwi {
    assert_eq!(s.bw(), BITS);
    let signals = selector(s, Some(x.bw()));
    // we will use the whole cross bar, with every signal controlling two diagonals
    // for the wraparound except for the `x.bw() - 1` one
    let mut rolled_signals = vec![inlawi!(0); 2 * x.bw() - 1];
    rolled_signals[x.bw() - 1].copy_assign(&signals[0]).unwrap();
    for i in 0..(x.bw() - 1) {
        rolled_signals[i].copy_assign(&signals[i + 1]).unwrap();
        rolled_signals[i + x.bw()]
            .copy_assign(&signals[i + 1])
            .unwrap();
    }
    rolled_signals.reverse();
    let mut out = ExtAwi::zero(x.nzbw());
    crossbar(&mut out, x, &rolled_signals, (0, 2 * x.bw() - 1));
    out
}

pub fn rotr(x: &Bits, s: &Bits) -> ExtAwi {
    assert_eq!(s.bw(), BITS);
    let signals = selector(s, Some(x.bw()));
    // we will use the whole cross bar, with every signal controlling two diagonals
    // for the wraparound except for the `x.bw() - 1` one
    let mut rolled_signals = vec![inlawi!(0); 2 * x.bw() - 1];
    rolled_signals[x.bw() - 1].copy_assign(&signals[0]).unwrap();
    for i in 0..(x.bw() - 1) {
        rolled_signals[i].copy_assign(&signals[i + 1]).unwrap();
        rolled_signals[i + x.bw()]
            .copy_assign(&signals[i + 1])
            .unwrap();
    }
    let mut out = ExtAwi::zero(x.nzbw());
    crossbar(&mut out, x, &rolled_signals, (0, 2 * x.bw() - 1));
    out
}

pub fn bitwise_not(x: &Bits) -> ExtAwi {
    let mut out = ExtAwi::zero(x.nzbw());
    for i in 0..x.bw() {
        let mut tmp = inlawi!(0);
        let inx = InlAwi::from(x.get(i).unwrap());
        tmp.lut_assign(&inlawi!(01), &inx).unwrap();
        out.set(i, tmp.to_bool()).unwrap();
    }
    out
}

pub fn bitwise(lhs: &Bits, rhs: &Bits, lut: inlawi_ty!(4)) -> ExtAwi {
    assert_eq!(lhs.bw(), rhs.bw());
    let mut out = ExtAwi::zero(lhs.nzbw());
    for i in 0..lhs.bw() {
        let mut tmp = inlawi!(0);
        let mut inx = inlawi!(00);
        inx.set(0, lhs.get(i).unwrap()).unwrap();
        inx.set(1, rhs.get(i).unwrap()).unwrap();
        tmp.lut_assign(&lut, &inx).unwrap();
        out.set(i, tmp.to_bool()).unwrap();
    }
    out
}

pub fn incrementer(x: &Bits, cin: &Bits, dec: bool) -> (ExtAwi, inlawi_ty!(1)) {
    assert_eq!(cin.bw(), 1);
    // half adder or subtractor
    let lut = if dec {
        inlawi!(1110_1001)
    } else {
        inlawi!(1001_0100)
    };
    let mut out = ExtAwi::zero(x.nzbw());
    let mut carry = InlAwi::from(cin.to_bool());
    for i in 0..x.bw() {
        let mut carry_sum = inlawi!(00);
        let mut inx = inlawi!(00);
        inx.set(0, carry.to_bool()).unwrap();
        inx.set(1, x.get(i).unwrap()).unwrap();
        carry_sum.lut_assign(&lut, &inx).unwrap();
        out.set(i, carry_sum.get(0).unwrap()).unwrap();
        carry.bool_assign(carry_sum.get(1).unwrap());
    }
    (out, carry)
}

// TODO select carry adder
/*
// for every pair of bits, calculate their sums and couts assuming 0 or 1 cins.
let mut s0_i = a ^ b; // a ^ b ^ 0
let mut s1_i = !s0_i; // a ^ b ^ 1
let mut c0_i = a & b; // carry of a + b + 0
let mut c1_i = a | b; // carry of a + b + 1
for i in 0..lb {
    let s0_tmp = carry_block_mux(c0_i, s0_i, s1_i, i).0;
    let s1_tmp = carry_block_mux(c1_i, s0_i, s1_i, i).1;
    let c0_tmp = carry_block_mux(c0_i, c0_i, c1_i, i).0;
    let c1_tmp = carry_block_mux(c1_i, c0_i, c1_i, i).1;
    s0_i = s0_tmp;
    s1_i = s1_tmp;
    c0_i = c0_tmp;
    c1_i = c1_tmp;
}
*/
pub fn cin_sum(cin: &Bits, lhs: &Bits, rhs: &Bits) -> (ExtAwi, inlawi_ty!(1), inlawi_ty!(1)) {
    assert_eq!(cin.bw(), 1);
    assert_eq!(lhs.bw(), rhs.bw());
    let bw = lhs.bw();
    // full adder
    let lut = inlawi!(1110_1001_1001_0100);
    let mut out = ExtAwi::zero(lhs.nzbw());
    let mut carry = InlAwi::from(cin.to_bool());
    for i in 0..bw {
        let mut carry_sum = inlawi!(00);
        let mut inx = inlawi!(000);
        inx.set(0, carry.to_bool()).unwrap();
        inx.set(1, lhs.get(i).unwrap()).unwrap();
        inx.set(2, rhs.get(i).unwrap()).unwrap();
        carry_sum.lut_assign(&lut, &inx).unwrap();
        out.set(i, carry_sum.get(0).unwrap()).unwrap();
        carry.bool_assign(carry_sum.get(1).unwrap());
    }
    let mut signed_overflow = inlawi!(0);
    let mut inx = inlawi!(000);
    inx.set(0, lhs.get(bw - 1).unwrap()).unwrap();
    inx.set(1, rhs.get(bw - 1).unwrap()).unwrap();
    inx.set(2, out.get(bw - 1).unwrap()).unwrap();
    signed_overflow
        .lut_assign(&inlawi!(0001_1000), &inx)
        .unwrap();
    (out, carry, signed_overflow)
}

pub fn negator(x: &Bits, neg: &Bits) -> ExtAwi {
    assert_eq!(neg.bw(), 1);
    // half adder with input inversion control
    let lut = inlawi!(0100_1001_1001_0100);
    let mut out = ExtAwi::zero(x.nzbw());
    let mut carry = InlAwi::from(neg.to_bool());
    for i in 0..x.bw() {
        let mut carry_sum = inlawi!(00);
        let mut inx = inlawi!(000);
        inx.set(0, carry.to_bool()).unwrap();
        inx.set(1, x.get(i).unwrap()).unwrap();
        inx.set(2, neg.to_bool()).unwrap();
        carry_sum.lut_assign(&lut, &inx).unwrap();
        out.set(i, carry_sum.get(0).unwrap()).unwrap();
        carry.bool_assign(carry_sum.get(1).unwrap());
    }
    out
}

pub fn field_to(lhs: &Bits, to: &Bits, rhs: &Bits, width: &Bits) -> ExtAwi {
    assert_eq!(to.bw(), BITS);
    assert_eq!(width.bw(), BITS);

    // simplified version of `field` below

    let num = lhs.bw();
    let next_pow = num.next_power_of_two();
    let mut lb_num = next_pow.trailing_zeros() as usize;
    if next_pow == num {
        // need extra bit to get all `n + 1`
        lb_num += 1;
    }
    if let Some(w) = NonZeroUsize::new(lb_num) {
        let mut signals = selector(to, Some(num));
        signals.reverse();

        let mut rhs_to_lhs = ExtAwi::zero(lhs.nzbw());
        crossbar(&mut rhs_to_lhs, rhs, &signals, (0, lhs.bw()));

        // to + width
        let mut tmp = ExtAwi::zero(w);
        tmp.usize_assign(to.to_usize());
        tmp.add_assign(&extawi!(width[..(w.get())]).unwrap())
            .unwrap();
        let tmask = tsmear_inx(&tmp, lhs.bw());
        // lhs.bw() - to
        let mut tmp = ExtAwi::zero(w);
        tmp.usize_assign(lhs.bw());
        tmp.sub_assign(&extawi!(to[..(w.get())]).unwrap()).unwrap();
        let mut lmask = tsmear_inx(&tmp, lhs.bw());
        lmask.reverse();

        let mut out = ExtAwi::from_bits(lhs);
        let lut = inlawi!(1011_1111_1000_0000);
        for i in 0..lhs.bw() {
            let mut tmp = inlawi!(0000);
            tmp.set(0, rhs_to_lhs.get(i).unwrap()).unwrap();
            tmp.set(1, tmask[i].to_bool()).unwrap();
            tmp.set(2, lmask[i].to_bool()).unwrap();
            tmp.set(3, lhs.get(i).unwrap()).unwrap();
            let mut lut_out = inlawi!(0);
            lut_out.lut_assign(&lut, &tmp).unwrap();
            out.set(i, lut_out.to_bool()).unwrap();
        }
        out
    } else {
        let lut = inlawi!(rhs[0], lhs[0]).unwrap();
        let mut out = extawi!(0);
        out.lut_assign(&lut, width).unwrap();
        out
    }
}

pub fn field(lhs: &Bits, to: &Bits, rhs: &Bits, from: &Bits, width: &Bits) -> ExtAwi {
    assert_eq!(to.bw(), BITS);
    assert_eq!(from.bw(), BITS);
    assert_eq!(width.bw(), BITS);

    // we use some summation to get the fielding done with a single crossbar

    // the basic shift offset is based on `to - from`, to keep the shift value
    // positive in case of `to == 0` and `from == rhs.bw()` we add `rhs.bw()` to
    // this value. The opposite extreme is therefore `to == lhs.bw()` and `from ==
    // 0`, which will be equal to `lhs.bw() + rhs.bw()` because of the added
    // `rhs.bw()`.
    let num = lhs.bw() + rhs.bw();
    let lb_num = num.next_power_of_two().trailing_zeros() as usize;
    if let Some(w) = NonZeroUsize::new(lb_num) {
        let mut shift = ExtAwi::zero(w);
        shift.usize_assign(rhs.bw());
        shift
            .add_assign(&extawi!(to[..(w.get())]).unwrap())
            .unwrap();
        shift
            .sub_assign(&extawi!(from[..(w.get())]).unwrap())
            .unwrap();

        let mut signals = selector(&shift, Some(num));
        signals.reverse();

        let mut rhs_to_lhs = ExtAwi::zero(lhs.nzbw());
        // really what `field` is is a well defined full crossbar, the masking part
        // after this is optimized to nothing if `rhs` is zero.
        crossbar(&mut rhs_to_lhs, rhs, &signals, (0, num));

        // `rhs` is now shifted correctly but we need a mask to overwrite the correct
        // bits of `lhs`. We use opposing `tsmears` and AND them together to get the
        // `width` window in the correct spot.

        // to + width
        let mut tmp = ExtAwi::zero(w);
        tmp.usize_assign(to.to_usize());
        tmp.add_assign(&extawi!(width[..(w.get())]).unwrap())
            .unwrap();
        let tmask = tsmear_inx(&tmp, lhs.bw());
        // lhs.bw() - to
        let mut tmp = ExtAwi::zero(w);
        tmp.usize_assign(lhs.bw());
        tmp.sub_assign(&extawi!(to[..(w.get())]).unwrap()).unwrap();
        let mut lmask = tsmear_inx(&tmp, lhs.bw());
        lmask.reverse();

        let mut out = ExtAwi::from_bits(lhs);
        // when `tmask` and `lmask` are both set, mux_assign in `rhs`
        let lut = inlawi!(1011_1111_1000_0000);
        for i in 0..lhs.bw() {
            let mut tmp = inlawi!(0000);
            tmp.set(0, rhs_to_lhs.get(i).unwrap()).unwrap();
            tmp.set(1, tmask[i].to_bool()).unwrap();
            tmp.set(2, lmask[i].to_bool()).unwrap();
            tmp.set(3, lhs.get(i).unwrap()).unwrap();
            let mut lut_out = inlawi!(0);
            lut_out.lut_assign(&lut, &tmp).unwrap();
            out.set(i, lut_out.to_bool()).unwrap();
        }
        out
    } else {
        // `lhs.bw() == 1`, `rhs.bw() == 1`, `width` is the only thing that matters
        let lut = inlawi!(rhs[0], lhs[0]).unwrap();
        let mut out = extawi!(0);
        out.lut_assign(&lut, width).unwrap();
        out
    }
}

pub fn equal(lhs: &Bits, rhs: &Bits) -> inlawi_ty!(1) {
    let mut ranks = vec![vec![]];
    let lut_xnor = inlawi!(1001);
    for i in 0..lhs.bw() {
        let mut tmp0 = inlawi!(00);
        tmp0.set(0, lhs.get(i).unwrap()).unwrap();
        tmp0.set(1, rhs.get(i).unwrap()).unwrap();
        let mut tmp1 = inlawi!(0);
        tmp1.lut_assign(&lut_xnor, &tmp0).unwrap();
        ranks[0].push(tmp1);
    }
    // binary tree reduce
    let lut_and = inlawi!(1000);
    loop {
        let prev_rank = ranks.last().unwrap();
        let rank_len = prev_rank.len();
        if rank_len == 1 {
            break prev_rank[0]
        }
        let mut next_rank = vec![];
        for i in 0..(rank_len / 2) {
            let mut tmp0 = inlawi!(00);
            tmp0.set(0, prev_rank[2 * i].to_bool()).unwrap();
            tmp0.set(1, prev_rank[2 * i + 1].to_bool()).unwrap();
            let mut tmp1 = inlawi!(0);
            tmp1.lut_assign(&lut_and, &tmp0).unwrap();
            next_rank.push(tmp1);
        }
        if (rank_len & 1) != 0 {
            next_rank.push(*prev_rank.last().unwrap())
        }
        ranks.push(next_rank);
    }
}

/// Uses the minimum number of bits to handle all cases, you may need to call
/// `to_usize` on the result
pub fn count_ones(x: &Bits) -> ExtAwi {
    // a tuple of an intermediate sum and the max possible value of that sum
    let mut ranks: Vec<Vec<(ExtAwi, awi::ExtAwi)>> = vec![vec![]];
    for i in 0..x.bw() {
        ranks[0].push((ExtAwi::from(x.get(i).unwrap()), awi::ExtAwi::from(true)));
    }
    loop {
        let prev_rank = ranks.last().unwrap();
        let rank_len = prev_rank.len();
        if rank_len == 1 {
            break prev_rank[0].0.clone()
        }
        let mut next_rank = vec![];
        let mut i = 0;
        loop {
            if i >= rank_len {
                break
            }
            // each rank adds another bit, keep adding until overflow
            let mut next_sum = extawi!(0, prev_rank[i].0);
            let mut next_max = {
                use awi::*;
                extawi!(0, prev_rank[i].1)
            };
            loop {
                i += 1;
                if i >= rank_len {
                    break
                }
                let w = next_max.bw();
                {
                    use awi::*;
                    let mut tmp = ExtAwi::zero(next_max.nzbw());
                    if tmp
                        .cin_sum_assign(
                            false,
                            &extawi!(zero: .., prev_rank[i].1; ..w).unwrap(),
                            &next_max,
                        )
                        .unwrap()
                        .0
                    {
                        // do not add another previous sum to this sum because of overflow
                        break
                    }
                    cc!(tmp; next_max).unwrap();
                }
                next_sum
                    .add_assign(&extawi!(zero: .., prev_rank[i].0; ..w).unwrap())
                    .unwrap();
            }
            next_rank.push((next_sum, next_max));
        }
        ranks.push(next_rank);
    }
}

// If there is a set bit, it and the bits less significant than it will be set
pub fn tsmear(x: &Bits) -> ExtAwi {
    let mut tmp0 = ExtAwi::from(x);
    let mut lvl = 0;
    // exponentially OR cascade the smear
    loop {
        let s = 1 << lvl;
        if s >= x.bw() {
            break tmp0
        }
        let mut tmp1 = tmp0.clone();
        tmp1.lshr_assign(s).unwrap();
        tmp0.or_assign(&tmp1).unwrap();
        lvl += 1;
    }
}

pub fn leading_zeros(x: &Bits) -> ExtAwi {
    let mut tmp = tsmear(x);
    tmp.not_assign();
    count_ones(&tmp)
}

pub fn trailing_zeros(x: &Bits) -> ExtAwi {
    let mut tmp = ExtAwi::from_bits(x);
    tmp.rev_assign();
    let mut tmp = tsmear(&tmp);
    tmp.not_assign();
    count_ones(&tmp)
}

pub fn significant_bits(x: &Bits) -> ExtAwi {
    count_ones(&tsmear(x))
}

pub fn lut_set(table: &Bits, entry: &Bits, inx: &Bits) -> ExtAwi {
    let num_entries = 1 << inx.bw();
    assert_eq!(table.bw(), entry.bw() * num_entries);
    let signals = selector(inx, Some(num_entries));
    let mut out = ExtAwi::from_bits(table);
    let lut_mux = inlawi!(1100_1010);
    for (j, signal) in signals.into_iter().enumerate() {
        for i in 0..entry.bw() {
            let lut_inx = i + (j * entry.bw());
            // mux_assign between `lhs` or `entry` based on the signal
            let mut tmp0 = inlawi!(000);
            tmp0.set(0, table.get(lut_inx).unwrap()).unwrap();
            tmp0.set(1, entry.get(i).unwrap()).unwrap();
            tmp0.set(2, signal.to_bool()).unwrap();
            let mut tmp1 = inlawi!(0);
            tmp1.lut_assign(&lut_mux, &tmp0).unwrap();
            out.set(lut_inx, tmp1.to_bool()).unwrap();
        }
    }
    out
}

pub fn mul_add(out_w: NonZeroUsize, add: Option<&Bits>, lhs: &Bits, rhs: &Bits) -> ExtAwi {
    // make `rhs` the smaller side, column size will be minimized
    let (lhs, rhs) = if lhs.bw() < rhs.bw() {
        (rhs, lhs)
    } else {
        (lhs, rhs)
    };

    let and = inlawi!(1000);
    let place_map0: &mut Vec<Vec<inlawi_ty!(1)>> = &mut vec![];
    let place_map1: &mut Vec<Vec<inlawi_ty!(1)>> = &mut vec![];
    for _ in 0..out_w.get() {
        place_map0.push(vec![]);
        place_map1.push(vec![]);
    }
    for j in 0..rhs.bw() {
        for i in 0..lhs.bw() {
            if let Some(place) = place_map0.get_mut(i + j) {
                let mut tmp = inlawi!(00);
                tmp.set(0, rhs.get(j).unwrap()).unwrap();
                tmp.set(1, lhs.get(i).unwrap()).unwrap();
                let mut ji = inlawi!(0);
                ji.lut_assign(&and, &tmp).unwrap();
                place.push(ji);
            }
        }
    }
    if let Some(add) = add {
        for i in 0..add.bw() {
            if let Some(place) = place_map0.get_mut(i) {
                place.push(inlawi!(add[i]).unwrap());
            }
        }
    }

    // after every bit that will be added is in its place, the columns of bits
    // sharing the same place are counted, resulting in a new set of columns, and
    // the process is repeated again. This reduces very quickly e.g. 65 -> 7 -> 3 ->
    // 2. The final set of 2 deep columns is added together with a fast adder.

    loop {
        let mut gt2 = false;
        for i in 0..place_map0.len() {
            if place_map0[i].len() > 2 {
                gt2 = true;
            }
        }
        if !gt2 {
            // if all columns 2 or less in height, break and use a fast adder
            break
        }
        for i in 0..place_map0.len() {
            if let Some(w) = NonZeroUsize::new(place_map0[i].len()) {
                let mut column = ExtAwi::zero(w);
                for (i, bit) in place_map0[i].drain(..).enumerate() {
                    column.set(i, bit.to_bool()).unwrap();
                }
                let row = count_ones(&column);
                for j in 0..row.bw() {
                    if let Some(place) = place_map1.get_mut(i + j) {
                        place.push(inlawi!(row[j]).unwrap())
                    }
                }
            }
        }
        mem::swap(place_map0, place_map1);
    }

    let mut out = ExtAwi::zero(out_w);
    let mut tmp = ExtAwi::zero(out_w);
    for i in 0..out.bw() {
        for (j, bit) in place_map0[i].iter().enumerate() {
            if j == 0 {
                out.set(i, bit.to_bool()).unwrap();
            } else if j == 1 {
                tmp.set(i, bit.to_bool()).unwrap();
            } else {
                unreachable!()
            }
        }
    }
    out.add_assign(&tmp).unwrap();
    out
}

/// DAG version of division, most implementations should probably use a fast
/// multiplier and a combination of the algorithms in the `specialized-div-rem`
/// crate, or Goldschmidt division. TODO if `div` is constant or there are
/// enough divisions sharing the same divisor, use fixed point inverses and
/// multiplication. TODO try out other algorithms in the `specialized-div-rem`
/// crate for this implementation.
pub fn division(duo: &Bits, div: &Bits) -> (ExtAwi, ExtAwi) {
    assert_eq!(duo.bw(), div.bw());

    // this uses the nonrestoring SWAR algorithm, with `duo` and `div` extended by
    // one bit so we don't need one of the edge case handlers. TODO can we
    // remove or optimize more of the prelude?

    let original_w = duo.nzbw();
    let w = NonZeroUsize::new(original_w.get() + 1).unwrap();
    let mut tmp = ExtAwi::zero(w);
    tmp.zero_resize_assign(duo);
    let duo = tmp;
    let mut tmp = ExtAwi::zero(w);
    tmp.zero_resize_assign(div);
    let div = tmp;

    let div_original = div.clone();

    /*
    if div == 0 {
        $zero_div_fn()
    }
    if duo < div {
        return (0, duo)
    }
    // SWAR opening
    let div_original = div;

    let mut shl = (div.leading_zeros() - duo.leading_zeros()) as usize;
    if duo < (div << shl) {
        // when the msb of `duo` and `div` are aligned, the resulting `div` may be
        // larger than `duo`, so we decrease the shift by 1.
        shl -= 1;
    }
    let mut div: $uX = (div << shl);
    duo = duo.wrapping_sub(div);
    let mut quo: $uX = 1 << shl;
    if duo < div_original {
        return (quo, duo);
    }
    // NOTE: only with extended `duo` and `div` can we do this
    let mask: $uX = (1 << shl) - 1;

    // central loop
    let div: $uX = div.wrapping_sub(1);
    let mut i = shl;
    loop {
        if i == 0 {
            break;
        }
        i -= 1;
        // note: the `wrapping_shl(1)` can be factored out, but would require another
        // restoring division step to prevent `(duo as $iX)` from overflowing
        if (duo as $iX) < 0 {
            // Negated binary long division step.
            duo = duo.wrapping_shl(1).wrapping_add(div);
        } else {
            // Normal long division step.
            duo = duo.wrapping_shl(1).wrapping_sub(div);
        }
    }
    if (duo as $iX) < 0 {
        // Restore. This was not needed in the original nonrestoring algorithm because of
        // the `duo < div_original` checks.
        duo = duo.wrapping_add(div);
    }
    // unpack
    return ((duo & mask) | quo, duo >> shl);
    */

    let duo_lt_div = duo.ult(&div).unwrap();

    // if there is a shortcut value it gets put in here and the `short`cut flag is
    // set to disable downstream shortcuts
    let mut short_quo = ExtAwi::zero(w);
    let mut short_rem = ExtAwi::zero(w);
    // leave `short_quo` as zero in both cases
    short_rem.mux_assign(&duo, duo_lt_div).unwrap();
    let mut short = duo_lt_div;

    let mut shl = leading_zeros(&div);
    shl.sub_assign(&leading_zeros(&duo)).unwrap();
    // if duo < (div << shl)
    let mut shifted_div = ExtAwi::from_bits(&div);
    shifted_div.shl_assign(shl.to_usize()).unwrap();
    let reshift = duo.ult(&shifted_div).unwrap();
    shl.dec_assign(!reshift);

    // if we need to reshift to correct for the shl decrement
    let mut reshifted = shifted_div.clone();
    reshifted.lshr_assign(1).unwrap();
    let mut div = shifted_div;
    div.mux_assign(&reshifted, reshift).unwrap();

    let mut duo = ExtAwi::from_bits(&duo);
    duo.sub_assign(&div).unwrap();
    // 1 << shl efficiently
    let tmp = selector_awi(&shl, Some(w.get()));
    let mut quo = ExtAwi::zero(w);
    quo.zero_resize_assign(&tmp);

    // if duo < div_original
    let b = duo.ult(&div_original).unwrap();
    short_quo.mux_assign(&quo, b & !short).unwrap();
    short_rem.mux_assign(&duo, b & !short).unwrap();
    short |= b;
    let mut mask = quo.clone();
    mask.dec_assign(false);

    // central loop
    div.dec_assign(false);

    let mut i = shl.clone();
    for _ in 0..w.get() {
        let b = i.is_zero();
        i.dec_assign(b);

        // Normal or Negated binary long division step.
        let mut tmp0 = div.clone();
        tmp0.neg_assign(!duo.msb());
        let mut tmp1 = duo.clone();
        tmp1.shl_assign(1).unwrap();
        tmp1.add_assign(&tmp0).unwrap();
        duo.mux_assign(&tmp1, !b).unwrap();
    }
    // final restore
    let mut tmp = ExtAwi::zero(w);
    tmp.mux_assign(&div, duo.msb()).unwrap();
    duo.add_assign(&tmp).unwrap();

    // unpack

    let mut tmp_quo = duo.clone();
    tmp_quo.and_assign(&mask).unwrap();
    tmp_quo.or_assign(&quo).unwrap();
    let mut tmp_rem = duo.clone();
    tmp_rem.lshr_assign(shl.to_usize()).unwrap();

    short_quo.mux_assign(&tmp_quo, !short).unwrap();
    short_rem.mux_assign(&tmp_rem, !short).unwrap();

    let mut tmp0 = ExtAwi::zero(original_w);
    let mut tmp1 = ExtAwi::zero(original_w);
    tmp0.zero_resize_assign(&short_quo);
    tmp1.zero_resize_assign(&short_rem);
    (tmp0, tmp1)
}
