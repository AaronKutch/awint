#![cfg_attr(not(debug_assertions), allow(unused_variables))]
#![cfg_attr(not(debug_assertions), allow(dead_code))]
#![allow(clippy::manual_map)]

use std::num::NonZeroUsize;

use awint_ext::{awint_internals::USIZE_BITS, Bits, ExtAwi};
use Op::*;

use crate::{EvalError, Op};

/// The result of an evaluation on an `Op<ExtAwi>`
///
/// In cases like `UQuo` where both invalid bitwidths and values at the same
/// time are possible, `Noop` takes precedence
#[derive(Debug, Clone)]
pub enum EvalResult {
    /// A Valid result
    Valid(ExtAwi),
    /// Pass-through, usually because of an Awi operation that can fail from
    /// out-of-bounds values
    Pass(ExtAwi),
    /// No-operation, usually because of Awi operations with invalid bitwidths
    Noop,
    /// Some evaluation error because of something that is not an Awi operation.
    /// This includes `Invalid`, `Opaque`, `Literal` with bitwidth mismatch, the
    /// static variants with bad inputs, and bad bitwidths on operations
    /// involving compile-time bitwidths (such as booleans and `usize`s in
    /// arguements)
    Error(EvalError),
}

use EvalResult::*;

fn cbool(x: &Bits) -> Result<bool, EvalError> {
    if x.bw() == 1 {
        Ok(x.to_bool())
    } else {
        Err(EvalError::OtherStr(
            "a literal in an `Op<ExtAwi>` was not a boolean as expected",
        ))
    }
}

fn cusize(x: &Bits) -> Result<usize, EvalError> {
    if x.bw() == USIZE_BITS {
        Ok(x.to_usize())
    } else {
        Err(EvalError::OtherStr(
            "a literal in an `Op<ExtAwi>` was not a usize as expected",
        ))
    }
}

fn ceq(x: NonZeroUsize, y: NonZeroUsize) -> Result<(), EvalError> {
    if x == y {
        Ok(())
    } else {
        Err(EvalError::OtherStr(
            "`self_w` in an `Op<NonZeroUsize>` was not as expected",
        ))
    }
}

macro_rules! cbool {
    ($expr:expr) => {{
        #[cfg(debug_assertions)]
        {
            match cbool(&$expr) {
                Ok(x) => x,
                Err(e) => return EvalResult::Error(e),
            }
        }
        #[cfg(not(debug_assertions))]
        {
            $expr.to_bool()
        }
    }};
}

macro_rules! cusize {
    ($expr:expr) => {{
        #[cfg(debug_assertions)]
        {
            match cusize(&$expr) {
                Ok(x) => x,
                Err(e) => return EvalResult::Error(e),
            }
        }
        #[cfg(not(debug_assertions))]
        {
            $expr.to_usize()
        }
    }};
}

// This is if there is redundancy that should be enforced to be equal on the
// crate side
macro_rules! ceq {
    ($x:expr, $y:expr) => {
        #[cfg(debug_assertions)]
        {
            match ceq($x, $y) {
                Ok(()) => (),
                Err(e) => return EvalResult::Error(e),
            }
        }
    };
}

impl Op<ExtAwi> {
    /// Evaluates the result of an `Op<ExtAwi>`
    pub fn eval(self, self_w: NonZeroUsize) -> EvalResult {
        let w = self_w;
        let res: Option<ExtAwi> = match self {
            Invalid => return Error(EvalError::Unevaluatable),
            Opaque(..) => return Error(EvalError::Unevaluatable),
            Literal(a) => {
                if w != a.nzbw() {
                    return Error(EvalError::OtherStr("`Literal` with mismatching bitwidths"))
                }
                Some(a)
            }
            StaticLut([a], lit) => {
                let mut r = ExtAwi::zero(w);
                if r.lut_(&lit, &a).is_some() {
                    Some(r)
                } else {
                    return Error(EvalError::OtherStr("`StaticLut` with bad bitwidths"))
                }
            }
            StaticGet([a], inx) => {
                if let Some(b) = a.get(inx) {
                    Some(ExtAwi::from_bool(b))
                } else {
                    return Error(EvalError::OtherStr("`StaticGet` with `inx` out of bounds"))
                }
            }
            StaticSet([mut a, b], inx) => {
                if a.set(inx, cbool!(b)).is_some() {
                    Some(a)
                } else {
                    return Error(EvalError::OtherStr("`StaticSet` with `inx` out of bounds"))
                }
            }
            Resize([a, b]) => {
                let mut r = ExtAwi::zero(w);
                r.resize_(&a, cbool!(b));
                Some(r)
            }
            ZeroResize([a]) => {
                let mut r = ExtAwi::zero(w);
                r.zero_resize_(&a);
                Some(r)
            }
            SignResize([a]) => {
                let mut r = ExtAwi::zero(w);
                r.sign_resize_(&a);
                Some(r)
            }
            Copy([a]) => Some(a),
            Lut([a, b]) => {
                let mut r = ExtAwi::zero(w);
                if r.lut_(&a, &b).is_some() {
                    Some(r)
                } else {
                    None
                }
            }
            Funnel([a, b]) => {
                let mut r = ExtAwi::zero(w);
                if r.funnel_(&a, &b).is_some() {
                    Some(r)
                } else {
                    None
                }
            }
            CinSum([a, b, c]) => {
                let mut r = ExtAwi::zero(w);
                if r.cin_sum_(cbool!(a), &b, &c).is_some() {
                    Some(r)
                } else {
                    None
                }
            }
            Not([mut a]) => {
                a.not_();
                Some(a)
            }
            Rev([mut a]) => {
                a.rev_();
                Some(a)
            }
            Abs([mut a]) => {
                a.abs_();
                Some(a)
            }
            IsZero([a]) => Some(ExtAwi::from_bool(a.is_zero())),
            IsUmax([a]) => Some(ExtAwi::from_bool(a.is_umax())),
            IsImax([a]) => Some(ExtAwi::from_bool(a.is_imax())),
            IsImin([a]) => Some(ExtAwi::from_bool(a.is_imin())),
            IsUone([a]) => Some(ExtAwi::from_bool(a.is_uone())),
            Lsb([a]) => Some(ExtAwi::from_bool(a.lsb())),
            Msb([a]) => Some(ExtAwi::from_bool(a.msb())),
            Lz([a]) => Some(ExtAwi::from_usize(a.lz())),
            Tz([a]) => Some(ExtAwi::from_usize(a.tz())),
            Sig([a]) => Some(ExtAwi::from_usize(a.sig())),
            CountOnes([a]) => Some(ExtAwi::from_usize(a.count_ones())),
            Or([mut a, b]) => {
                if a.or_(&b).is_some() {
                    Some(a)
                } else {
                    None
                }
            }
            And([mut a, b]) => {
                if a.and_(&b).is_some() {
                    Some(a)
                } else {
                    None
                }
            }
            Xor([mut a, b]) => {
                if a.xor_(&b).is_some() {
                    Some(a)
                } else {
                    None
                }
            }
            Shl([mut a, b]) => {
                if a.shl_(cusize!(b)).is_some() {
                    Some(a)
                } else {
                    ceq!(w, a.nzbw());
                    return Pass(a)
                }
            }
            Lshr([mut a, b]) => {
                if a.lshr_(cusize!(b)).is_some() {
                    Some(a)
                } else {
                    ceq!(w, a.nzbw());
                    return Pass(a)
                }
            }
            Ashr([mut a, b]) => {
                if a.ashr_(cusize!(b)).is_some() {
                    Some(a)
                } else {
                    ceq!(w, a.nzbw());
                    return Pass(a)
                }
            }
            Rotl([mut a, b]) => {
                if a.rotl_(cusize!(b)).is_some() {
                    Some(a)
                } else {
                    ceq!(w, a.nzbw());
                    return Pass(a)
                }
            }
            Rotr([mut a, b]) => {
                if a.rotr_(cusize!(b)).is_some() {
                    Some(a)
                } else {
                    ceq!(w, a.nzbw());
                    return Pass(a)
                }
            }
            Add([mut a, b]) => {
                if a.add_(&b).is_some() {
                    Some(a)
                } else {
                    None
                }
            }
            Sub([mut a, b]) => {
                if a.sub_(&b).is_some() {
                    Some(a)
                } else {
                    None
                }
            }
            Rsb([mut a, b]) => {
                if a.rsb_(&b).is_some() {
                    Some(a)
                } else {
                    None
                }
            }
            Eq([a, b]) => {
                if let Some(b) = a.const_eq(&b) {
                    Some(ExtAwi::from_bool(b))
                } else {
                    None
                }
            }
            Ne([a, b]) => {
                if let Some(b) = a.const_ne(&b) {
                    Some(ExtAwi::from_bool(b))
                } else {
                    None
                }
            }
            Ult([a, b]) => {
                if let Some(b) = a.ult(&b) {
                    Some(ExtAwi::from_bool(b))
                } else {
                    None
                }
            }
            Ule([a, b]) => {
                if let Some(b) = a.ule(&b) {
                    Some(ExtAwi::from_bool(b))
                } else {
                    None
                }
            }
            Ilt([a, b]) => {
                if let Some(b) = a.ilt(&b) {
                    Some(ExtAwi::from_bool(b))
                } else {
                    None
                }
            }
            Ile([a, b]) => {
                if let Some(b) = a.ile(&b) {
                    Some(ExtAwi::from_bool(b))
                } else {
                    None
                }
            }
            Inc([mut a, b]) => {
                a.inc_(cbool!(b));
                Some(a)
            }
            Dec([mut a, b]) => {
                a.dec_(cbool!(b));
                Some(a)
            }
            Neg([mut a, b]) => {
                a.neg_(cbool!(b));
                Some(a)
            }
            ZeroResizeOverflow([a], w) => {
                let mut tmp_awi = ExtAwi::zero(w);
                Some(ExtAwi::from_bool(tmp_awi.zero_resize_(&a)))
            }
            SignResizeOverflow([a], w) => {
                let mut tmp_awi = ExtAwi::zero(w);
                Some(ExtAwi::from_bool(tmp_awi.sign_resize_(&a)))
            }
            Get([a, b]) => {
                if let Some(b) = a.get(cusize!(b)) {
                    Some(ExtAwi::from_bool(b))
                } else {
                    return Pass(a)
                }
            }
            Set([mut a, b, c]) => {
                if a.set(cusize!(b), cbool!(c)).is_some() {
                    Some(a)
                } else {
                    ceq!(w, a.nzbw());
                    return Pass(a)
                }
            }
            Mux([mut a, b, c]) => {
                if a.mux_(&b, cbool!(c)).is_some() {
                    Some(a)
                } else {
                    None
                }
            }
            LutSet([mut a, b, c]) => {
                if a.lut_set(&b, &c).is_some() {
                    Some(a)
                } else {
                    None
                }
            }
            Field([mut a, b, c, d, e]) => {
                if a.field(cusize!(b), &c, cusize!(d), cusize!(e)).is_some() {
                    Some(a)
                } else {
                    ceq!(w, a.nzbw());
                    return Pass(a)
                }
            }
            FieldTo([mut a, b, c, d]) => {
                if a.field_to(cusize!(b), &c, cusize!(d)).is_some() {
                    Some(a)
                } else {
                    ceq!(w, a.nzbw());
                    return Pass(a)
                }
            }
            FieldFrom([mut a, b, c, d]) => {
                if a.field_from(&b, cusize!(c), cusize!(d)).is_some() {
                    Some(a)
                } else {
                    ceq!(w, a.nzbw());
                    return Pass(a)
                }
            }
            FieldWidth([mut a, b, c]) => {
                if a.field_width(&b, cusize!(c)).is_some() {
                    Some(a)
                } else {
                    ceq!(w, a.nzbw());
                    return Pass(a)
                }
            }
            FieldBit([mut a, b, c, d]) => {
                if a.field_bit(cusize!(b), &c, cusize!(d)).is_some() {
                    Some(a)
                } else {
                    ceq!(w, a.nzbw());
                    return Pass(a)
                }
            }
            ArbMulAdd([mut a, b, c]) => {
                a.arb_umul_add_(&b, &c);
                Some(a)
            }
            UnsignedOverflow([a, b, c]) => {
                // note that `self_w` and `self.get_bw(a)` are both 1
                let mut t = ExtAwi::zero(b.nzbw());
                if let Some((o, _)) = t.cin_sum_(cbool!(a), &b, &c) {
                    Some(ExtAwi::from_bool(o))
                } else {
                    None
                }
            }
            SignedOverflow([a, b, c]) => {
                let mut t = ExtAwi::zero(b.nzbw());
                if let Some((_, o)) = t.cin_sum_(cbool!(a), &b, &c) {
                    Some(ExtAwi::from_bool(o))
                } else {
                    None
                }
            }
            IncCout([mut a, b]) => Some(ExtAwi::from_bool(a.inc_(cbool!(b)))),
            DecCout([mut a, b]) => Some(ExtAwi::from_bool(a.dec_(cbool!(b)))),
            UQuo([a, b]) => {
                // Noop needs to take precedence
                if (w.get() != a.bw()) || (w.get() != b.bw()) {
                    None
                } else if b.is_zero() {
                    return Pass(a)
                } else {
                    let mut r = ExtAwi::zero(w);
                    let mut t = ExtAwi::zero(w);
                    Bits::udivide(&mut r, &mut t, &a, &b).unwrap();
                    Some(r)
                }
            }
            URem([a, b]) => {
                if (w.get() != a.bw()) || (w.get() != b.bw()) {
                    None
                } else if b.is_zero() {
                    return Pass(a)
                } else {
                    let mut r = ExtAwi::zero(w);
                    let mut t = ExtAwi::zero(w);
                    Bits::udivide(&mut t, &mut r, &a, &b).unwrap();
                    Some(r)
                }
            }
            IQuo([mut a, mut b]) => {
                if (w.get() != a.bw()) || (w.get() != b.bw()) {
                    None
                } else if b.is_zero() {
                    return Pass(a)
                } else {
                    let mut r = ExtAwi::zero(w);
                    let mut t = ExtAwi::zero(w);
                    Bits::idivide(&mut r, &mut t, &mut a, &mut b).unwrap();
                    Some(r)
                }
            }
            IRem([mut a, mut b]) => {
                if (w.get() != a.bw()) || (w.get() != b.bw()) {
                    None
                } else if b.is_zero() {
                    return Pass(a)
                } else {
                    let mut r = ExtAwi::zero(w);
                    let mut t = ExtAwi::zero(w);
                    Bits::idivide(&mut t, &mut r, &mut a, &mut b).unwrap();
                    Some(r)
                }
            }
        };
        if let Some(r) = res {
            ceq!(w, r.nzbw());
            Valid(r)
        } else {
            Noop
        }
    }
}
