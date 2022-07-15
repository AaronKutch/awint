//! Using combined normal and mimick types to assist in lowering

use std::num::NonZeroUsize;

use awint_macros::*;

use crate::mimick::{Bits, ExtAwi, InlAwi};

/// Given `inx.bw()` bits, this returns `2^inx.bw()` signals for every possible
/// state of `inx`. The `i`th signal is true only if `inx.to_usize() == 1`.
/// `cap` optionally restricts the number of signals
pub fn selector(inx: &Bits, cap: Option<usize>) -> ExtAwi {
    let mut signals =
        ExtAwi::zero(NonZeroUsize::new(cap.unwrap_or_else(|| 1usize << inx.bw())).unwrap());
    for i in 0..signals.bw() {
        let mut signal = inlawi!(1);
        let tmp = extawi!(signal, inx[0]).unwrap();
        for j in 1..inx.bw() {
            // depending on the `j`th bit of `i`, keep the signal line true
            if (i & (1 << j)) == 0 {
                signal.lut(&inlawi!(0100), &tmp).unwrap();
            } else {
                signal.lut(&inlawi!(1000), &tmp).unwrap();
            }
        }
        cc!(signal[0]; signals[i]).unwrap();
    }
    signals
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
pub fn dynamic_to_static_lut(out: &mut Bits, inx: &Bits, table: &Bits) {
    let signals = selector(inx, None);
    for j in 0..out.bw() {
        let mut column = inlawi!(0);
        for i in 0..signals.bw() {
            let cell = InlAwi::from(table.get((i * out.bw()) + j).unwrap());
            let triple = extawi!(column, cell, signals[i]).unwrap();
            // if the column is set or both the cell and signal are set
            column.lut(&inlawi!(1111_1000), &triple).unwrap();
        }
        out.set(j, column.to_bool()).unwrap();
    }
}

pub fn dynamic_to_static_get(bits: &Bits, inx: &Bits) -> inlawi_ty!(1) {
    let mut signals = selector(inx, Some(bits.bw()));
    // mask
    signals.and_assign(bits);
    let mut out = inlawi!(0);
    for i in 0..signals.bw() {
        let tmp = extawi!(out, signals[i]).unwrap();
        // horizontally OR
        out.lut(&inlawi!(1110), &tmp).unwrap();
    }
    out
}

pub fn dynamic_to_static_set(bits: &mut Bits, inx: &Bits, bit: &Bits) {
    let signals = selector(inx, Some(bits.bw()));
    let mut tmp = inlawi!(0);
    for i in 0..signals.bw() {
        let triple = extawi!(bits[i], bit, signals[i]).unwrap();
        // multiplex between using `bits` or the `bit` depending on the signal
        tmp.lut(&inlawi!(1101_1000), &triple).unwrap();
        bits.set(i, tmp.to_bool()).unwrap();
    }
}

pub fn resize(mut out: &mut Bits, x: &Bits, signed: bool) {
    if out.nzbw() <= x.nzbw() {
        for i in 0..out.bw() {
            cc!(x[i]; out[i]).unwrap();
        }
    } else {
        for i in 0..x.bw() {
            cc!(x[i]; out[i]).unwrap();
        }
        if signed {
            for i in x.bw()..out.bw() {
                cc!(x[x.bw() - 1]; out[i]).unwrap();
            }
        } else {
            for i in x.bw()..out.bw() {
                cc!(0; out[i]).unwrap();
            }
        }
    }
}
