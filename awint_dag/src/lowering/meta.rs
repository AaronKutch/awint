//! Using combined normal and mimick types to assist in lowering

use awint_macros::*;

use crate::mimick::{Bits, ExtAwi, InlAwi};

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
    let mut signals: Vec<inlawi_ty!(1)> = vec![];
    let rows = 1 << inx.bw();
    for i in 0..rows {
        let mut signal = InlAwi::zero();
        // the signal is only set if `i == inx`
        signal
            .lut(&extawi!(zero: .., 1, ..i; ..rows).unwrap(), inx)
            .unwrap();
        signals.push(signal);
    }
    for j in 0..out.bw() {
        let mut column = <inlawi_ty!(1)>::zero();
        for i in 0..rows {
            let mut cell = signals[i].clone();
            cell.and_assign(&InlAwi::from_bool(table.get((i * out.bw()) + j).unwrap()))
                .unwrap();
            column.or_assign(&cell).unwrap();
        }
        out.set(j, column.to_bool()).unwrap();
    }
}
