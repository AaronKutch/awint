use std::num::NonZeroUsize;

use awint_ext::{awint_internals::BITS, Bits, ExtAwi};
use Op::*;

use crate::{EvalError, Op};

/// A Valid result
///
/// `Valid(ExtAwi),`
///
/// Pass-through, usually because of an Awi operation that can fail from
/// out-of-bounds values
///
/// `Pass(ExtAwi),`
///
/// No-operation, usually because of Awi operations with invalid bitwidths
///
/// `Noop,`
///
/// Some evaluation error because of something that is not an Awi operation.
/// This includes `Invalid`, `Opaque`, `Literal` with bitwidth mismatch, the
/// static variants with bad inputs, and bad bitwidths on operations
/// involving compile-time bitwidths (such as booleans and `usize`s in
/// arguements)
///
/// `Error(EvalError),`
///
/// In cases like `UQuo` where both invalid bitwidths and values at the same
/// time are possible, `Noop` takes precedence
#[derive(Debug, Clone)]
pub enum EvalResult {
    Valid(ExtAwi),
    Pass(ExtAwi),
    Noop,
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
    if x.bw() == BITS {
        Ok(x.to_usize())
    } else {
        Err(EvalError::OtherStr(
            "a literal in an `Op<ExtAwi>` was not a usize as expected",
        ))
    }
}

macro_rules! cbool {
    ($expr:expr) => {
        match cbool(&$expr) {
            Ok(x) => x,
            Err(e) => return EvalResult::Error(e),
        }
    };
}

macro_rules! cusize {
    ($expr:expr) => {
        match cusize(&$expr) {
            Ok(x) => x,
            Err(e) => return EvalResult::Error(e),
        }
    };
}

impl Op<ExtAwi> {
    pub fn eval(self, self_w: NonZeroUsize) -> EvalResult {
        let mut r = ExtAwi::zero(self_w);
        let option = match self {
            Invalid => return Error(EvalError::Unevaluatable),
            Opaque(_) => return Error(EvalError::Unevaluatable),
            Literal(a) => {
                if r.copy_(&a).is_none() {
                    return Error(EvalError::OtherStr("`Literal` with mismatching bitwidths"))
                }
                Some(())
            }
            StaticLut([a], lit) => {
                if r.lut_(&lit, &a).is_some() {
                    Some(())
                } else {
                    return Error(EvalError::OtherStr("`StaticLut` with bad bitwidths"))
                }
            }
            StaticGet([a], inx) => {
                if let Some(b) = a.get(inx) {
                    r.bool_(b);
                    Some(())
                } else {
                    return Error(EvalError::OtherStr("`StaticGet` with `inx` out of bounds"))
                }
            }
            StaticSet([a, b], inx) => {
                if r.copy_(&a).is_some() {
                    r.set(inx, cbool!(b))
                } else {
                    return Error(EvalError::OtherStr("`StaticSet` with `inx` out of bounds"))
                }
            }
            Resize([a, b]) => {
                r.resize_(&a, cbool!(b));
                Some(())
            }
            ZeroResize([a]) => {
                r.zero_resize_(&a);
                Some(())
            }
            SignResize([a]) => {
                r.sign_resize_(&a);
                Some(())
            }
            Copy([a]) => r.copy_(&a),
            Lut([a, b]) => r.lut_(&a, &b),
            Funnel([a, b]) => r.funnel_(&a, &b),
            CinSum([a, b, c]) => {
                if r.cin_sum_(cbool!(a), &b, &c).is_some() {
                    Some(())
                } else {
                    None
                }
            }
            Not([a]) => {
                let e = r.copy_(&a);
                r.not_();
                e
            }
            Rev([a]) => {
                let e = r.copy_(&a);
                r.rev_();
                e
            }
            Abs([a]) => {
                let e = r.copy_(&a);
                r.abs_();
                e
            }
            IsZero([a]) => {
                r.bool_(a.is_zero());
                Some(())
            }
            IsUmax([a]) => {
                r.bool_(a.is_umax());
                Some(())
            }
            IsImax([a]) => {
                r.bool_(a.is_imax());
                Some(())
            }
            IsImin([a]) => {
                r.bool_(a.is_imin());
                Some(())
            }
            IsUone([a]) => {
                r.bool_(a.is_uone());
                Some(())
            }
            Lsb([a]) => {
                r.bool_(a.lsb());
                Some(())
            }
            Msb([a]) => {
                r.bool_(a.msb());
                Some(())
            }
            Lz([a]) => {
                r.usize_(a.lz());
                Some(())
            }
            Tz([a]) => {
                r.usize_(a.tz());
                Some(())
            }
            Sig([a]) => {
                r.usize_(a.sig());
                Some(())
            }
            CountOnes([a]) => {
                r.usize_(a.count_ones());
                Some(())
            }
            Or([a, b]) => {
                if r.copy_(&a).is_some() {
                    r.or_(&b)
                } else {
                    None
                }
            }
            And([a, b]) => {
                if r.copy_(&a).is_some() {
                    r.and_(&b)
                } else {
                    None
                }
            }
            Xor([a, b]) => {
                if r.copy_(&a).is_some() {
                    r.xor_(&b)
                } else {
                    None
                }
            }
            Shl([a, b]) => {
                if r.copy_(&a).is_some() {
                    if r.shl_(cusize!(b)).is_some() {
                        Some(())
                    } else {
                        return Pass(r)
                    }
                } else {
                    None
                }
            }
            Lshr([a, b]) => {
                if r.copy_(&a).is_some() {
                    if r.lshr_(cusize!(b)).is_some() {
                        Some(())
                    } else {
                        return Pass(r)
                    }
                } else {
                    None
                }
            }
            Ashr([a, b]) => {
                if r.copy_(&a).is_some() {
                    if r.ashr_(cusize!(b)).is_some() {
                        Some(())
                    } else {
                        return Pass(r)
                    }
                } else {
                    None
                }
            }
            Rotl([a, b]) => {
                if r.copy_(&a).is_some() {
                    if r.rotl_(cusize!(b)).is_some() {
                        Some(())
                    } else {
                        return Pass(r)
                    }
                } else {
                    None
                }
            }
            Rotr([a, b]) => {
                if r.copy_(&a).is_some() {
                    if r.rotr_(cusize!(b)).is_some() {
                        Some(())
                    } else {
                        return Pass(r)
                    }
                } else {
                    None
                }
            }
            Add([a, b]) => {
                if r.copy_(&a).is_some() {
                    r.add_(&b)
                } else {
                    None
                }
            }
            Sub([a, b]) => {
                if r.copy_(&a).is_some() {
                    r.sub_(&b)
                } else {
                    None
                }
            }
            Rsb([a, b]) => {
                if r.copy_(&a).is_some() {
                    r.rsb_(&b)
                } else {
                    None
                }
            }
            Eq([a, b]) => {
                if let Some(b) = a.const_eq(&b) {
                    r.bool_(b);
                    Some(())
                } else {
                    None
                }
            }
            Ne([a, b]) => {
                if let Some(b) = a.const_ne(&b) {
                    r.bool_(b);
                    Some(())
                } else {
                    None
                }
            }
            Ult([a, b]) => {
                if let Some(b) = a.ult(&b) {
                    r.bool_(b);
                    Some(())
                } else {
                    None
                }
            }
            Ule([a, b]) => {
                if let Some(b) = a.ule(&b) {
                    r.bool_(b);
                    Some(())
                } else {
                    None
                }
            }
            Ilt([a, b]) => {
                if let Some(b) = a.ilt(&b) {
                    r.bool_(b);
                    Some(())
                } else {
                    None
                }
            }
            Ile([a, b]) => {
                if let Some(b) = a.ile(&b) {
                    r.bool_(b);
                    Some(())
                } else {
                    None
                }
            }
            Inc([a, b]) => {
                if r.copy_(&a).is_some() {
                    r.inc_(cbool!(b));
                    Some(())
                } else {
                    None
                }
            }
            Dec([a, b]) => {
                if r.copy_(&a).is_some() {
                    r.dec_(cbool!(b));
                    Some(())
                } else {
                    None
                }
            }
            Neg([a, b]) => {
                let e = r.copy_(&a);
                r.neg_(cbool!(b));
                e
            }
            ZeroResizeOverflow([a], w) => {
                let mut tmp_awi = ExtAwi::zero(w);
                r.bool_(tmp_awi.zero_resize_(&a));
                Some(())
            }
            SignResizeOverflow([a], w) => {
                let mut tmp_awi = ExtAwi::zero(w);
                r.bool_(tmp_awi.sign_resize_(&a));
                Some(())
            }
            Get([a, b]) => {
                if let Some(b) = a.get(cusize!(b)) {
                    r.bool_(b);
                    Some(())
                } else {
                    return Pass(a)
                }
            }
            Set([a, b, c]) => {
                if r.copy_(&a).is_some() {
                    if r.set(cusize!(b), cbool!(c)).is_some() {
                        Some(())
                    } else {
                        return Pass(r)
                    }
                } else {
                    None
                }
            }
            Mux([a, b, c]) => {
                if r.copy_(&a).is_some() {
                    r.mux_(&b, cbool!(c))
                } else {
                    None
                }
            }
            LutSet([a, b, c]) => {
                if r.copy_(&a).is_some() {
                    r.lut_set(&b, &c)
                } else {
                    None
                }
            }
            Field([a, b, c, d, e]) => {
                if r.copy_(&a).is_some() {
                    if r.field(cusize!(b), &c, cusize!(d), cusize!(e)).is_some() {
                        Some(())
                    } else {
                        return Pass(r)
                    }
                } else {
                    None
                }
            }
            FieldTo([a, b, c, d]) => {
                if r.copy_(&a).is_some() {
                    if r.field_to(cusize!(b), &c, cusize!(d)).is_some() {
                        Some(())
                    } else {
                        return Pass(r)
                    }
                } else {
                    None
                }
            }
            FieldFrom([a, b, c, d]) => {
                if r.copy_(&a).is_some() {
                    if r.field_from(&b, cusize!(c), cusize!(d)).is_some() {
                        Some(())
                    } else {
                        return Pass(r)
                    }
                } else {
                    None
                }
            }
            FieldWidth([a, b, c]) => {
                if r.copy_(&a).is_some() {
                    if r.field_width(&b, cusize!(c)).is_some() {
                        Some(())
                    } else {
                        return Pass(r)
                    }
                } else {
                    None
                }
            }
            FieldBit([a, b, c, d]) => {
                if r.copy_(&a).is_some() {
                    if r.field_bit(cusize!(b), &c, cusize!(d)).is_some() {
                        Some(())
                    } else {
                        return Pass(r)
                    }
                } else {
                    None
                }
            }
            MulAdd([a, b, c]) => {
                if r.copy_(&a).is_some() {
                    r.arb_umul_add_(&b, &c);
                    Some(())
                } else {
                    None
                }
            }
            UnsignedOverflow([a, b, c]) => {
                // note that `self_w` and `self.get_bw(a)` are both 1
                let mut t = ExtAwi::zero(b.nzbw());
                if let Some((o, _)) = t.cin_sum_(cbool!(a), &b, &c) {
                    r.bool_(o);
                    Some(())
                } else {
                    None
                }
            }
            SignedOverflow([a, b, c]) => {
                let mut t = ExtAwi::zero(b.nzbw());
                if let Some((_, o)) = t.cin_sum_(cbool!(a), &b, &c) {
                    r.bool_(o);
                    Some(())
                } else {
                    None
                }
            }
            IncCout([a, b]) => {
                let mut t = ExtAwi::zero(a.nzbw());
                if t.copy_(&a).is_some() {
                    r.bool_(t.inc_(cbool!(b)));
                    Some(())
                } else {
                    None
                }
            }
            DecCout([a, b]) => {
                let mut t = ExtAwi::zero(a.nzbw());
                if t.copy_(&a).is_some() {
                    r.bool_(t.dec_(cbool!(b)));
                    Some(())
                } else {
                    None
                }
            }
            UQuo([a, b]) => {
                // Noop needs to take precedence
                if (r.bw() != a.bw()) || (r.bw() != b.bw()) {
                    None
                } else if b.is_zero() {
                    return Pass(a)
                } else {
                    let mut t = ExtAwi::zero(self_w);
                    Bits::udivide(&mut r, &mut t, &a, &b).unwrap();
                    Some(())
                }
            }
            URem([a, b]) => {
                if (r.bw() != a.bw()) || (r.bw() != b.bw()) {
                    None
                } else if b.is_zero() {
                    return Pass(a)
                } else {
                    let mut t = ExtAwi::zero(self_w);
                    Bits::udivide(&mut t, &mut r, &a, &b).unwrap();
                    Some(())
                }
            }
            IQuo([mut a, mut b]) => {
                if (r.bw() != a.bw()) || (r.bw() != b.bw()) {
                    None
                } else if b.is_zero() {
                    return Pass(a)
                } else {
                    let mut t = ExtAwi::zero(self_w);
                    Bits::idivide(&mut r, &mut t, &mut a, &mut b).unwrap();
                    Some(())
                }
            }
            IRem([mut a, mut b]) => {
                if (r.bw() != a.bw()) || (r.bw() != b.bw()) {
                    None
                } else if b.is_zero() {
                    return Pass(a)
                } else {
                    let mut t = ExtAwi::zero(self_w);
                    Bits::idivide(&mut t, &mut r, &mut a, &mut b).unwrap();
                    Some(())
                }
            }
        };
        if option.is_some() {
            Valid(r)
        } else {
            Noop
        }
    }
}
