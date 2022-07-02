//! Lowers everything into LUT form

use std::num::NonZeroUsize;

use triple_arena::{Ptr, PtrTrait};

use crate::{
    common::Op::*,
    lowering::{Dag, EvalError, Node},
};

impl<P: PtrTrait> Dag<P> {
    /// Returns if the lowering was all the way to a literal or a LUT with a
    /// literal table.
    pub fn lower_node(&mut self, ptr: Ptr<P>) -> Result<bool, EvalError> {
        let start_op = self[ptr].op.clone();
        match start_op {
            Literal(_) => return Ok(true),
            Invalid => return Err(EvalError::Unevaluatable),
            Opaque => return Err(EvalError::Unevaluatable),
            Lut(_) => {
                let [lut, inx] = self.get_2ops(ptr)?;
                if !self[lut].op.is_literal() {
                    // Normalize. Complexity explodes really fast if trying
                    // to keep as a single LUT, lets use a meta LUT.
                    //
                    // e.x.
                    // i_1 i_0
                    //   0   0 x_0_0 x_1_0
                    //   0   1 x_0_1 x_1_1
                    //   1   0 x_0_2 x_1_2
                    //   1   1 x_0_3 x_1_3
                    //         y_0   y_1
                    // =>
                    // y_0 = (s_0 && x_0_0) || (s_1 && x_0_1) || ...
                    // y_1 = (s_0 && x_1_0) || (s_1 && x_1_1) || ...
                    // // a signal line for each row
                    // s_0 = (!i_1) && (!i_0)
                    // s_1 = (!i_1) && i_0
                    // ...
                    let mut signals = vec![];
                    let inx_w = self.get_bw(inx)?;
                    let num_rows = 1usize << inx_w.get();
                    let nz_num_rows = NonZeroUsize::new(num_rows).unwrap();
                    for i in 0..num_rows {
                        signals.push(self.dag.insert(Node {
                            nzbw: Some(nz_num_rows),
                            op: todo!(), //Literal(ExtAwi),
                            ops: todo!(),
                            deps: todo!(),
                        }));
                    }
                }
            }
            _ => return Err(EvalError::Unimplemented),
        }
        Ok(false)
    }

    /// Note: `eval` should be before and after
    pub fn lower(&mut self) {
        let list = self.ptrs();
        for p in list {
            self.lower_node(p).unwrap();
        }
    }
}
