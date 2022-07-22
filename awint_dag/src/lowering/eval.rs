use std::fmt::Write;

use awint_core::Bits;
use awint_ext::ExtAwi;
use triple_arena::{Ptr, PtrTrait};
use Op::*;

use crate::{
    common::{EvalError, Op},
    lowering::Dag,
};

impl<P: PtrTrait> Dag<P> {
    /// Assumes the node itself is evaluatable and all sources for `node` are
    /// literals. Note: decrements dependents but does not remove dead nodes.
    pub fn eval_node(&mut self, node: Ptr<P>) -> Result<(), EvalError> {
        macro_rules! check_bw {
            ($lhs:expr, $rhs:expr) => {
                if $lhs != $rhs {
                    return Err(EvalError::WrongBitwidth)
                }
            };
        }
        let op = self[node].op.take();
        for source in op.operands() {
            self[source].rc = if let Some(x) = self[source].rc.checked_sub(1) {
                x
            } else {
                return Err(EvalError::OtherStr("tried to subtract a 0 reference count"))
            };
        }
        let self_w = if let Some(w) = self[node].nzbw {
            w
        } else {
            return Err(EvalError::NonStaticBitwidth)
        };
        // need this for errors
        let op_err = op.clone();
        let mut r = ExtAwi::zero(self_w);
        let option = match op {
            Invalid => return Err(EvalError::Unevaluatable),
            Opaque(_) => return Err(EvalError::Unevaluatable),
            Literal(_) => return Err(EvalError::Unevaluatable),
            StaticLut([a], lit) => r.lut(&lit, self.lit(a)),
            StaticGet([a], inx) => {
                if let Some(b) = self.lit(a).get(inx) {
                    r.bool_assign(b);
                    Some(())
                } else {
                    None
                }
            }
            StaticSet([a, b], inx) => {
                if r.copy_assign(self.lit(a)).is_some() {
                    r.set(inx, self.bool(b)?)
                } else {
                    None
                }
            }
            Resize([a, b], w) => {
                check_bw!(w, self_w);
                r.resize_assign(self.lit(a), self.bool(b)?);
                Some(())
            }
            ZeroResize([a], w) => {
                check_bw!(w, self_w);
                r.zero_resize_assign(self.lit(a));
                Some(())
            }
            SignResize([a], w) => {
                check_bw!(w, self_w);
                r.sign_resize_assign(self.lit(a));
                Some(())
            }
            Copy([a]) => r.copy_assign(self.lit(a)),
            Lut([a, b], w) => {
                check_bw!(w, self_w);
                r.lut(self.lit(a), self.lit(b))
            }
            Funnel([a, b]) => r.funnel(self.lit(a), self.lit(b)),
            CinSum([a, b, c]) => {
                if r.cin_sum_assign(self.bool(a)?, self.lit(b), self.lit(c))
                    .is_some()
                {
                    Some(())
                } else {
                    None
                }
            }
            Not([a]) => {
                let e = r.copy_assign(self.lit(a));
                r.not_assign();
                e
            }
            Rev([a]) => {
                let e = r.copy_assign(self.lit(a));
                r.rev_assign();
                e
            }
            Abs([a]) => {
                let e = r.copy_assign(self.lit(a));
                r.abs_assign();
                e
            }
            IsZero([a]) => {
                r.bool_assign(self.lit(a).is_zero());
                Some(())
            }
            IsUmax([a]) => {
                r.bool_assign(self.lit(a).is_umax());
                Some(())
            }
            IsImax([a]) => {
                r.bool_assign(self.lit(a).is_imax());
                Some(())
            }
            IsImin([a]) => {
                r.bool_assign(self.lit(a).is_imin());
                Some(())
            }
            IsUone([a]) => {
                r.bool_assign(self.lit(a).is_uone());
                Some(())
            }
            Lsb([a]) => {
                r.bool_assign(self.lit(a).lsb());
                Some(())
            }
            Msb([a]) => {
                r.bool_assign(self.lit(a).msb());
                Some(())
            }
            Lz([a]) => {
                r.usize_assign(self.lit(a).lz());
                Some(())
            }
            Tz([a]) => {
                r.usize_assign(self.lit(a).tz());
                Some(())
            }
            Sig([a]) => {
                r.usize_assign(self.lit(a).sig());
                Some(())
            }
            CountOnes([a]) => {
                r.usize_assign(self.lit(a).count_ones());
                Some(())
            }
            Or([a, b]) => {
                if r.copy_assign(self.lit(a)).is_some() {
                    r.or_assign(self.lit(b))
                } else {
                    None
                }
            }
            And([a, b]) => {
                if r.copy_assign(self.lit(a)).is_some() {
                    r.and_assign(self.lit(b))
                } else {
                    None
                }
            }
            Xor([a, b]) => {
                if r.copy_assign(self.lit(a)).is_some() {
                    r.xor_assign(self.lit(b))
                } else {
                    None
                }
            }
            Shl([a, b]) => {
                if r.copy_assign(self.lit(a)).is_some() {
                    r.shl_assign(self.usize(b)?)
                } else {
                    None
                }
            }
            Lshr([a, b]) => {
                if r.copy_assign(self.lit(a)).is_some() {
                    r.lshr_assign(self.usize(b)?)
                } else {
                    None
                }
            }
            Ashr([a, b]) => {
                if r.copy_assign(self.lit(a)).is_some() {
                    r.ashr_assign(self.usize(b)?)
                } else {
                    None
                }
            }
            Rotl([a, b]) => {
                if r.copy_assign(self.lit(a)).is_some() {
                    r.rotl_assign(self.usize(b)?)
                } else {
                    None
                }
            }
            Rotr([a, b]) => {
                if r.copy_assign(self.lit(a)).is_some() {
                    r.rotr_assign(self.usize(b)?)
                } else {
                    None
                }
            }
            Add([a, b]) => {
                if r.copy_assign(self.lit(a)).is_some() {
                    r.add_assign(self.lit(b))
                } else {
                    None
                }
            }
            Sub([a, b]) => {
                if r.copy_assign(self.lit(a)).is_some() {
                    r.sub_assign(self.lit(b))
                } else {
                    None
                }
            }
            Rsb([a, b]) => {
                if r.copy_assign(self.lit(a)).is_some() {
                    r.rsb_assign(self.lit(b))
                } else {
                    None
                }
            }
            Eq([a, b]) => {
                if let Some(b) = self.lit(a).const_eq(self.lit(b)) {
                    r.bool_assign(b);
                    Some(())
                } else {
                    None
                }
            }
            Ne([a, b]) => {
                if let Some(b) = self.lit(a).const_ne(self.lit(b)) {
                    r.bool_assign(b);
                    Some(())
                } else {
                    None
                }
            }
            Ult([a, b]) => {
                if let Some(b) = self.lit(a).ult(self.lit(b)) {
                    r.bool_assign(b);
                    Some(())
                } else {
                    None
                }
            }
            Ule([a, b]) => {
                if let Some(b) = self.lit(a).ule(self.lit(b)) {
                    r.bool_assign(b);
                    Some(())
                } else {
                    None
                }
            }
            Ilt([a, b]) => {
                if let Some(b) = self.lit(a).ilt(self.lit(b)) {
                    r.bool_assign(b);
                    Some(())
                } else {
                    None
                }
            }
            Ile([a, b]) => {
                if let Some(b) = self.lit(a).ile(self.lit(b)) {
                    r.bool_assign(b);
                    Some(())
                } else {
                    None
                }
            }
            Inc([a, b]) => {
                if r.copy_assign(self.lit(a)).is_some() {
                    r.inc_assign(self.bool(b)?);
                    Some(())
                } else {
                    None
                }
            }
            Dec([a, b]) => {
                if r.copy_assign(self.lit(a)).is_some() {
                    r.dec_assign(self.bool(b)?);
                    Some(())
                } else {
                    None
                }
            }
            Neg([a, b]) => {
                let e = r.copy_assign(self.lit(a));
                r.neg_assign(self.bool(b)?);
                e
            }
            ZeroResizeOverflow([a], w) => {
                let mut tmp_awi = ExtAwi::zero(w);
                r.bool_assign(tmp_awi.zero_resize_assign(self.lit(a)));
                Some(())
            }
            SignResizeOverflow([a], w) => {
                let mut tmp_awi = ExtAwi::zero(w);
                r.bool_assign(tmp_awi.sign_resize_assign(self.lit(a)));
                Some(())
            }
            Get([a, b]) => {
                if let Some(b) = self.lit(a).get(self.usize(b)?) {
                    r.bool_assign(b);
                    Some(())
                } else {
                    None
                }
            }
            Set([a, b, c]) => {
                if r.copy_assign(self.lit(a)).is_some() {
                    r.set(self.usize(b)?, self.bool(c)?)
                } else {
                    None
                }
            }
            LutSet([a, b, c]) => {
                if r.copy_assign(self.lit(a)).is_some() {
                    r.lut_set(self.lit(b), self.lit(c))
                } else {
                    None
                }
            }
            Field(v) => {
                if r.copy_assign(self.lit(v[0])).is_some() {
                    r.field(
                        self.usize(v[1])?,
                        self.lit(v[2]),
                        self.usize(v[3])?,
                        self.usize(v[4])?,
                    )
                } else {
                    None
                }
            }
            FieldTo([a, b, c, d]) => {
                if r.copy_assign(self.lit(a)).is_some() {
                    r.field_to(self.usize(b)?, self.lit(c), self.usize(d)?)
                } else {
                    None
                }
            }
            FieldFrom([a, b, c, d]) => {
                if r.copy_assign(self.lit(a)).is_some() {
                    r.field_from(self.lit(b), self.usize(c)?, self.usize(d)?)
                } else {
                    None
                }
            }
            FieldWidth([a, b, c]) => {
                if r.copy_assign(self.lit(a)).is_some() {
                    r.field_width(self.lit(b), self.usize(c)?)
                } else {
                    None
                }
            }
            FieldBit([a, b, c, d]) => {
                if r.copy_assign(self.lit(a)).is_some() {
                    r.field_bit(self.usize(b)?, self.lit(c), self.usize(d)?)
                } else {
                    None
                }
            }
            MulAdd([a, b, c]) => {
                if r.copy_assign(self.lit(a)).is_some() {
                    r.mul_add_assign(self.lit(b), self.lit(c))
                } else {
                    None
                }
            }
            UQuo([a, b]) => {
                let mut t = ExtAwi::zero(self_w);
                Bits::udivide(&mut r, &mut t, self.lit(a), self.lit(b))
            }
            URem([a, b]) => {
                let mut t = ExtAwi::zero(self_w);
                Bits::udivide(&mut t, &mut r, self.lit(a), self.lit(b))
            }
            UnsignedOverflow([a, b, c]) => {
                // note that `self_w` and `self.get_bw(a)` are both 1
                let mut t = ExtAwi::zero(self.get_bw(b)?);
                if let Some((o, _)) = t.cin_sum_assign(self.bool(a)?, self.lit(b), self.lit(c)) {
                    r.bool_assign(o);
                    Some(())
                } else {
                    None
                }
            }
            SignedOverflow([a, b, c]) => {
                let mut t = ExtAwi::zero(self.get_bw(b)?);
                if let Some((_, o)) = t.cin_sum_assign(self.bool(a)?, self.lit(b), self.lit(c)) {
                    r.bool_assign(o);
                    Some(())
                } else {
                    None
                }
            }
            IncCout([a, b]) => {
                let mut t = ExtAwi::zero(self.get_bw(a)?);
                if t.copy_assign(self.lit(a)).is_some() {
                    r.bool_assign(t.inc_assign(self.bool(b)?));
                    Some(())
                } else {
                    None
                }
            }
            DecCout([a, b]) => {
                let mut t = ExtAwi::zero(self.get_bw(a)?);
                if t.copy_assign(self.lit(a)).is_some() {
                    r.bool_assign(t.dec_assign(self.bool(b)?));
                    Some(())
                } else {
                    None
                }
            }
            op @ (IQuo(_) | IRem(_)) => {
                let mut t = ExtAwi::zero(self_w);
                let mut t0 = ExtAwi::zero(self_w);
                let mut t1 = ExtAwi::zero(self_w);
                match op {
                    IQuo([a, b]) => {
                        if let (Some(()), Some(())) =
                            (t0.copy_assign(self.lit(a)), t1.copy_assign(self.lit(b)))
                        {
                            Bits::idivide(&mut r, &mut t, &mut t0, &mut t1)
                        } else {
                            None
                        }
                    }
                    IRem([a, b]) => {
                        if let (Some(()), Some(())) =
                            (t0.copy_assign(self.lit(a)), t1.copy_assign(self.lit(b)))
                        {
                            Bits::idivide(&mut t, &mut r, &mut t0, &mut t1)
                        } else {
                            None
                        }
                    }
                    _ => unreachable!(),
                }
            }
        };
        if option.is_none() {
            let operands = op_err.operands();
            let mut s = String::new();
            for op in operands {
                write!(s, "{:?}, ", self[op]).unwrap();
            }
            Err(EvalError::OtherString(format!(
                "evaluation failure on operation {:?} ({})",
                op_err, s
            )))
        } else {
            self[node].op = Literal(r);
            Ok(())
        }
    }

    /// Evaluates the tree leading to `leaf` as much as possible
    pub fn eval_tree(&mut self, leaf: Ptr<P>) -> Result<(), EvalError> {
        self.visit_gen += 1;
        let gen = self.visit_gen;
        // DFS from leaf to roots
        // the bool is set to false when an unevaluatabe node is in the sources
        let mut path: Vec<(usize, Ptr<P>, bool)> = vec![(0, leaf, true)];
        loop {
            let (i, p, b) = path[path.len() - 1];
            /*if !self.dag.contains(p) {
                self.render_to_svg_file(std::path::PathBuf::from("rendered.svg"))
                    .unwrap();
                panic!();
            }*/
            let ops = self[p].op.operands();
            if ops.is_empty() {
                // reached a root
                path.pop().unwrap();
                if path.is_empty() {
                    break
                }
                path.last_mut().unwrap().0 += 1;
                if !self[p].op.is_literal() {
                    // is an `Invalid` or `Opaque`
                    path.last_mut().unwrap().2 = false;
                }
            } else if i >= ops.len() {
                // checked all sources
                path.pop().unwrap();
                if b {
                    if let Err(e) = self.eval_node(p) {
                        self[p].err = Some(e.clone());
                        return Err(e)
                    }
                }
                if path.is_empty() {
                    break
                }
                if !b {
                    path.last_mut().unwrap().2 = false;
                }
            } else {
                let next_p = ops[i];
                if self[next_p].visit_num == gen {
                    // peek at node for evaluatableness but do not visit node, this prevents
                    // exponential growth
                    path.last_mut().unwrap().0 += 1;
                    path.last_mut().unwrap().2 &= self[next_p].op.is_literal();
                } else {
                    self[next_p].visit_num = gen;
                    path.push((0, next_p, true));
                }
            }
        }
        Ok(())
    }

    pub fn cull(&mut self) {
        // cull unused nodes
        for p in self.ptrs() {
            if self[p].rc == 0 {
                self.dag.remove(p).unwrap();
            }
        }
    }
}
