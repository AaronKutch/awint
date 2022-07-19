//! Using combined normal and mimick types to assist in lowering

use std::num::NonZeroUsize;

use awint_macros::*;

use crate::mimick::{Bits, ExtAwi, InlAwi};

// These first few functions need to use manual `get` and `set` and only literal
// macros within loop blocks to prevent infinite lowering loops

/// Given `inx.bw()` bits, this returns `2^inx.bw()` signals for every possible
/// state of `inx`. The `i`th signal is true only if `inx.to_usize() == 1`.
/// `cap` optionally restricts the number of signals
pub fn selector(inx: &Bits, cap: Option<usize>) -> Vec<inlawi_ty!(1)> {
    let num = cap.unwrap_or_else(|| 1usize << inx.bw());
    let mut signals = vec![];
    for i in 0..num {
        let mut signal = inlawi!(1);
        for j in 0..inx.bw() {
            let mut tmp = inlawi!(00);
            tmp.set(0, inx.get(j).unwrap());
            tmp.set(1, signal.to_bool());
            // depending on the `j`th bit of `i`, keep the signal line true
            if (i & (1 << j)) == 0 {
                signal.lut(&inlawi!(0100), &tmp).unwrap();
            } else {
                signal.lut(&inlawi!(1000), &tmp).unwrap();
            }
        }
        signals.push(signal);
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
pub fn dynamic_to_static_lut(out: &mut Bits, table: &Bits, inx: &Bits) {
    //dbg!(table.bw(), out.bw(), inx.bw());
    // if this is broken it breaks a lot of stuff
    assert!(table.bw() == (out.bw().checked_mul(1 << inx.bw()).unwrap()));
    let signals = selector(inx, None);
    for j in 0..out.bw() {
        let mut column = inlawi!(0);
        for (i, signal) in signals.iter().enumerate() {
            let mut tmp = inlawi!(000);
            tmp.set(0, signal.to_bool()).unwrap();
            tmp.set(1, table.get((i * out.bw()) + j).unwrap()).unwrap();
            tmp.set(2, column.to_bool()).unwrap();
            // if the column is set or both the cell and signal are set
            column.lut(&inlawi!(1111_1000), &tmp).unwrap();
        }
        out.set(j, column.to_bool()).unwrap();
    }
}

pub fn dynamic_to_static_get(bits: &Bits, inx: &Bits) -> inlawi_ty!(1) {
    let signals = selector(inx, Some(bits.bw()));
    let mut out = inlawi!(0);
    for (i, signal) in signals.iter().enumerate() {
        let mut tmp = inlawi!(000);
        tmp.set(0, signal.to_bool()).unwrap();
        tmp.set(1, bits.get(i).unwrap()).unwrap();
        tmp.set(2, out.to_bool()).unwrap();
        // horizontally OR the product of the signals and `bits`
        out.lut(&inlawi!(1111_1000), &tmp).unwrap();
    }
    out
}

pub fn dynamic_to_static_set(bits: &Bits, inx: &Bits, bit: &Bits) -> ExtAwi {
    let signals = selector(inx, Some(bits.bw()));
    let mut out = ExtAwi::zero(bits.nzbw());
    for (i, signal) in signals.iter().enumerate() {
        let mut tmp0 = inlawi!(000);
        tmp0.set(0, signal.to_bool()).unwrap();
        tmp0.set(1, bit.to_bool()).unwrap();
        tmp0.set(2, bit.get(i).unwrap()).unwrap();
        let mut tmp1 = inlawi!(0);
        // multiplex between using `bits` or the `bit` depending on the signal
        tmp1.lut(&inlawi!(1101_1000), &tmp0).unwrap();
        out.set(i, tmp1.to_bool()).unwrap();
    }
    out
}

pub fn resize(x: &Bits, w: NonZeroUsize, signed: bool) -> ExtAwi {
    let mut out = ExtAwi::zero(w);
    if out.nzbw() == x.nzbw() {
        out.copy_assign(x);
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
        } else {
            for i in x.bw()..out.bw() {
                out.set(i, false).unwrap();
            }
        }
    }
    out
}
