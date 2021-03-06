//! Lowers everything into LUT form

use awint_macros::inlawi;
use triple_arena::{Ptr, PtrTrait};

use super::{
    bitwise, bitwise_not, cin_sum, dynamic_to_static_get, dynamic_to_static_lut,
    dynamic_to_static_set, incrementer, resize,
};
use crate::{
    common::{EvalError, Lineage, Op::*},
    lowering::{negator, Dag},
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
            Copy(_) => (),
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
                    let bits = ExtAwi::opaque(self.get_bw(bits)?);
                    let inx = ExtAwi::opaque(self.get_bw(inx)?);
                    let bit = ExtAwi::opaque(self.get_bw(bit)?);
                    let out = dynamic_to_static_set(&bits, &inx, &bit);
                    self.graft(ptr, list, &[
                        out.state(),
                        bits.state(),
                        inx.state(),
                        bit.state(),
                    ])?;
                }
            }
            FieldBit([lhs, to, rhs, from]) => {
                let rhs = ExtAwi::opaque(self.get_bw(rhs)?);
                let from = ExtAwi::opaque(self.get_bw(from)?);
                let bit = rhs.get(from.to_usize()).unwrap();
                let lhs = ExtAwi::opaque(self.get_bw(lhs)?);
                let to = ExtAwi::opaque(self.get_bw(to)?);
                let mut out = lhs.clone();
                out.set(to.to_usize(), bit);
                self.graft(ptr, list, &[
                    out.state(),
                    lhs.state(),
                    to.state(),
                    rhs.state(),
                    from.state(),
                ])?;
            }
            ZeroResize([x], w) => {
                let x = ExtAwi::opaque(self.get_bw(x)?);
                let out = resize(&x, w, false);
                self.graft(ptr, list, &[out.state(), x.state()])?;
            }
            SignResize([x], w) => {
                let x = ExtAwi::opaque(self.get_bw(x)?);
                let out = resize(&x, w, true);
                self.graft(ptr, list, &[out.state(), x.state()])?;
            }
            Lsb([x]) => {
                let x = ExtAwi::opaque(self.get_bw(x)?);
                let out = x.get(0).unwrap();
                self.graft(ptr, list, &[out.state(), x.state()])?;
            }
            Msb([x]) => {
                let x = ExtAwi::opaque(self.get_bw(x)?);
                let out = x.get(x.bw() - 1).unwrap();
                self.graft(ptr, list, &[out.state(), x.state()])?;
            }
            Not([x]) => {
                let x = ExtAwi::opaque(self.get_bw(x)?);
                let out = bitwise_not(&x);
                self.graft(ptr, list, &[out.state(), x.state()])?;
            }
            Or([lhs, rhs]) => {
                let lhs = ExtAwi::opaque(self.get_bw(lhs)?);
                let rhs = ExtAwi::opaque(self.get_bw(rhs)?);
                let out = bitwise(&lhs, &rhs, inlawi!(1110));
                self.graft(ptr, list, &[out.state(), lhs.state(), rhs.state()])?;
            }
            And([lhs, rhs]) => {
                let lhs = ExtAwi::opaque(self.get_bw(lhs)?);
                let rhs = ExtAwi::opaque(self.get_bw(rhs)?);
                let out = bitwise(&lhs, &rhs, inlawi!(1000));
                self.graft(ptr, list, &[out.state(), lhs.state(), rhs.state()])?;
            }
            Xor([lhs, rhs]) => {
                let lhs = ExtAwi::opaque(self.get_bw(lhs)?);
                let rhs = ExtAwi::opaque(self.get_bw(rhs)?);
                let out = bitwise(&lhs, &rhs, inlawi!(0110));
                self.graft(ptr, list, &[out.state(), lhs.state(), rhs.state()])?;
            }
            Inc([x, cin]) => {
                let x = ExtAwi::opaque(self.get_bw(x)?);
                let cin = ExtAwi::opaque(self.get_bw(cin)?);
                let out = incrementer(&x, &cin, false).0;
                self.graft(ptr, list, &[out.state(), x.state(), cin.state()])?;
            }
            IncCout([x, cin]) => {
                let x = ExtAwi::opaque(self.get_bw(x)?);
                let cin = ExtAwi::opaque(self.get_bw(cin)?);
                let out = incrementer(&x, &cin, false).1;
                self.graft(ptr, list, &[out.state(), x.state(), cin.state()])?;
            }
            Dec([x, cin]) => {
                let x = ExtAwi::opaque(self.get_bw(x)?);
                let cin = ExtAwi::opaque(self.get_bw(cin)?);
                let out = incrementer(&x, &cin, true).0;
                self.graft(ptr, list, &[out.state(), x.state(), cin.state()])?;
            }
            DecCout([x, cin]) => {
                let x = ExtAwi::opaque(self.get_bw(x)?);
                let cin = ExtAwi::opaque(self.get_bw(cin)?);
                let out = incrementer(&x, &cin, true).1;
                self.graft(ptr, list, &[out.state(), x.state(), cin.state()])?;
            }
            CinSum([cin, lhs, rhs]) => {
                let cin = ExtAwi::opaque(self.get_bw(cin)?);
                let lhs = ExtAwi::opaque(self.get_bw(lhs)?);
                let rhs = ExtAwi::opaque(self.get_bw(rhs)?);
                let out = cin_sum(&cin, &lhs, &rhs).0;
                self.graft(ptr, list, &[
                    out.state(),
                    cin.state(),
                    lhs.state(),
                    rhs.state(),
                ])?;
            }
            UnsignedOverflow([cin, lhs, rhs]) => {
                let cin = ExtAwi::opaque(self.get_bw(cin)?);
                let lhs = ExtAwi::opaque(self.get_bw(lhs)?);
                let rhs = ExtAwi::opaque(self.get_bw(rhs)?);
                let out = cin_sum(&cin, &lhs, &rhs).1;
                self.graft(ptr, list, &[
                    out.state(),
                    cin.state(),
                    lhs.state(),
                    rhs.state(),
                ])?;
            }
            SignedOverflow([cin, lhs, rhs]) => {
                let cin = ExtAwi::opaque(self.get_bw(cin)?);
                let lhs = ExtAwi::opaque(self.get_bw(lhs)?);
                let rhs = ExtAwi::opaque(self.get_bw(rhs)?);
                let out = cin_sum(&cin, &lhs, &rhs).2;
                self.graft(ptr, list, &[
                    out.state(),
                    cin.state(),
                    lhs.state(),
                    rhs.state(),
                ])?;
            }
            Neg([x, neg]) => {
                let x = ExtAwi::opaque(self.get_bw(x)?);
                let neg = ExtAwi::opaque(self.get_bw(neg)?);
                assert_eq!(neg.bw(), 1);
                let out = negator(&x, &neg);
                self.graft(ptr, list, &[out.state(), x.state(), neg.state()])?;
            }
            Abs([x]) => {
                let x = ExtAwi::opaque(self.get_bw(x)?);
                let mut out = x.clone();
                out.neg_assign(x.msb());
                self.graft(ptr, list, &[out.state(), x.state()])?;
            }
            op => return Err(EvalError::OtherString(format!("unimplemented: {:?}", op))),
        }
        Ok(())
    }

    /// Lowers all nodes in the DAG. Note: `eval` should be before and after
    /// this call for efficiency.
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
