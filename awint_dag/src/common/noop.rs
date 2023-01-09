use std::num::NonZeroUsize;

use awint_ext::{awi, awint_internals::BITS};
use Op::*;

use crate::{EvalError, Op};

/// No problems were found in regards to statically known values
///
/// `Operational,`
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
#[derive(Debug, Clone)]
pub enum NoopResult {
    Operational,
    Noop,
    Error(EvalError),
}

use NoopResult::*;

fn cbool(x: NonZeroUsize) -> Result<(), EvalError> {
    if x.get() == 1 {
        Ok(())
    } else {
        Err(EvalError::OtherStr(
            "a literal in an `Op<NonZeroUsize>` was not a boolean as expected",
        ))
    }
}

fn cusize(x: NonZeroUsize) -> Result<(), EvalError> {
    if x.get() == BITS {
        Ok(())
    } else {
        Err(EvalError::OtherStr(
            "a literal in an `Op<NonZeroUsize>` was not a usize as expected",
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
    ($expr:expr) => {
        #[cfg(debug_assertions)]
        {
            match cbool($expr) {
                Ok(()) => (),
                Err(e) => return NoopResult::Error(e),
            }
        }
    };
}

macro_rules! cusize {
    ($expr:expr) => {
        #[cfg(debug_assertions)]
        {
            match cusize($expr) {
                Ok(()) => (),
                Err(e) => return NoopResult::Error(e),
            }
        }
    };
}

// This is if there is redundancy that should be enforced to be equal on the
// crate side
macro_rules! ceq {
    ($x:expr, $y:expr) => {
        #[cfg(debug_assertions)]
        {
            match ceq($x, $y) {
                Ok(()) => (),
                Err(e) => return NoopResult::Error(e),
            }
        }
    };
}

impl Op<NonZeroUsize> {
    /// This is just for checking bitwidths and other statically known things.
    ///
    /// If `debug_assertions` are active, this also checks for correct
    /// crate-side compile time sizes
    pub fn noop_check(self, self_w: NonZeroUsize) -> NoopResult {
        let w = self_w;
        let res: bool = match self {
            Invalid => return Error(EvalError::Unevaluatable),
            Opaque(_) => return Error(EvalError::Unevaluatable),
            Literal(a) => {
                if w != a.nzbw() {
                    return Error(EvalError::OtherStr("`Literal` with mismatching bitwidths"))
                }
                true
            }
            StaticLut([a], lit) => {
                if a.get() < BITS {
                    if let awi::Some(lut_len) = (1usize << a.get()).checked_mul(w.get()) {
                        if lut_len == lit.bw() {
                            return NoopResult::Operational
                        }
                    }
                }
                return Error(EvalError::OtherStr("`StaticLut` with bad bitwidths"))
            }
            StaticGet([a], inx) => {
                cbool!(w);
                if inx < a.get() {
                    true
                } else {
                    return Error(EvalError::OtherStr("`StaticGet` with `inx` out of bounds"))
                }
            }
            StaticSet([a, b], inx) => {
                ceq!(w, a);
                cbool!(b);
                if inx >= a.get() {
                    return Error(EvalError::OtherStr("`StaticSet` with `inx` out of bounds"))
                }
                true
            }
            Resize([_, b]) => {
                cbool!(b);
                true
            }
            ZeroResize([_]) => true,
            SignResize([_]) => true,
            Copy([a]) => w == a,
            Lut([a, b]) => {
                let mut res = false;
                if b.get() < BITS {
                    if let awi::Some(lut_len) = (1usize << b.get()).checked_mul(w.get()) {
                        if lut_len == a.get() {
                            res = true;
                        }
                    }
                }
                res
            }
            Funnel([a, b]) => {
                (b.get() < (BITS - 1))
                    && ((1usize << b.get()) == w.get())
                    && ((w.get() << 1) == a.get())
            }
            CinSum([a, b, c]) => {
                cbool!(a);
                (w == b) && (w == c)
            }
            Not([a]) => w == a,
            Rev([a]) => w == a,
            Abs([a]) => w == a,
            IsZero([_]) => {
                cbool!(w);
                true
            }
            IsUmax([_]) => {
                cbool!(w);
                true
            }
            IsImax([_]) => {
                cbool!(w);
                true
            }
            IsImin([_]) => {
                cbool!(w);
                true
            }
            IsUone([_]) => {
                cbool!(w);
                true
            }
            Lsb([_]) => {
                cbool!(w);
                true
            }
            Msb([_]) => {
                cbool!(w);
                true
            }
            Lz([_]) => {
                cusize!(w);
                true
            }
            Tz([_]) => {
                cusize!(w);
                true
            }
            Sig([_]) => {
                cusize!(w);
                true
            }
            CountOnes([_]) => {
                cusize!(w);
                true
            }
            Or([a, b]) => (w == a) && (w == b),
            And([a, b]) => (w == a) && (w == b),
            Xor([a, b]) => (w == a) && (w == b),
            Shl([a, b]) => {
                cusize!(b);
                w == a
            }
            Lshr([a, b]) => {
                cusize!(b);
                w == a
            }
            Ashr([a, b]) => {
                cusize!(b);
                w == a
            }
            Rotl([a, b]) => {
                cusize!(b);
                w == a
            }
            Rotr([a, b]) => {
                cusize!(b);
                w == a
            }
            Add([a, b]) => (w == a) && (w == b),
            Sub([a, b]) => (w == a) && (w == b),
            Rsb([a, b]) => (w == a) && (w == b),
            Eq([a, b]) => {
                cbool!(w);
                a == b
            }
            Ne([a, b]) => {
                cbool!(w);
                a == b
            }
            Ult([a, b]) => {
                cbool!(w);
                a == b
            }
            Ule([a, b]) => {
                cbool!(w);
                a == b
            }
            Ilt([a, b]) => {
                cbool!(w);
                a == b
            }
            Ile([a, b]) => {
                cbool!(w);
                a == b
            }
            Inc([a, b]) => {
                ceq!(w, a);
                cbool!(b);
                true
            }
            Dec([a, b]) => {
                ceq!(w, a);
                cbool!(b);
                true
            }
            Neg([a, b]) => {
                ceq!(w, a);
                cbool!(b);
                true
            }
            ZeroResizeOverflow([_], _) => {
                cbool!(w);
                true
            }
            SignResizeOverflow([_], _) => {
                cbool!(w);
                true
            }
            Get([_, b]) => {
                cusize!(b);
                cbool!(w);
                true
            }
            Set([a, b, c]) => {
                ceq!(w, a);
                cusize!(b);
                cbool!(c);
                true
            }
            Mux([a, b, c]) => {
                ceq!(w, a);
                cbool!(c);
                a == b
            }
            LutSet([a, b, c]) => {
                let mut res = false;
                if c.get() < BITS {
                    if let Some(lut_len) = (1usize << c.get()).checked_mul(b.get()) {
                        if lut_len == a.get() {
                            res = w == a;
                        }
                    }
                }
                res
            }
            Field([a, b, _, d, e]) => {
                ceq!(w, a);
                cusize!(b);
                cusize!(d);
                cusize!(e);
                true
            }
            FieldTo([a, b, _, d]) => {
                ceq!(w, a);
                cusize!(b);
                cusize!(d);
                true
            }
            FieldFrom([a, _, c, d]) => {
                ceq!(w, a);
                cusize!(c);
                cusize!(d);
                true
            }
            FieldWidth([a, _, c]) => {
                ceq!(w, a);
                cusize!(c);
                true
            }
            FieldBit([a, b, _, d]) => {
                ceq!(w, a);
                cusize!(b);
                cusize!(d);
                true
            }
            MulAdd([a, _, _]) => {
                ceq!(w, a);
                true
            }
            UnsignedOverflow([a, b, c]) => {
                cbool!(w);
                cbool!(a);
                b == c
            }
            SignedOverflow([a, b, c]) => {
                cbool!(w);
                cbool!(a);
                b == c
            }
            IncCout([_, b]) => {
                cbool!(w);
                cbool!(b);
                true
            }
            DecCout([_, b]) => {
                cbool!(w);
                cbool!(b);
                true
            }
            UQuo([a, b]) => (w == a) && (w == b),
            URem([a, b]) => (w == a) && (w == b),
            IQuo([a, b]) => (w == a) && (w == b),
            IRem([a, b]) => (w == a) && (w == b),
        };
        if res {
            Operational
        } else {
            Noop
        }
    }
}
