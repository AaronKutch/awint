//! Lowers everything into LUT form

use std::{cmp::min, num::NonZeroUsize};

use awint_macros::inlawi;

use crate::{
    lowering::{
        meta::{
            ashr, bitwise, bitwise_not, cin_sum, dynamic_to_static_get, dynamic_to_static_lut,
            dynamic_to_static_set, field, field_from, field_to, field_width, funnel, incrementer,
            lshr, negator, resize, rotl, rotr, shl, static_field,
        },
        Dag, PNode,
    },
    mimick::{Bits, ExtAwi, InlAwi},
    EvalError, Lineage,
    Op::*,
};

impl Dag {
    /// Lowers everything down to `Invalid`, `Opaque`, `Copy`, `StaticGet`,
    /// `StaticSet`, and `StaticLut`. New nodes that may be unlowered are
    /// colored with `visit`. Returns `true` if the node is already lowered.
    pub fn lower_node(&mut self, ptr: PNode, visit: u64) -> Result<bool, EvalError> {
        if !self.dag.contains(ptr) {
            return Err(EvalError::InvalidPtr)
        }
        let start_op = self[ptr].op.clone();
        let out_w = self.get_bw(ptr);
        let v = visit;
        match start_op {
            Invalid => return Err(EvalError::OtherStr("encountered `Invalid` in lowering")),
            Opaque(_) | Literal(_) | Copy(_) | StaticLut(..) | StaticGet(..) | StaticSet(..) => {
                return Ok(true)
            }
            Lut([lut, inx]) => {
                if self[lut].op.is_literal() {
                    self[ptr].op = StaticLut([inx], awint_ext::ExtAwi::from(self.lit(lut)));
                    self.dec_rc(lut)?;
                } else {
                    let mut out = ExtAwi::zero(out_w);
                    let lut = ExtAwi::opaque(self.get_bw(lut));
                    let inx = ExtAwi::opaque(self.get_bw(inx));
                    dynamic_to_static_lut(&mut out, &lut, &inx);
                    self.graft(ptr, v, &[out.state(), lut.state(), inx.state()])?;
                }
            }
            Get([bits, inx]) => {
                if self[inx].op.is_literal() {
                    self[ptr].op = StaticGet([bits], self.usize(inx).unwrap());
                    self.dec_rc(inx)?;
                } else {
                    let bits = ExtAwi::opaque(self.get_bw(bits));
                    let inx = ExtAwi::opaque(self.get_bw(inx));
                    let out = dynamic_to_static_get(&bits, &inx);
                    self.graft(ptr, v, &[out.state(), bits.state(), inx.state()])?;
                }
            }
            Set([bits, inx, bit]) => {
                if self[inx].op.is_literal() {
                    self[ptr].op = StaticSet([bits, bit], self.usize(inx).unwrap());
                    self.dec_rc(inx)?;
                } else {
                    let bits = ExtAwi::opaque(self.get_bw(bits));
                    let inx = ExtAwi::opaque(self.get_bw(inx));
                    let bit = ExtAwi::opaque(self.get_bw(bit));
                    let out = dynamic_to_static_set(&bits, &inx, &bit);
                    self.graft(ptr, v, &[
                        out.state(),
                        bits.state(),
                        inx.state(),
                        bit.state(),
                    ])?;
                }
            }
            FieldBit([lhs, to, rhs, from]) => {
                let rhs = ExtAwi::opaque(self.get_bw(rhs));
                let from = ExtAwi::opaque(self.get_bw(from));
                let bit = rhs.get(from.to_usize()).unwrap();
                let lhs = ExtAwi::opaque(self.get_bw(lhs));
                let to = ExtAwi::opaque(self.get_bw(to));
                // keep `lhs` the same, `out` has the set bit
                let mut out = lhs.clone();
                out.set(to.to_usize(), bit);
                self.graft(ptr, v, &[
                    out.state(),
                    lhs.state(),
                    to.state(),
                    rhs.state(),
                    from.state(),
                ])?;
            }
            ZeroResize([x]) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let out = resize(&x, out_w, false);
                self.graft(ptr, v, &[out.state(), x.state()])?;
            }
            SignResize([x]) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let out = resize(&x, out_w, true);
                self.graft(ptr, v, &[out.state(), x.state()])?;
            }
            Lsb([x]) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let out = x.get(0).unwrap();
                self.graft(ptr, v, &[out.state(), x.state()])?;
            }
            Msb([x]) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let out = x.get(x.bw() - 1).unwrap();
                self.graft(ptr, v, &[out.state(), x.state()])?;
            }
            FieldWidth([lhs, rhs, width]) => {
                if self[width].op.is_literal() {
                    let lhs = ExtAwi::opaque(self.get_bw(lhs));
                    let rhs = ExtAwi::opaque(self.get_bw(rhs));
                    let out = static_field(&lhs, 0, &rhs, 0, self.usize(width).unwrap());
                    self.graft(ptr, v, &[
                        out.state(),
                        lhs.state(),
                        rhs.state(),
                        ExtAwi::opaque(self.get_bw(width)).state(),
                    ])?;
                } else {
                    let lhs = ExtAwi::opaque(self.get_bw(lhs));
                    let rhs = ExtAwi::opaque(self.get_bw(rhs));
                    let width = ExtAwi::opaque(self.get_bw(width));
                    let out = field_width(&lhs, &rhs, &width);
                    self.graft(ptr, v, &[
                        out.state(),
                        lhs.state(),
                        rhs.state(),
                        width.state(),
                    ])?;
                }
            }
            Funnel([x, s]) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let s = ExtAwi::opaque(self.get_bw(s));
                let out = funnel(&x, &s);
                self.graft(ptr, v, &[out.state(), x.state(), s.state()])?;
            }
            FieldFrom([lhs, rhs, from, width]) => {
                if self[from].op.is_literal() {
                    let lhs = ExtAwi::opaque(self.get_bw(lhs));
                    let rhs = ExtAwi::opaque(self.get_bw(rhs));
                    let width = ExtAwi::opaque(self.get_bw(width));
                    let from_u = self.usize(from)?;
                    let out = if let Some(w) = NonZeroUsize::new(rhs.bw() - from_u) {
                        let tmp0 = ExtAwi::zero(w);
                        let tmp1 = static_field(&tmp0, 0, &rhs, from_u, rhs.bw() - from_u);
                        let mut out = lhs.clone();
                        out.field_width(&tmp1, width.to_usize());
                        out
                    } else {
                        lhs.clone()
                    };
                    self.graft(ptr, v, &[
                        out.state(),
                        lhs.state(),
                        rhs.state(),
                        ExtAwi::opaque(self.get_bw(from)).state(),
                        width.state(),
                    ])?;
                } else {
                    let lhs = ExtAwi::opaque(self.get_bw(lhs));
                    let rhs = ExtAwi::opaque(self.get_bw(rhs));
                    let from = ExtAwi::opaque(self.get_bw(from));
                    let width = ExtAwi::opaque(self.get_bw(width));
                    // the optimizations on `width` are done later on an inner `field_width` call
                    let out = field_from(&lhs, &rhs, &from, &width);
                    self.graft(ptr, v, &[
                        out.state(),
                        lhs.state(),
                        rhs.state(),
                        from.state(),
                        width.state(),
                    ])?;
                }
            }
            Shl([x, s]) => {
                if self[s].op.is_literal() {
                    let x = ExtAwi::opaque(self.get_bw(x));
                    let s_u = self.usize(s)?;
                    let tmp = ExtAwi::zero(x.nzbw());
                    let out = static_field(&tmp, s_u, &x, 0, x.bw() - s_u);
                    self.graft(ptr, v, &[
                        out.state(),
                        x.state(),
                        ExtAwi::opaque(self.get_bw(s)).state(),
                    ])?;
                } else {
                    let x = ExtAwi::opaque(self.get_bw(x));
                    let s = ExtAwi::opaque(self.get_bw(s));
                    let out = shl(&x, &s);
                    self.graft(ptr, v, &[out.state(), x.state(), s.state()])?;
                }
            }
            Lshr([x, s]) => {
                if self[s].op.is_literal() {
                    let x = ExtAwi::opaque(self.get_bw(x));
                    let s_u = self.usize(s)?;
                    let tmp = ExtAwi::zero(x.nzbw());
                    let out = static_field(&tmp, 0, &x, s_u, x.bw() - s_u);
                    self.graft(ptr, v, &[
                        out.state(),
                        x.state(),
                        ExtAwi::opaque(self.get_bw(s)).state(),
                    ])?;
                } else {
                    let x = ExtAwi::opaque(self.get_bw(x));
                    let s = ExtAwi::opaque(self.get_bw(s));
                    let out = lshr(&x, &s);
                    self.graft(ptr, v, &[out.state(), x.state(), s.state()])?;
                }
            }
            Ashr([x, s]) => {
                if self[s].op.is_literal() {
                    let x = ExtAwi::opaque(self.get_bw(x));
                    let s_u = self.usize(s)?;
                    let mut tmp = ExtAwi::zero(x.nzbw());
                    for i in 0..x.bw() {
                        tmp.set(i, x.msb());
                    }
                    let out = static_field(&tmp, 0, &x, s_u, x.bw() - s_u);
                    self.graft(ptr, v, &[
                        out.state(),
                        x.state(),
                        ExtAwi::opaque(self.get_bw(s)).state(),
                    ])?;
                } else {
                    let x = ExtAwi::opaque(self.get_bw(x));
                    let s = ExtAwi::opaque(self.get_bw(s));
                    let out = ashr(&x, &s);
                    self.graft(ptr, v, &[out.state(), x.state(), s.state()])?;
                }
            }
            Rotl([x, s]) => {
                if self[s].op.is_literal() {
                    let x = ExtAwi::opaque(self.get_bw(x));
                    let s_u = self.usize(s)?;
                    let out = if s_u == 0 {
                        x.clone()
                    } else {
                        let tmp = static_field(&ExtAwi::zero(x.nzbw()), s_u, &x, 0, x.bw() - s_u);
                        static_field(&tmp, 0, &x, x.bw() - s_u, s_u)
                    };
                    self.graft(ptr, v, &[
                        out.state(),
                        x.state(),
                        ExtAwi::opaque(self.get_bw(s)).state(),
                    ])?;
                } else {
                    let x = ExtAwi::opaque(self.get_bw(x));
                    let s = ExtAwi::opaque(self.get_bw(s));
                    let out = rotl(&x, &s);
                    self.graft(ptr, v, &[out.state(), x.state(), s.state()])?;
                }
            }
            Rotr([x, s]) => {
                if self[s].op.is_literal() {
                    let x = ExtAwi::opaque(self.get_bw(x));
                    let s_u = self.usize(s)?;
                    let out = if s_u == 0 {
                        x.clone()
                    } else {
                        let tmp = static_field(&ExtAwi::zero(x.nzbw()), 0, &x, s_u, x.bw() - s_u);
                        static_field(&tmp, x.bw() - s_u, &x, 0, s_u)
                    };
                    self.graft(ptr, v, &[
                        out.state(),
                        x.state(),
                        ExtAwi::opaque(self.get_bw(s)).state(),
                    ])?;
                } else {
                    let x = ExtAwi::opaque(self.get_bw(x));
                    let s = ExtAwi::opaque(self.get_bw(s));
                    let out = rotr(&x, &s);
                    self.graft(ptr, v, &[out.state(), x.state(), s.state()])?;
                }
            }
            Not([x]) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let out = bitwise_not(&x);
                self.graft(ptr, v, &[out.state(), x.state()])?;
            }
            Or([lhs, rhs]) => {
                let lhs = ExtAwi::opaque(self.get_bw(lhs));
                let rhs = ExtAwi::opaque(self.get_bw(rhs));
                let out = bitwise(&lhs, &rhs, inlawi!(1110));
                self.graft(ptr, v, &[out.state(), lhs.state(), rhs.state()])?;
            }
            And([lhs, rhs]) => {
                let lhs = ExtAwi::opaque(self.get_bw(lhs));
                let rhs = ExtAwi::opaque(self.get_bw(rhs));
                let out = bitwise(&lhs, &rhs, inlawi!(1000));
                self.graft(ptr, v, &[out.state(), lhs.state(), rhs.state()])?;
            }
            Xor([lhs, rhs]) => {
                let lhs = ExtAwi::opaque(self.get_bw(lhs));
                let rhs = ExtAwi::opaque(self.get_bw(rhs));
                let out = bitwise(&lhs, &rhs, inlawi!(0110));
                self.graft(ptr, v, &[out.state(), lhs.state(), rhs.state()])?;
            }
            Inc([x, cin]) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let cin = ExtAwi::opaque(self.get_bw(cin));
                let out = incrementer(&x, &cin, false).0;
                self.graft(ptr, v, &[out.state(), x.state(), cin.state()])?;
            }
            IncCout([x, cin]) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let cin = ExtAwi::opaque(self.get_bw(cin));
                let out = incrementer(&x, &cin, false).1;
                self.graft(ptr, v, &[out.state(), x.state(), cin.state()])?;
            }
            Dec([x, cin]) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let cin = ExtAwi::opaque(self.get_bw(cin));
                let out = incrementer(&x, &cin, true).0;
                self.graft(ptr, v, &[out.state(), x.state(), cin.state()])?;
            }
            DecCout([x, cin]) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let cin = ExtAwi::opaque(self.get_bw(cin));
                let out = incrementer(&x, &cin, true).1;
                self.graft(ptr, v, &[out.state(), x.state(), cin.state()])?;
            }
            CinSum([cin, lhs, rhs]) => {
                let cin = ExtAwi::opaque(self.get_bw(cin));
                let lhs = ExtAwi::opaque(self.get_bw(lhs));
                let rhs = ExtAwi::opaque(self.get_bw(rhs));
                let out = cin_sum(&cin, &lhs, &rhs).0;
                self.graft(ptr, v, &[
                    out.state(),
                    cin.state(),
                    lhs.state(),
                    rhs.state(),
                ])?;
            }
            UnsignedOverflow([cin, lhs, rhs]) => {
                let cin = ExtAwi::opaque(self.get_bw(cin));
                let lhs = ExtAwi::opaque(self.get_bw(lhs));
                let rhs = ExtAwi::opaque(self.get_bw(rhs));
                let out = cin_sum(&cin, &lhs, &rhs).1;
                self.graft(ptr, v, &[
                    out.state(),
                    cin.state(),
                    lhs.state(),
                    rhs.state(),
                ])?;
            }
            SignedOverflow([cin, lhs, rhs]) => {
                let cin = ExtAwi::opaque(self.get_bw(cin));
                let lhs = ExtAwi::opaque(self.get_bw(lhs));
                let rhs = ExtAwi::opaque(self.get_bw(rhs));
                let out = cin_sum(&cin, &lhs, &rhs).2;
                self.graft(ptr, v, &[
                    out.state(),
                    cin.state(),
                    lhs.state(),
                    rhs.state(),
                ])?;
            }
            Neg([x, neg]) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let neg = ExtAwi::opaque(self.get_bw(neg));
                assert_eq!(neg.bw(), 1);
                let out = negator(&x, &neg);
                self.graft(ptr, v, &[out.state(), x.state(), neg.state()])?;
            }
            Abs([x]) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let mut out = x.clone();
                out.neg_assign(x.msb());
                self.graft(ptr, v, &[out.state(), x.state()])?;
            }
            Add([lhs, rhs]) => {
                let lhs = ExtAwi::opaque(self.get_bw(lhs));
                let rhs = ExtAwi::opaque(self.get_bw(rhs));
                let out = cin_sum(&inlawi!(0), &lhs, &rhs).0;
                self.graft(ptr, v, &[out.state(), lhs.state(), rhs.state()])?;
            }
            Sub([lhs, rhs]) => {
                let lhs = ExtAwi::opaque(self.get_bw(lhs));
                let rhs = ExtAwi::opaque(self.get_bw(rhs));
                let mut rhs_tmp = rhs.clone();
                rhs_tmp.neg_assign(true);
                let mut out = lhs.clone();
                out.add_assign(&rhs_tmp).unwrap();
                self.graft(ptr, v, &[out.state(), lhs.state(), rhs.state()])?;
            }
            Rsb([lhs, rhs]) => {
                let lhs = ExtAwi::opaque(self.get_bw(lhs));
                let rhs = ExtAwi::opaque(self.get_bw(rhs));
                let mut out = lhs.clone();
                out.neg_assign(true);
                out.add_assign(&rhs).unwrap();
                self.graft(ptr, v, &[out.state(), lhs.state(), rhs.state()])?;
            }
            FieldTo([lhs, to, rhs, width]) => {
                if self[to].op.is_literal() {
                    let lhs = ExtAwi::opaque(self.get_bw(lhs));
                    let to_u = self.usize(to)?;
                    let rhs = ExtAwi::opaque(self.get_bw(rhs));
                    let width = ExtAwi::opaque(self.get_bw(width));

                    let out = if let Some(w) = NonZeroUsize::new(lhs.bw() - to_u) {
                        let mut lhs_hi = static_field(&ExtAwi::zero(w), 0, &lhs, to_u, w.get());
                        lhs_hi.field_width(&rhs, width.to_usize()).unwrap();
                        static_field(&lhs, to_u, &lhs_hi, 0, w.get())
                    } else {
                        lhs.clone()
                    };
                    self.graft(ptr, v, &[
                        out.state(),
                        lhs.state(),
                        ExtAwi::opaque(self.get_bw(to)).state(),
                        rhs.state(),
                        width.state(),
                    ])?;
                } else {
                    let lhs = ExtAwi::opaque(self.get_bw(lhs));
                    let to = ExtAwi::opaque(self.get_bw(to));
                    let rhs = ExtAwi::opaque(self.get_bw(rhs));
                    let width = ExtAwi::opaque(self.get_bw(width));
                    let out = field_to(&lhs, &to, &rhs, &width);
                    self.graft(ptr, v, &[
                        out.state(),
                        lhs.state(),
                        to.state(),
                        rhs.state(),
                        width.state(),
                    ])?;
                }
            }
            Field([lhs, to, rhs, from, width]) => {
                if self[to].op.is_literal() || self[from].op.is_literal() {
                    let lhs = ExtAwi::opaque(self.get_bw(lhs));
                    let to = ExtAwi::opaque(self.get_bw(to));
                    let rhs = ExtAwi::opaque(self.get_bw(rhs));
                    let from = ExtAwi::opaque(self.get_bw(from));
                    let width = ExtAwi::opaque(self.get_bw(width));

                    let min_w = min(lhs.bw(), rhs.bw());
                    let mut tmp = ExtAwi::zero(NonZeroUsize::new(min_w).unwrap());
                    tmp.field_from(&rhs, from.to_usize(), width.to_usize())
                        .unwrap();
                    let mut out = lhs.clone();
                    out.field_to(to.to_usize(), &tmp, width.to_usize()).unwrap();

                    self.graft(ptr, v, &[
                        out.state(),
                        lhs.state(),
                        to.state(),
                        rhs.state(),
                        from.state(),
                        width.state(),
                    ])?;
                } else {
                    let lhs = ExtAwi::opaque(self.get_bw(lhs));
                    let to = ExtAwi::opaque(self.get_bw(to));
                    let rhs = ExtAwi::opaque(self.get_bw(rhs));
                    let from = ExtAwi::opaque(self.get_bw(from));
                    let width = ExtAwi::opaque(self.get_bw(width));
                    let out = field(&lhs, &to, &rhs, &from, &width);
                    self.graft(ptr, v, &[
                        out.state(),
                        lhs.state(),
                        to.state(),
                        rhs.state(),
                        from.state(),
                        width.state(),
                    ])?;
                }
            }
            op => return Err(EvalError::OtherString(format!("unimplemented: {:?}", op))),
        }
        Ok(false)
    }

    /// Lowers all nodes in the tree of `leaf` not set to `visit`.
    pub fn lower_tree(&mut self, leaf: PNode, visit: u64) -> Result<(), EvalError> {
        let base_visit = visit;
        // We must use a DFS, one reason being that if downstream nodes are lowered
        // before upstream ones, we can easily end up in situations where a lower would
        // have been much simpler because of constants being propogated down.

        let mut unimplemented = false;
        let mut path: Vec<(usize, PNode)> = vec![(0, leaf)];
        loop {
            let (i, p) = path[path.len() - 1];
            let ops = self[p].op.operands();
            if i >= ops.len() {
                // checked all sources
                self.visit_gen += 1;
                let this_visit = self.visit_gen;
                // newly lowered nodes will be set to `this_visit` so that the DFS reexplores
                let eval = match self.lower_node(p, this_visit) {
                    Ok(lowered) => lowered,
                    Err(EvalError::Unimplemented) => {
                        // we lower as much as possible
                        unimplemented = true;
                        true
                    }
                    Err(e) => {
                        self[p].err = Some(e.clone());
                        return Err(e)
                    }
                };
                if eval {
                    match self.eval_tree(p, base_visit) {
                        Ok(()) => {}
                        Err(EvalError::Unevaluatable) => {}
                        Err(e) => {
                            self[p].err = Some(e.clone());
                            return Err(e)
                        }
                    }
                    path.pop().unwrap();
                    if path.is_empty() {
                        break
                    }
                } else {
                    // else do not call `path.pop`, restart the DFS here
                    path.last_mut().unwrap().0 = 0;
                }
            } else {
                let p_next = ops[i];
                let next_visit = self[p_next].visit;
                if next_visit == base_visit {
                    // peek at node for evaluatableness but do not visit node
                    path.last_mut().unwrap().0 += 1;
                } else {
                    self[p_next].visit = base_visit;
                    path.push((0, p_next));
                }
            }
        }

        if unimplemented {
            Err(EvalError::Unimplemented)
        } else {
            Ok(())
        }
    }
}
