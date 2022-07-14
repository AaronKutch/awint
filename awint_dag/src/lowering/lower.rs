//! Lowers everything into LUT form

use triple_arena::{Ptr, PtrTrait};

use super::dynamic_to_static_lut;
use crate::{
    common::{EvalError, Lineage, Op::*},
    lowering::Dag,
    mimick::ExtAwi,
};

impl<P: PtrTrait> Dag<P> {
    /// Lowers everything except for `Invalid`, `Opaque`, `Lut` with dynamic
    /// tables, and `Copy` down to `Lut`s with static tables. If unlowered
    /// nodes were produced they are added to `list`
    pub fn lower_node(&mut self, ptr: Ptr<P>, list: &mut Vec<Ptr<P>>) -> Result<bool, EvalError> {
        let start_op = self[ptr].op.clone();
        match start_op {
            Invalid => return Err(EvalError::Unevaluatable),
            Opaque(_) => return Ok(false),
            Literal(_) => return Ok(true),
            Lut([lut, inx], out_w) => {
                if !self[lut].op.is_literal() {
                    let mut out = ExtAwi::zero(out_w);
                    let lut = ExtAwi::opaque(self.get_bw(lut)?);
                    let inx = ExtAwi::opaque(self.get_bw(inx)?);
                    dynamic_to_static_lut(&mut out, &lut, &inx);
                    self.graft(ptr, list, &[out.state(), lut.state(), inx.state()])?;
                }
            }
            _ => return Err(EvalError::Unimplemented),
        }
        Ok(false)
    }

    /// Note: `eval` should be before and after
    pub fn lower(&mut self) -> Result<(), EvalError> {
        let mut list = self.ptrs();
        while let Some(p) = list.pop() {
            self.lower_node(p, &mut list)?;
        }
        // TODO eliminate all `Copy`s in favor of implicit copies
        Ok(())
    }
}
