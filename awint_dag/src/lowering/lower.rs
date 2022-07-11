//! Lowers everything into LUT form

use triple_arena::{Ptr, PtrTrait};

use super::dynamic_to_static_lut;
use crate::{
    common::{Lineage, Op::*},
    lowering::{Dag, EvalError},
    mimick::ExtAwi,
};

impl<P: PtrTrait> Dag<P> {
    /// Returns if the lowering was all the way to a literal or a LUT with a
    /// literal table.
    pub fn lower_node(&mut self, ptr: Ptr<P>) -> Result<bool, EvalError> {
        let start_op = self[ptr].op.clone();
        match start_op {
            Literal(_) => return Ok(true),
            Invalid => return Err(EvalError::Unevaluatable),
            Opaque => return Ok(false),
            Lut(out_w) => {
                let [lut, inx] = self.get_2ops(ptr)?;
                if !self[lut].op.is_literal() {
                    let mut out = ExtAwi::zero(out_w);
                    let lut = ExtAwi::opaque(self.get_bw(lut)?);
                    let inx = ExtAwi::opaque(self.get_bw(inx)?);
                    dynamic_to_static_lut(&mut out, &lut, &inx);
                    self.graft(ptr, out.state(), vec![lut.state(), inx.state()])?;
                }
            }
            _ => return Err(EvalError::Unimplemented),
        }
        Ok(false)
    }

    /// Note: `eval` should be before and after
    pub fn lower(&mut self) -> Result<(), EvalError> {
        let list = self.ptrs();
        for p in list {
            self.lower_node(p)?;
        }
        Ok(())
    }
}
