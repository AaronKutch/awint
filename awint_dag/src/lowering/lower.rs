//! Lowers everything into LUT form

use awint_core::bw;
use awint_macros::{inlawi, inlawi_ty};
use triple_arena::{Ptr, PtrTrait};

use super::{dynamic_to_static_get, dynamic_to_static_lut, dynamic_to_static_set, resize};
use crate::{
    common::{EvalError, Lineage, Op::*},
    lowering::Dag,
    mimick::{Bits, ExtAwi, InlAwi},
};

impl<P: PtrTrait> Dag<P> {
    /// Lowers everything down to `Invalid`, `Opaque`, `Copy`, `Get` with static
    /// indexes, `Set` with static indexes, and `Lut`s with static tables. If
    /// unlowered nodes were produced they are added to `list`
    pub fn lower_node(&mut self, ptr: Ptr<P>, list: &mut Vec<Ptr<P>>) -> Result<(), EvalError> {
        if !self.dag.contains(ptr) {
            return Err(EvalError::InvalidPtr)
        }
        let start_op = self[ptr].op.clone();
        match start_op {
            Invalid => return Err(EvalError::Unevaluatable),
            Opaque(_) => (),
            Literal(_) => (),
            Lut([lut, inx], out_w) => {
                if !self[lut].op.is_literal() {
                    let mut out = ExtAwi::zero(out_w);
                    let lut = ExtAwi::opaque(self.get_bw(lut)?);
                    let inx = ExtAwi::opaque(self.get_bw(inx)?);
                    dynamic_to_static_lut(&mut out, &lut, &inx);
                    self.graft(ptr, list, &[out.state(), lut.state(), inx.state()])?;
                }
            }
            Get([bits, inx]) => {
                if !self[inx].op.is_literal() {
                    let bits = ExtAwi::opaque(self.get_bw(bits)?);
                    let inx = ExtAwi::opaque(self.get_bw(inx)?);
                    let out = dynamic_to_static_get(&bits, &inx);
                    self.graft(ptr, list, &[out.state(), bits.state(), inx.state()])?;
                }
            }
            Set([bits, inx, bit]) => {
                if !self[inx].op.is_literal() {
                    let mut bits = ExtAwi::opaque(self.get_bw(bits)?);
                    let inx = ExtAwi::opaque(self.get_bw(inx)?);
                    let bit = ExtAwi::opaque(self.get_bw(bit)?);
                    dynamic_to_static_set(&mut bits, &inx, &bit);
                    self.graft(ptr, list, &[bits.state(), inx.state(), bit.state()])?;
                }
            }
            ZeroResize([x], w) => {
                if self[x].nzbw == Some(w) {
                    self[x].op = Copy([x]);
                }
                let mut out = ExtAwi::opaque(w);
                let x = ExtAwi::opaque(self.get_bw(x)?);
                resize(&mut out, &x, false);
                self.graft(ptr, list, &[out.state(), x.state()])?;
            }
            SignResize([x], w) => {
                if self[x].nzbw == Some(w) {
                    self[x].op = Copy([x]);
                }
                let mut out = ExtAwi::opaque(w);
                let x = ExtAwi::opaque(self.get_bw(x)?);
                resize(&mut out, &x, true);
                self.graft(ptr, list, &[out.state(), x.state()])?;
            }
            Or([lhs, _]) => {
                if self[lhs].nzbw == Some(bw(1)) {
                    let mut out = <inlawi_ty!(1)>::zero();
                    let lhs = <inlawi_ty!(1)>::opaque();
                    let rhs = <inlawi_ty!(1)>::opaque();
                    let mut tmp = <inlawi_ty!(2)>::zero();
                    tmp.set(0, lhs.to_bool()).unwrap();
                    tmp.set(1, rhs.to_bool()).unwrap();
                    out.lut(&inlawi!(1110), &tmp).unwrap();
                    self.graft(ptr, list, &[out.state(), lhs.state(), rhs.state()])?;
                }
            }
            _ => return Err(EvalError::Unimplemented),
        }
        Ok(())
    }

    /// Note: `eval` should be before and after this call
    pub fn lower(&mut self) -> Result<(), EvalError> {
        let mut list = self.ptrs();
        let mut unimplemented = false;
        while let Some(p) = list.pop() {
            match self.lower_node(p, &mut list) {
                Ok(_) => (),
                Err(EvalError::Unimplemented) => unimplemented = true,
                Err(e) => return Err(e),
            }
        }
        if unimplemented {
            Err(EvalError::Unimplemented)
        } else {
            Ok(())
        }
    }
}
