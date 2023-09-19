//! Lowers everything into LUT form

// TODO https://github.com/rust-lang/rust-clippy/issues/10577
#![allow(clippy::redundant_clone)]

use std::{cmp::min, num::NonZeroUsize};

use awint_ext::bw;
use awint_macro_internals::triple_arena::Advancer;
use awint_macros::{extawi, inlawi};

use crate::{
    lowering::{meta::*, OpDag, PNode},
    mimick::{Bits, ExtAwi, InlAwi},
    EvalError, Lineage,
    Op::{self, *},
    StateEpoch,
};

impl OpDag {
    /// Lowers everything down to `Invalid`, `Opaque`, `Copy`, `StaticGet`,
    /// `StaticSet`, and `StaticLut`. Nodes that get lowered are
    /// colored with `visit`. Returns `true` if the node is already lowered.
    pub fn lower_node(&mut self, ptr: PNode, visit: u64) -> Result<bool, EvalError> {
        // create a temporary epoch for the grafting in this function
        let epoch = StateEpoch::new();
        if !self.a.contains(ptr) {
            return Err(EvalError::InvalidPtr)
        }
        let start_op = self[ptr].op.clone();
        let out_w = self.get_bw(ptr);
        let v = visit;
        match start_op {
            Invalid => return Err(EvalError::OtherStr("encountered `Invalid` in lowering")),
            Opaque(..) | Literal(_) | Copy(_) | StaticLut(..) | StaticGet(..) | StaticSet(..) => {
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
                    self.graft(epoch, ptr, v, &[out.state(), lut.state(), inx.state()])?;
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
                    self.graft(epoch, ptr, v, &[out.state(), bits.state(), inx.state()])?;
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
                    self.graft(epoch, ptr, v, &[
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
                out.set(to.to_usize(), bit).unwrap();
                self.graft(epoch, ptr, v, &[
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
                self.graft(epoch, ptr, v, &[out.state(), x.state()])?;
            }
            SignResize([x]) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let out = resize(&x, out_w, true);
                self.graft(epoch, ptr, v, &[out.state(), x.state()])?;
            }
            Resize([x, b]) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let b = ExtAwi::opaque(self.get_bw(b));
                let out = resize_cond(&x, out_w, &b);
                self.graft(epoch, ptr, v, &[out.state(), x.state(), b.state()])?;
            }
            Lsb([x]) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let out = x.get(0).unwrap();
                self.graft(epoch, ptr, v, &[out.state(), x.state()])?;
            }
            Msb([x]) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let out = x.get(x.bw() - 1).unwrap();
                self.graft(epoch, ptr, v, &[out.state(), x.state()])?;
            }
            FieldWidth([lhs, rhs, width]) => {
                let lhs_bw = self.get_bw(lhs);
                let rhs_bw = self.get_bw(rhs);
                if self[width].op.is_literal() {
                    let lhs = ExtAwi::opaque(lhs_bw);
                    let rhs = ExtAwi::opaque(rhs_bw);
                    let out = static_field(&lhs, 0, &rhs, 0, self.usize(width).unwrap()).0;
                    self.graft(epoch, ptr, v, &[
                        out.state(),
                        lhs.state(),
                        rhs.state(),
                        ExtAwi::opaque(self.get_bw(width)).state(),
                    ])?;
                } else {
                    let lhs = ExtAwi::opaque(lhs_bw);
                    let rhs = ExtAwi::opaque(rhs_bw);
                    let width = ExtAwi::opaque(self.get_bw(width));
                    let fail = width.ugt(&InlAwi::from_usize(lhs_bw.get())).unwrap()
                        | width.ugt(&InlAwi::from_usize(rhs_bw.get())).unwrap();
                    let mut tmp_width = width.clone();
                    tmp_width.mux_(&InlAwi::from_usize(0), fail).unwrap();
                    let out = field_width(&lhs, &rhs, &tmp_width);
                    self.graft(epoch, ptr, v, &[
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
                let out = funnel_(&x, &s);
                self.graft(epoch, ptr, v, &[out.state(), x.state(), s.state()])?;
            }
            FieldFrom([lhs, rhs, from, width]) => {
                let lhs_bw = self.get_bw(lhs);
                let rhs_bw = self.get_bw(rhs);
                if self[from].op.is_literal() {
                    let lhs = ExtAwi::opaque(lhs_bw);
                    let rhs = ExtAwi::opaque(rhs_bw);
                    let width = ExtAwi::opaque(self.get_bw(width));
                    let from_u = self.usize(from)?;
                    let out = if let Some(w) = NonZeroUsize::new(rhs.bw() - from_u) {
                        let tmp0 = ExtAwi::zero(w);
                        let (tmp1, o) = static_field(&tmp0, 0, &rhs, from_u, rhs.bw() - from_u);
                        let mut out = lhs.clone();
                        if o {
                            out
                        } else {
                            out.field_width(&tmp1, width.to_usize()).unwrap();
                            out
                        }
                    } else {
                        lhs.clone()
                    };
                    self.graft(epoch, ptr, v, &[
                        out.state(),
                        lhs.state(),
                        rhs.state(),
                        ExtAwi::opaque(self.get_bw(from)).state(),
                        width.state(),
                    ])?;
                } else {
                    let lhs = ExtAwi::opaque(lhs_bw);
                    let rhs = ExtAwi::opaque(rhs_bw);
                    let from = ExtAwi::opaque(self.get_bw(from));
                    let width = ExtAwi::opaque(self.get_bw(width));
                    let mut tmp = InlAwi::from_usize(rhs_bw.get());
                    tmp.sub_(&width).unwrap();
                    // the other two fail conditions are in `field_width`
                    let fail = from.ugt(&tmp).unwrap();
                    let mut tmp_width = width.clone();
                    tmp_width.mux_(&InlAwi::from_usize(0), fail).unwrap();
                    // the optimizations on `width` are done later on an inner `field_width` call
                    let out = field_from(&lhs, &rhs, &from, &tmp_width);
                    self.graft(epoch, ptr, v, &[
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
                    let out = static_field(&tmp, s_u, &x, 0, x.bw() - s_u).0;
                    self.graft(epoch, ptr, v, &[
                        out.state(),
                        x.state(),
                        ExtAwi::opaque(self.get_bw(s)).state(),
                    ])?;
                } else {
                    let x = ExtAwi::opaque(self.get_bw(x));
                    let s = ExtAwi::opaque(self.get_bw(s));
                    let out = shl(&x, &s);
                    self.graft(epoch, ptr, v, &[out.state(), x.state(), s.state()])?;
                }
            }
            Lshr([x, s]) => {
                if self[s].op.is_literal() {
                    let x = ExtAwi::opaque(self.get_bw(x));
                    let s_u = self.usize(s)?;
                    let tmp = ExtAwi::zero(x.nzbw());
                    let out = static_field(&tmp, 0, &x, s_u, x.bw() - s_u).0;
                    self.graft(epoch, ptr, v, &[
                        out.state(),
                        x.state(),
                        ExtAwi::opaque(self.get_bw(s)).state(),
                    ])?;
                } else {
                    let x = ExtAwi::opaque(self.get_bw(x));
                    let s = ExtAwi::opaque(self.get_bw(s));
                    let out = lshr(&x, &s);
                    self.graft(epoch, ptr, v, &[out.state(), x.state(), s.state()])?;
                }
            }
            Ashr([x, s]) => {
                if self[s].op.is_literal() {
                    let x = ExtAwi::opaque(self.get_bw(x));
                    let s_u = self.usize(s)?;
                    let mut tmp = ExtAwi::zero(x.nzbw());
                    for i in 0..x.bw() {
                        tmp.set(i, x.msb()).unwrap();
                    }
                    let out = static_field(&tmp, 0, &x, s_u, x.bw() - s_u).0;
                    self.graft(epoch, ptr, v, &[
                        out.state(),
                        x.state(),
                        ExtAwi::opaque(self.get_bw(s)).state(),
                    ])?;
                } else {
                    let x = ExtAwi::opaque(self.get_bw(x));
                    let s = ExtAwi::opaque(self.get_bw(s));
                    let out = ashr(&x, &s);
                    self.graft(epoch, ptr, v, &[out.state(), x.state(), s.state()])?;
                }
            }
            Rotl([x, s]) => {
                if self[s].op.is_literal() {
                    let x = ExtAwi::opaque(self.get_bw(x));
                    let s_u = self.usize(s)?;
                    let out = if s_u == 0 {
                        x.clone()
                    } else {
                        let tmp = static_field(&ExtAwi::zero(x.nzbw()), s_u, &x, 0, x.bw() - s_u).0;
                        static_field(&tmp, 0, &x, x.bw() - s_u, s_u).0
                    };
                    self.graft(epoch, ptr, v, &[
                        out.state(),
                        x.state(),
                        ExtAwi::opaque(self.get_bw(s)).state(),
                    ])?;
                } else {
                    let x = ExtAwi::opaque(self.get_bw(x));
                    let s = ExtAwi::opaque(self.get_bw(s));
                    let out = rotl(&x, &s);
                    self.graft(epoch, ptr, v, &[out.state(), x.state(), s.state()])?;
                }
            }
            Rotr([x, s]) => {
                if self[s].op.is_literal() {
                    let x = ExtAwi::opaque(self.get_bw(x));
                    let s_u = self.usize(s)?;
                    let out = if s_u == 0 {
                        x.clone()
                    } else {
                        let tmp = static_field(&ExtAwi::zero(x.nzbw()), 0, &x, s_u, x.bw() - s_u).0;
                        static_field(&tmp, x.bw() - s_u, &x, 0, s_u).0
                    };
                    self.graft(epoch, ptr, v, &[
                        out.state(),
                        x.state(),
                        ExtAwi::opaque(self.get_bw(s)).state(),
                    ])?;
                } else {
                    let x = ExtAwi::opaque(self.get_bw(x));
                    let s = ExtAwi::opaque(self.get_bw(s));
                    let out = rotr(&x, &s);
                    self.graft(epoch, ptr, v, &[out.state(), x.state(), s.state()])?;
                }
            }
            Not([x]) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let out = bitwise_not(&x);
                self.graft(epoch, ptr, v, &[out.state(), x.state()])?;
            }
            Or([lhs, rhs]) => {
                let lhs = ExtAwi::opaque(self.get_bw(lhs));
                let rhs = ExtAwi::opaque(self.get_bw(rhs));
                let out = bitwise(&lhs, &rhs, inlawi!(1110));
                self.graft(epoch, ptr, v, &[out.state(), lhs.state(), rhs.state()])?;
            }
            And([lhs, rhs]) => {
                let lhs = ExtAwi::opaque(self.get_bw(lhs));
                let rhs = ExtAwi::opaque(self.get_bw(rhs));
                let out = bitwise(&lhs, &rhs, inlawi!(1000));
                self.graft(epoch, ptr, v, &[out.state(), lhs.state(), rhs.state()])?;
            }
            Xor([lhs, rhs]) => {
                let lhs = ExtAwi::opaque(self.get_bw(lhs));
                let rhs = ExtAwi::opaque(self.get_bw(rhs));
                let out = bitwise(&lhs, &rhs, inlawi!(0110));
                self.graft(epoch, ptr, v, &[out.state(), lhs.state(), rhs.state()])?;
            }
            Inc([x, cin]) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let cin = ExtAwi::opaque(self.get_bw(cin));
                let out = incrementer(&x, &cin, false).0;
                self.graft(epoch, ptr, v, &[out.state(), x.state(), cin.state()])?;
            }
            IncCout([x, cin]) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let cin = ExtAwi::opaque(self.get_bw(cin));
                let out = incrementer(&x, &cin, false).1;
                self.graft(epoch, ptr, v, &[out.state(), x.state(), cin.state()])?;
            }
            Dec([x, cin]) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let cin = ExtAwi::opaque(self.get_bw(cin));
                let out = incrementer(&x, &cin, true).0;
                self.graft(epoch, ptr, v, &[out.state(), x.state(), cin.state()])?;
            }
            DecCout([x, cin]) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let cin = ExtAwi::opaque(self.get_bw(cin));
                let out = incrementer(&x, &cin, true).1;
                self.graft(epoch, ptr, v, &[out.state(), x.state(), cin.state()])?;
            }
            CinSum([cin, lhs, rhs]) => {
                let cin = ExtAwi::opaque(self.get_bw(cin));
                let lhs = ExtAwi::opaque(self.get_bw(lhs));
                let rhs = ExtAwi::opaque(self.get_bw(rhs));
                let out = cin_sum(&cin, &lhs, &rhs).0;
                self.graft(epoch, ptr, v, &[
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
                self.graft(epoch, ptr, v, &[
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
                self.graft(epoch, ptr, v, &[
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
                self.graft(epoch, ptr, v, &[out.state(), x.state(), neg.state()])?;
            }
            Abs([x]) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let mut out = x.clone();
                out.neg_(x.msb());
                self.graft(epoch, ptr, v, &[out.state(), x.state()])?;
            }
            Add([lhs, rhs]) => {
                let lhs = ExtAwi::opaque(self.get_bw(lhs));
                let rhs = ExtAwi::opaque(self.get_bw(rhs));
                let out = cin_sum(&inlawi!(0), &lhs, &rhs).0;
                self.graft(epoch, ptr, v, &[out.state(), lhs.state(), rhs.state()])?;
            }
            Sub([lhs, rhs]) => {
                let lhs = ExtAwi::opaque(self.get_bw(lhs));
                let rhs = ExtAwi::opaque(self.get_bw(rhs));
                let mut rhs_tmp = rhs.clone();
                rhs_tmp.neg_(true);
                let mut out = lhs.clone();
                out.add_(&rhs_tmp).unwrap();
                self.graft(epoch, ptr, v, &[out.state(), lhs.state(), rhs.state()])?;
            }
            Rsb([lhs, rhs]) => {
                let lhs = ExtAwi::opaque(self.get_bw(lhs));
                let rhs = ExtAwi::opaque(self.get_bw(rhs));
                let mut out = lhs.clone();
                out.neg_(true);
                out.add_(&rhs).unwrap();
                self.graft(epoch, ptr, v, &[out.state(), lhs.state(), rhs.state()])?;
            }
            FieldTo([lhs, to, rhs, width]) => {
                if self[to].op.is_literal() {
                    let lhs = ExtAwi::opaque(self.get_bw(lhs));
                    let to_u = self.usize(to)?;
                    let rhs = ExtAwi::opaque(self.get_bw(rhs));
                    let width = ExtAwi::opaque(self.get_bw(width));

                    let out = if let Some(w) = NonZeroUsize::new(lhs.bw() - to_u) {
                        let (mut lhs_hi, o) =
                            static_field(&ExtAwi::zero(w), 0, &lhs, to_u, w.get());
                        lhs_hi.field_width(&rhs, width.to_usize()).unwrap();
                        if o {
                            lhs.clone()
                        } else {
                            static_field(&lhs, to_u, &lhs_hi, 0, w.get()).0
                        }
                    } else {
                        lhs.clone()
                    };
                    self.graft(epoch, ptr, v, &[
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
                    self.graft(epoch, ptr, v, &[
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

                    self.graft(epoch, ptr, v, &[
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
                    self.graft(epoch, ptr, v, &[
                        out.state(),
                        lhs.state(),
                        to.state(),
                        rhs.state(),
                        from.state(),
                        width.state(),
                    ])?;
                }
            }
            Rev([x]) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let mut out = ExtAwi::zero(x.nzbw());
                for i in 0..x.bw() {
                    out.set(i, x.get(x.bw() - 1 - i).unwrap()).unwrap()
                }
                self.graft(epoch, ptr, v, &[out.state(), x.state()])?;
            }
            Eq([lhs, rhs]) => {
                let lhs = ExtAwi::opaque(self.get_bw(lhs));
                let rhs = ExtAwi::opaque(self.get_bw(rhs));
                let out = equal(&lhs, &rhs);
                self.graft(epoch, ptr, v, &[out.state(), lhs.state(), rhs.state()])?;
            }
            Ne([lhs, rhs]) => {
                let lhs = ExtAwi::opaque(self.get_bw(lhs));
                let rhs = ExtAwi::opaque(self.get_bw(rhs));
                let mut out = equal(&lhs, &rhs);
                out.not_();
                self.graft(epoch, ptr, v, &[out.state(), lhs.state(), rhs.state()])?;
            }
            Ult([lhs, rhs]) => {
                let w = self.get_bw(lhs);
                let lhs = ExtAwi::opaque(w);
                let rhs = ExtAwi::opaque(self.get_bw(rhs));
                let mut not_lhs = lhs.clone();
                not_lhs.not_();
                let mut tmp = ExtAwi::zero(w);
                // TODO should probably use some short termination circuit like what
                // `tsmear_inx` uses
                let (out, _) = tmp.cin_sum_(false, &not_lhs, &rhs).unwrap();
                self.graft(epoch, ptr, v, &[out.state(), lhs.state(), rhs.state()])?;
            }
            Ule([lhs, rhs]) => {
                let w = self.get_bw(lhs);
                let lhs = ExtAwi::opaque(w);
                let rhs = ExtAwi::opaque(self.get_bw(rhs));
                let mut not_lhs = lhs.clone();
                not_lhs.not_();
                let mut tmp = ExtAwi::zero(w);
                let (out, _) = tmp.cin_sum_(true, &not_lhs, &rhs).unwrap();
                self.graft(epoch, ptr, v, &[out.state(), lhs.state(), rhs.state()])?;
            }
            Ilt([lhs, rhs]) => {
                let w = self.get_bw(lhs);
                let lhs = ExtAwi::opaque(w);
                let rhs = ExtAwi::opaque(self.get_bw(rhs));
                let mut out = inlawi!(0);
                if w.get() == 1 {
                    let mut tmp = inlawi!(00);
                    tmp.set(0, lhs.msb()).unwrap();
                    tmp.set(1, rhs.msb()).unwrap();
                    out.lut_(&inlawi!(0010), &tmp).unwrap();
                } else {
                    let lhs_lo = extawi!(lhs[..(lhs.bw() - 1)]).unwrap();
                    let rhs_lo = extawi!(rhs[..(rhs.bw() - 1)]).unwrap();
                    let lo_lt = lhs_lo.ult(&rhs_lo).unwrap();
                    let mut tmp = inlawi!(000);
                    tmp.set(0, lo_lt).unwrap();
                    tmp.set(1, lhs.msb()).unwrap();
                    tmp.set(2, rhs.msb()).unwrap();
                    // if `lhs.msb() != rhs.msb()` then `lhs.msb()` determines signed-less-than,
                    // otherwise `lo_lt` determines
                    out.lut_(&inlawi!(10001110), &tmp).unwrap();
                }
                self.graft(epoch, ptr, v, &[out.state(), lhs.state(), rhs.state()])?;
            }
            Ile([lhs, rhs]) => {
                let w = self.get_bw(lhs);
                let lhs = ExtAwi::opaque(w);
                let rhs = ExtAwi::opaque(self.get_bw(rhs));
                let mut out = inlawi!(0);
                if w.get() == 1 {
                    let mut tmp = inlawi!(00);
                    tmp.set(0, lhs.msb()).unwrap();
                    tmp.set(1, rhs.msb()).unwrap();
                    out.lut_(&inlawi!(1011), &tmp).unwrap();
                } else {
                    let lhs_lo = extawi!(lhs[..(lhs.bw() - 1)]).unwrap();
                    let rhs_lo = extawi!(rhs[..(rhs.bw() - 1)]).unwrap();
                    let lo_lt = lhs_lo.ule(&rhs_lo).unwrap();
                    let mut tmp = inlawi!(000);
                    tmp.set(0, lo_lt).unwrap();
                    tmp.set(1, lhs.msb()).unwrap();
                    tmp.set(2, rhs.msb()).unwrap();
                    out.lut_(&inlawi!(10001110), &tmp).unwrap();
                }
                self.graft(epoch, ptr, v, &[out.state(), lhs.state(), rhs.state()])?;
            }
            op @ (IsZero(_) | IsUmax(_) | IsImax(_) | IsImin(_) | IsUone(_)) => {
                let x = ExtAwi::opaque(self.get_bw(op.operands()[0]));
                let w = x.bw();
                let out = InlAwi::from(match op {
                    IsZero(_) => x.const_eq(&extawi!(zero: ..w).unwrap()).unwrap(),
                    IsUmax(_) => x.const_eq(&extawi!(umax: ..w).unwrap()).unwrap(),
                    IsImax(_) => x.const_eq(&extawi!(imax: ..w).unwrap()).unwrap(),
                    IsImin(_) => x.const_eq(&extawi!(imin: ..w).unwrap()).unwrap(),
                    IsUone(_) => x.const_eq(&extawi!(uone: ..w).unwrap()).unwrap(),
                    _ => unreachable!(),
                });
                self.graft(epoch, ptr, v, &[out.state(), x.state()])?;
            }
            CountOnes([x]) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let out = count_ones(&x).to_usize();
                self.graft(epoch, ptr, v, &[out.state(), x.state()])?;
            }
            Lz([x]) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let out = leading_zeros(&x).to_usize();
                self.graft(epoch, ptr, v, &[out.state(), x.state()])?;
            }
            Tz([x]) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let out = trailing_zeros(&x).to_usize();
                self.graft(epoch, ptr, v, &[out.state(), x.state()])?;
            }
            Sig([x]) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let out = significant_bits(&x).to_usize();
                self.graft(epoch, ptr, v, &[out.state(), x.state()])?;
            }
            LutSet([table, entry, inx]) => {
                let table = ExtAwi::opaque(self.get_bw(table));
                let entry = ExtAwi::opaque(self.get_bw(entry));
                let inx = ExtAwi::opaque(self.get_bw(inx));
                let out = lut_set(&table, &entry, &inx);
                self.graft(epoch, ptr, v, &[
                    out.state(),
                    table.state(),
                    entry.state(),
                    inx.state(),
                ])?;
            }
            ZeroResizeOverflow([x], w) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let mut out = ExtAwi::zero(bw(1));
                let w = w.get();
                if w < x.bw() {
                    out.bool_(!extawi!(x[w..]).unwrap().is_zero());
                }
                self.graft(epoch, ptr, v, &[out.state(), x.state()])?;
            }
            SignResizeOverflow([x], w) => {
                let x = ExtAwi::opaque(self.get_bw(x));
                let mut out = ExtAwi::zero(bw(1));
                let w = w.get();
                if w < x.bw() {
                    // the new msb and the bits above it should equal the old msb
                    let critical = extawi!(x[(w - 1)..]).unwrap();
                    let mut tmp = inlawi!(00);
                    tmp.set(0, critical.is_zero()).unwrap();
                    tmp.set(1, critical.is_umax()).unwrap();
                    out.lut_(&inlawi!(1001), &tmp).unwrap();
                }
                self.graft(epoch, ptr, v, &[out.state(), x.state()])?;
            }
            ArbMulAdd([add, lhs, rhs]) => {
                let w = self.get_bw(add);
                let add = ExtAwi::opaque(w);
                let lhs = ExtAwi::opaque(self.get_bw(lhs));
                let rhs = ExtAwi::opaque(self.get_bw(rhs));
                let out = mul_add(w, Some(&add), &lhs, &rhs);
                self.graft(epoch, ptr, v, &[
                    out.state(),
                    add.state(),
                    lhs.state(),
                    rhs.state(),
                ])?;
            }
            Mux([x0, x1, inx]) => {
                let x0 = ExtAwi::opaque(self.get_bw(x0));
                let x1 = ExtAwi::opaque(self.get_bw(x1));
                let inx_tmp = ExtAwi::opaque(self.get_bw(inx));
                let out = if self[inx].op.is_literal() {
                    let b = self.bool(inx)?;
                    if b {
                        x1.clone()
                    } else {
                        x0.clone()
                    }
                } else {
                    mux_(&x0, &x1, &inx_tmp)
                };
                self.graft(epoch, ptr, v, &[
                    out.state(),
                    x0.state(),
                    x1.state(),
                    inx_tmp.state(),
                ])?;
            }
            // TODO in the divisions especially and in other operations, we need to look at the
            // operand tree and combine multiple ops together in a single lowering operation
            UQuo([duo, div]) => {
                let duo = ExtAwi::opaque(self.get_bw(duo));
                let div = ExtAwi::opaque(self.get_bw(div));
                let quo = division(&duo, &div).0;
                self.graft(epoch, ptr, v, &[quo.state(), duo.state(), div.state()])?;
            }
            URem([duo, div]) => {
                let duo = ExtAwi::opaque(self.get_bw(duo));
                let div = ExtAwi::opaque(self.get_bw(div));
                let rem = division(&duo, &div).1;
                self.graft(epoch, ptr, v, &[rem.state(), duo.state(), div.state()])?;
            }
            IQuo([duo, div]) => {
                let duo = ExtAwi::opaque(self.get_bw(duo));
                let div = ExtAwi::opaque(self.get_bw(div));
                let duo_msb = duo.msb();
                let div_msb = div.msb();
                // keeping arguments opaque
                let mut tmp_duo = duo.clone();
                let mut tmp_div = div.clone();
                tmp_duo.neg_(duo_msb);
                tmp_div.neg_(div_msb);
                let mut quo = division(&tmp_duo, &tmp_div).0;
                let mut tmp0 = InlAwi::from(duo_msb);
                let tmp1 = InlAwi::from(div_msb);
                tmp0.xor_(&tmp1).unwrap();
                quo.neg_(tmp0.to_bool());
                self.graft(epoch, ptr, v, &[quo.state(), duo.state(), div.state()])?;
            }
            IRem([duo, div]) => {
                let duo = ExtAwi::opaque(self.get_bw(duo));
                let div = ExtAwi::opaque(self.get_bw(div));
                let duo_msb = duo.msb();
                let div_msb = div.msb();
                // keeping arguments opaque
                let mut tmp_duo = duo.clone();
                let mut tmp_div = div.clone();
                tmp_duo.neg_(duo_msb);
                tmp_div.neg_(div_msb);
                let mut rem = division(&tmp_duo, &tmp_div).1;
                rem.neg_(duo_msb);
                self.graft(epoch, ptr, v, &[rem.state(), duo.state(), div.state()])?;
            }
        }
        Ok(false)
    }

    /// Lowers all nodes in the tree of `leaf` not set to at least `tree_visit`.
    /// `tree_visit` should be one more than `lowered_visit`.
    pub fn lower_tree(
        &mut self,
        leaf: PNode,
        lowered_visit: u64,
        tree_visit: u64,
    ) -> Result<(), EvalError> {
        if self.a[leaf].visit >= tree_visit {
            return Ok(())
        }
        // We must use a DFS, one reason being that if downstream nodes are lowered
        // before upstream ones, we can easily end up in situations where a lower would
        // have been much simpler because of constants being propogated down.

        let mut unimplemented = false;
        let mut path: Vec<(usize, PNode, bool)> = vec![(0, leaf, true)];
        loop {
            let (i, p, all_literals) = path[path.len() - 1];
            let ops = self[p].op.operands();
            if ops.is_empty() {
                path.pop().unwrap();
                if path.is_empty() {
                    break
                }
                path.last_mut().unwrap().0 += 1;
                path.last_mut().unwrap().2 &= self[p].op.is_literal();
            } else if i >= ops.len() {
                // checked all sources
                if all_literals {
                    path.pop().unwrap();
                    match self.eval_node(p) {
                        Ok(()) => {
                            if path.is_empty() {
                                break
                            }
                        }
                        Err(EvalError::Unevaluatable) => {
                            if path.is_empty() {
                                break
                            }
                            path.last_mut().unwrap().2 = false;
                        }
                        Err(e) => {
                            self[p].err = Some(e.clone());
                            return Err(e)
                        }
                    }
                } else {
                    // newly lowered nodes will be set to `lowered_visit` which is less than
                    // `tree_visit` so that the DFS reexplores
                    let lowered = match self.lower_node(p, lowered_visit) {
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
                    if lowered {
                        path.pop().unwrap();
                        if path.is_empty() {
                            break
                        }
                    } else {
                        // else do not call `path.pop`, restart the DFS here
                        path.last_mut().unwrap().0 = 0;
                    }
                }
            } else {
                let mut p_next = ops[i];
                let next_visit = self[p_next].visit;
                if next_visit >= tree_visit {
                    while let Op::Copy([a]) = self[p_next].op {
                        // special optimization case: forward Copies
                        self[p].op.operands_mut()[i] = a;
                        self[a].rc = self[a].rc.checked_add(1).unwrap();
                        self.dec_rc(p_next).unwrap();
                        p_next = a;
                    }
                    // peek at node for evaluatableness but do not visit node
                    path.last_mut().unwrap().0 += 1;
                    path.last_mut().unwrap().2 &= self[p_next].op.is_literal();
                } else {
                    // prevent exponential growth
                    self[p_next].visit = tree_visit;
                    path.push((0, p_next, true));
                }
            }
        }

        if unimplemented {
            Err(EvalError::Unimplemented)
        } else {
            Ok(())
        }
    }

    /// Calls `lower_node` on everything, also evaluates everything. This also
    /// checks assertions, but not as strictly as `assert_assertions` which
    /// makes sure no assertions are unevaluated. Automatically calls
    /// `delete_unused_nodes` before and after the lowering.
    pub fn lower_all(&mut self) -> Result<(), EvalError> {
        // because I keep on forgetting this and end up with 64 bit `usize`s everywhere
        self.delete_unused_nodes();
        self.visit_gen += 1;
        let lowered_visit = self.visit_gen;
        self.visit_gen += 1;
        let tree_visit = self.visit_gen;
        let mut adv = self.a.advancer();
        while let Some(p) = adv.advance(&self.a) {
            self.lower_tree(p, lowered_visit, tree_visit)?;
        }
        self.assert_assertions_weak()?;
        self.unnote_true_assertions();
        self.delete_unused_nodes();
        Ok(())
    }
}
