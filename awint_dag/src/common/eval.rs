#![cfg_attr(not(debug_assertions), allow(unused_variables))]
#![cfg_attr(not(debug_assertions), allow(dead_code))]
#![allow(clippy::manual_map)]

use std::num::NonZeroUsize;

use awint_ext::{awint_internals::USIZE_BITS, Awi, Bits};
use awint_macros::cc;
use Op::*;

use crate::{DummyDefault, Op};

/// The result of an evaluation on an `Op<Awi>`
///
/// In cases like `UQuo` where both invalid bitwidths and values at the same
/// time are possible, `Noop` takes precedence
#[derive(Debug, Clone)]
pub enum EvalResult {
    /// A Valid result
    Valid(Awi),
    /// Pass-through of the first argument, usually because of an Awi operation
    /// that can fail from out-of-bounds values
    Pass(Awi),
    /// Pass-through but it is dependent on a value that is unknown
    PassUnevaluatable,
    /// No-operation, usually because of Awi operations with invalid bitwidths
    Noop,
    /// Unevaluatable because some inputs have unknown bitpatterns
    Unevaluatable,
    /// An `Op::Assert` evaluated to be successful
    AssertionSuccess,
    /// An `Op::Assert` evaluated to be a failure
    AssertionFailure,
    /// Some evaluation error because of something that is not an Awi operation.
    /// This includes `Invalid`, `Opaque`, `Literal` with bitwidth mismatch, the
    /// static variants with bad inputs, and bad bitwidths on operations
    /// involving compile-time bitwidths (such as booleans and `usize`s in
    /// arguements)
    Error(&'static str),
}

use EvalResult::*;

/// This struct is just used for the `eval` function.
///
/// In earlier versions we implemented `eval` for `Op<Awi>`, but there were
/// cases where only some inputs were unknown and something could still be
/// inferred from the partially known values or bitwidths.
#[derive(Debug, Clone)]
pub enum EAwi {
    /// The whole bit pattern is known
    KnownAwi(Awi),
    /// Only the bitwidth is known
    Bitwidth(NonZeroUsize),
}

impl DummyDefault for EAwi {
    fn default() -> Self {
        Self::Bitwidth(NonZeroUsize::new(1).unwrap())
    }
}

impl EAwi {
    pub fn nzbw(&self) -> NonZeroUsize {
        match self {
            EAwi::KnownAwi(awi) => awi.nzbw(),
            EAwi::Bitwidth(w) => *w,
        }
    }

    pub fn bw(&self) -> usize {
        self.nzbw().get()
    }
}

macro_rules! cases {
    ($a_init:ident, $a:ident => $known:block, $a_w:ident => $bitwidth:block,) => {
        match $a_init {
            #[allow(unused_mut)]
            EAwi::KnownAwi(mut $a) => $known,
            EAwi::Bitwidth($a_w) => $bitwidth,
        }
    };
}

macro_rules! awi1 {
    ($eawi0:ident, $block:block) => {
        #[allow(unused_mut)]
        if let EAwi::KnownAwi(mut $eawi0) = $eawi0 {
            $block
        } else {
            Unevaluatable
        }
    };
}

macro_rules! awi2 {
    ($eawi0:ident, $eawi1:ident, $block:block) => {
        #[allow(unused_mut)]
        if let (EAwi::KnownAwi(mut $eawi0), EAwi::KnownAwi(mut $eawi1)) = ($eawi0, $eawi1) {
            $block
        } else {
            Unevaluatable
        }
    };
}

macro_rules! awi3 {
    ($eawi0:ident, $eawi1:ident, $eawi2:ident, $block:block) => {
        #[allow(unused_mut)]
        if let (
            EAwi::KnownAwi(mut $eawi0),
            EAwi::KnownAwi(mut $eawi1),
            EAwi::KnownAwi(mut $eawi2),
        ) = ($eawi0, $eawi1, $eawi2)
        {
            $block
        } else {
            Unevaluatable
        }
    };
}

// typechecking
#[inline]
fn ceq(x: NonZeroUsize, y: NonZeroUsize) -> bool {
    x == y
}

macro_rules! ceq {
    ($x:expr, $y:expr) => {
        if !ceq($x, $y) {
            return EvalResult::Noop
        }
    };
}

const BUG_MESSAGE: &str =
    "a mimicking type bitwidth invariant was broken, this is a bug with `awint_dag`";

// The mimicking types should set the self width to be always equal sometimes,
// it indicates a bug and not just a `Noop` condition if they are unequal. This
// will check if debug assertions are on.
macro_rules! ceq_strict {
    ($x:expr, $y:expr) => {
        #[cfg(debug_assertions)]
        {
            if !ceq($x, $y) {
                return EvalResult::Error(BUG_MESSAGE)
            }
        }
    };
}

// these are all strict

// mainly for typechecking
#[inline]
fn cbool(x: &Bits) -> Option<bool> {
    if x.bw() == 1 {
        Some(x.to_bool())
    } else {
        None
    }
}

macro_rules! cbool {
    ($expr:expr) => {{
        #[cfg(debug_assertions)]
        {
            if let Some(b) = cbool($expr) {
                b
            } else {
                return EvalResult::Error(BUG_MESSAGE)
            }
        }
        #[cfg(not(debug_assertions))]
        {
            $expr.to_bool()
        }
    }};
}

#[inline]
fn cusize(x: &Bits) -> Option<usize> {
    if x.bw() == USIZE_BITS {
        Some(x.to_usize())
    } else {
        None
    }
}

macro_rules! cusize {
    ($expr:expr) => {{
        #[cfg(debug_assertions)]
        {
            if let Some(b) = cusize(&$expr) {
                b
            } else {
                return EvalResult::Error(BUG_MESSAGE)
            }
        }
        #[cfg(not(debug_assertions))]
        {
            $expr.to_usize()
        }
    }};
}

macro_rules! cbool_w {
    ($expr:expr) => {{
        #[cfg(debug_assertions)]
        {
            if $expr.get() != 1 {
                return EvalResult::Error(BUG_MESSAGE)
            }
        }
    }};
}

macro_rules! cusize_w {
    ($expr:expr) => {{
        #[cfg(debug_assertions)]
        {
            if $expr.get() != USIZE_BITS {
                return EvalResult::Error(BUG_MESSAGE)
            }
        }
    }};
}

macro_rules! shift {
    ($w:ident, $a:ident, $b:ident, $shift_fn:ident) => {{
        ceq_strict!($w, $a.nzbw());
        cases!($b,
            $b => {
                let b = cusize!($b);
                cases!($a,
                    a => {
                        if a.$shift_fn(b).is_some() {
                            Valid(a)
                        } else {
                            Pass(a)
                        }
                    },
                    _a_w => {
                        if b >= $w.get() {
                            PassUnevaluatable
                        } else {
                            Unevaluatable
                        }
                    },
                )
            },
            b_w => {
                cusize_w!(b_w);
                Unevaluatable
            },
        )
    }}
}

macro_rules! unary_bit {
    ($w:ident, $a:ident, $unary_fn:ident) => {{
        cbool_w!($w);
        awi1!($a, { Valid(Awi::from_bool($a.$unary_fn())) })
    }};
}

macro_rules! unary_usize {
    ($w:ident, $a:ident, $unary_fn:ident) => {{
        cusize_w!($w);
        awi1!($a, { Valid(Awi::from_usize($a.$unary_fn())) })
    }};
}

macro_rules! binary {
    ($w:ident, $a:ident, $b:ident, $binary_fn:ident) => {{
        ceq_strict!($w, $a.nzbw());
        ceq!($w, $b.nzbw());
        awi2!($a, $b, {
            $a.$binary_fn(&$b).unwrap();
            Valid($a)
        })
    }}
}

macro_rules! cmp {
    ($w:ident, $a:ident, $b:ident, $cmp_fn:ident) => {{
        cbool_w!($w);
        ceq!($a.nzbw(), $b.nzbw());
        awi2!($a, $b, {
            Valid(Awi::from_bool($a.$cmp_fn(&$b).unwrap()))
        })
    }}
}

macro_rules! range_op {
    ($w:ident, $a:ident, $b:ident, $c:ident, $range_op_fn:ident) => {{
        ceq_strict!($w, $a.nzbw());
        // need to check separately incase the `awi3` fails but some are constants
        if let EAwi::KnownAwi(ref b) = $b {
            let b = cusize!(&b);
            if b > $a.bw() {
                return Noop
            }
        }
        if let EAwi::KnownAwi(ref c) = $c {
            let c = cusize!(&c);
            if c > $a.bw() {
                return Noop
            }
        }
        awi3!($a, $b, $c, {
            let b = cusize!($b);
            let c = cusize!($c);
            if $a.$range_op_fn(b..c).is_some() {
                Valid($a)
            } else {
                Noop
            }
        })
    }}
}

impl Op<EAwi> {
    /// Evaluates the result of an `Op<Awi>`
    pub fn eval(self, self_w: NonZeroUsize) -> EvalResult {
        let w = self_w;
        match self {
            Invalid => Unevaluatable,
            Opaque(..) => Unevaluatable,
            Literal(a) => {
                ceq_strict!(w, a.nzbw());
                Valid(a)
            }
            Assert([a]) => {
                // more manual because it is more likely that there will be issues involving
                // `Assert`s
                cases!(a,
                    a => {
                        if a.bw() != 1 {
                            Error("`Assert` with bad bitwidths")
                        } else if a.to_bool() {
                            AssertionSuccess
                        } else {
                            AssertionFailure
                        }
                    },
                    a_w => {
                        if a_w.get() != 1 {
                            Error("`Assert` with bad bitwidths")
                        } else {
                            Unevaluatable
                        }
                    },
                )
            }
            Repeat([a]) => {
                cases!(a,
                    a => {
                        if a.bw() == 1 {
                            if a.to_bool() {
                                Valid(Awi::umax(w))
                            } else {
                                Valid(Awi::zero(w))
                            }
                        } else {
                            let mut r = Awi::zero(w);
                            let mut to = 0;
                            loop {
                                if to >= w.get() {
                                    break
                                }
                                cc!(
                                    .., a;
                                    .., r[to..];
                                ).unwrap();
                            }
                            Valid(r)
                        }
                    },
                    _a_w => {
                        Unevaluatable
                    },
                )
            }
            StaticGet([a], inx) => {
                cbool_w!(w);
                cases!(a,
                    a => {
                        if let Some(b) = a.get(inx) {
                            Valid(Awi::from_bool(b))
                        } else {
                            Error("`StaticGet` with `inx` out of bounds")
                        }
                    },
                    a_w => {
                        if inx < a_w.get() {
                            Unevaluatable
                        } else {
                            Error("`StaticGet` with `inx` out of bounds")
                        }
                    },
                )
            }
            Concat(concat) => {
                let mut total_width = 0usize;
                let mut known = true;
                for t in concat.as_slice() {
                    match t {
                        EAwi::KnownAwi(t) => {
                            total_width = total_width.checked_add(t.bw()).unwrap();
                        }
                        EAwi::Bitwidth(t_w) => {
                            total_width = total_width.checked_add(t_w.get()).unwrap();
                            known = false;
                        }
                    }
                }
                // should be impossible to be zero because the constructors require nonzero len
                // and bitwidth
                let total_width = NonZeroUsize::new(total_width).unwrap();
                if w != total_width {
                    return Error("`Concat` with bad concatenation");
                }
                if known {
                    let mut r = Awi::zero(total_width);
                    let mut to = 0;
                    for t in concat.as_slice() {
                        match t {
                            EAwi::KnownAwi(t) => {
                                let width = t.bw();
                                r.field_to(to, t, width).unwrap();
                                to += width;
                            }
                            EAwi::Bitwidth(_t_w) => {
                                unreachable!()
                            }
                        }
                    }
                    Valid(r)
                } else {
                    Unevaluatable
                }
            }
            ConcatFields(concat) => {
                let mut total_width = 0usize;
                let mut known = true;
                for (t, (from, width)) in concat.iter() {
                    match t {
                        EAwi::KnownAwi(t) => {
                            if (*width > t.nzbw()) || (*from > (t.bw() - width.get())) {
                                return Error("`ConcatFields` with bad concatenation");
                            }
                            total_width = total_width.checked_add(width.get()).unwrap();
                        }
                        EAwi::Bitwidth(t_w) => {
                            if (*width > *t_w) || (*from > (t_w.get() - width.get())) {
                                return Error("`ConcatFields` with bad concatenation");
                            }
                            total_width = total_width.checked_add(width.get()).unwrap();
                            known = false;
                        }
                    }
                }
                // should be impossible to be zero because the constructors require nonzero len
                // and bitwidth
                let total_width = NonZeroUsize::new(total_width).unwrap();
                if w != total_width {
                    return Error("`ConcatFields` with bad concatenation");
                }
                if known {
                    let mut r = Awi::zero(total_width);
                    let mut to = 0;
                    for (t, (from, width)) in concat.iter() {
                        match t {
                            EAwi::KnownAwi(t) => {
                                r.field(to, t, *from, width.get()).unwrap();
                                to += width.get();
                            }
                            EAwi::Bitwidth(_t_w) => {
                                unreachable!()
                            }
                        }
                    }
                    Valid(r)
                } else {
                    Unevaluatable
                }
            }
            StaticLut(concat, lit) => {
                let mut total_width = 0usize;
                let mut all_unknown = true;
                let mut all_known = true;
                for t in concat.as_slice() {
                    match t {
                        EAwi::KnownAwi(t) => {
                            total_width = total_width.checked_add(t.bw()).unwrap();
                            all_unknown = false;
                        }
                        EAwi::Bitwidth(t_w) => {
                            total_width = total_width.checked_add(t_w.get()).unwrap();
                            all_known = false;
                        }
                    }
                }
                // should be impossible to be zero because the constructors require nonzero len
                // and bitwidth
                let total_width = NonZeroUsize::new(total_width).unwrap();
                let mut good = false;
                if total_width.get() < USIZE_BITS {
                    if let Some(lut_len) = (1usize << total_width.get()).checked_mul(w.get()) {
                        if lut_len == lit.bw() {
                            good = true;
                        }
                    }
                }
                if !good {
                    return Error("`StaticLut` with bad bitwidths");
                }
                if all_known {
                    let mut inx = Awi::zero(total_width);
                    let mut to = 0;
                    for t in concat.as_slice() {
                        match t {
                            EAwi::KnownAwi(t) => {
                                let width = t.bw();
                                inx.field_to(to, t, width).unwrap();
                                to += width;
                            }
                            EAwi::Bitwidth(_t_w) => {
                                unreachable!()
                            }
                        }
                    }
                    let mut r = Awi::zero(w);
                    r.lut_(&lit, &inx).unwrap();
                    Valid(r)
                } else if all_unknown {
                    Unevaluatable
                } else {
                    // Check if the evaluation is the same when known bits are set to their values
                    // and all possible unknown bits are iterated through
                    let mut inx = Awi::zero(total_width);
                    let mut inx_known = Awi::zero(total_width);
                    let mut to = 0;
                    for t in concat.as_slice() {
                        match t {
                            EAwi::KnownAwi(t) => {
                                let width = t.bw();
                                inx.field_to(to, t, width).unwrap();
                                inx_known.field_to(to, &Awi::umax(t.nzbw()), width).unwrap();
                                to += width;
                            }
                            EAwi::Bitwidth(t_w) => {
                                to += t_w.get();
                            }
                        }
                    }
                    let inx = inx.to_usize();
                    let inx_known = inx_known.to_usize();
                    let mut test_inx = Awi::zero(total_width);
                    let mut common_eval = None;
                    let mut r = Awi::zero(w);
                    for i in 0..(1usize << total_width.get()) {
                        if (inx & inx_known) == (i & inx_known) {
                            test_inx.usize_(i);
                            r.lut_(&lit, &test_inx).unwrap();
                            if let Some(ref common_eval) = common_eval {
                                if *common_eval != r {
                                    return Unevaluatable
                                }
                            } else {
                                common_eval = Some(r.clone());
                            }
                        }
                    }
                    Valid(r)
                }
            }
            Resize([a, b]) => {
                awi2!(a, b, {
                    let mut r = Awi::zero(w);
                    r.resize_(&a, cbool!(&b));
                    Valid(r)
                })
            }
            ZeroResize([a]) => {
                awi1!(a, {
                    let mut r = Awi::zero(w);
                    r.zero_resize_(&a);
                    Valid(r)
                })
            }
            SignResize([a]) => {
                awi1!(a, {
                    let mut r = Awi::zero(w);
                    r.sign_resize_(&a);
                    Valid(r)
                })
            }
            Copy([a]) => {
                ceq_strict!(w, a.nzbw());
                awi1!(a, { Valid(a) })
            }
            Lut([a, b]) => {
                let mut res = false;
                if b.bw() < USIZE_BITS {
                    if let Some(lut_len) = (1usize << b.bw()).checked_mul(w.get()) {
                        if lut_len == a.bw() {
                            res = true;
                        }
                    }
                }
                if !res {
                    return Noop
                }
                // TODO some optimizing possible
                awi2!(a, b, {
                    let mut r = Awi::zero(w);
                    if r.lut_(&a, &b).is_some() {
                        Valid(r)
                    } else {
                        Unevaluatable
                    }
                })
            }
            Funnel([a, b]) => {
                if (b.bw() >= (USIZE_BITS - 1))
                    || ((1usize << b.bw()) != w.get())
                    || ((w.get() << 1) != a.bw())
                {
                    return Noop
                }
                awi2!(a, b, {
                    let mut r = Awi::zero(w);
                    if r.funnel_(&a, &b).is_some() {
                        Valid(r)
                    } else {
                        Unevaluatable
                    }
                })
            }
            CinSum([a, b, c]) => {
                ceq!(w, b.nzbw());
                ceq!(w, c.nzbw());
                cases!(a,
                    a => {
                        let a = cbool!(&a);
                        awi2!(b, c, {
                            let mut r = Awi::zero(b.nzbw());
                            r.cin_sum_(a, &b, &c).unwrap();
                            Valid(r)
                        })
                    },
                    a_w => {
                        cbool_w!(a_w);
                        Unevaluatable
                    },
                )
            }
            Not([a]) => {
                ceq_strict!(w, a.nzbw());
                awi1!(a, {
                    a.not_();
                    Valid(a)
                })
            }
            Rev([a]) => {
                ceq_strict!(w, a.nzbw());
                awi1!(a, {
                    a.rev_();
                    Valid(a)
                })
            }
            Abs([a]) => {
                ceq_strict!(w, a.nzbw());
                awi1!(a, {
                    a.abs_();
                    Valid(a)
                })
            }
            IsZero([a]) => unary_bit!(w, a, is_zero),
            IsUmax([a]) => unary_bit!(w, a, is_umax),
            IsImax([a]) => unary_bit!(w, a, is_imax),
            IsImin([a]) => unary_bit!(w, a, is_imin),
            IsUone([a]) => unary_bit!(w, a, is_uone),
            Lsb([a]) => unary_bit!(w, a, lsb),
            Msb([a]) => unary_bit!(w, a, msb),
            Lz([a]) => unary_usize!(w, a, lz),
            Tz([a]) => unary_usize!(w, a, tz),
            Sig([a]) => unary_usize!(w, a, sig),
            CountOnes([a]) => unary_usize!(w, a, count_ones),
            // `Or` and `And` are common enough that we want to use specialization to detect if one
            // or the other argument is all ones or zeros. This is particularly advantageous in bit
            // lines.
            Or([a, b]) => {
                ceq_strict!(w, a.nzbw());
                ceq!(w, b.nzbw());
                cases!(a,
                    a => {
                        cases!(b,
                            b => {
                                a.or_(&b).unwrap();
                                Valid(a)
                            },
                            _b_w => {
                                if a.is_umax() {
                                    Valid(a)
                                } else {
                                    Unevaluatable
                                }
                            },
                        )
                    },
                    _a_w => {
                        cases!(b,
                            b => {
                                if b.is_umax() {
                                    Valid(b)
                                } else {
                                    Unevaluatable
                                }
                            },
                            _b_w => {
                                Unevaluatable
                            },
                        )
                    },
                )
            }
            And([a, b]) => {
                ceq_strict!(w, a.nzbw());
                ceq!(w, b.nzbw());
                cases!(a,
                    a => {
                        cases!(b,
                            b => {
                                a.and_(&b).unwrap();
                                Valid(a)
                            },
                            _b_w => {
                                if a.is_zero() {
                                    Valid(a)
                                } else {
                                    Unevaluatable
                                }
                            },
                        )
                    },
                    _a_w => {
                        cases!(b,
                            b => {
                                if b.is_zero() {
                                    Valid(b)
                                } else {
                                    Unevaluatable
                                }
                            },
                            _b_w => {
                                Unevaluatable
                            },
                        )
                    },
                )
            }
            Xor([a, b]) => binary!(w, a, b, xor_),
            Shl([a, b]) => shift!(w, a, b, shl_),
            Lshr([a, b]) => shift!(w, a, b, lshr_),
            Ashr([a, b]) => shift!(w, a, b, ashr_),
            Rotl([a, b]) => shift!(w, a, b, rotl_),
            Rotr([a, b]) => shift!(w, a, b, rotr_),
            Add([a, b]) => binary!(w, a, b, add_),
            Sub([a, b]) => binary!(w, a, b, sub_),
            Rsb([a, b]) => binary!(w, a, b, rsb_),
            Eq([a, b]) => cmp!(w, a, b, const_eq),
            Ne([a, b]) => cmp!(w, a, b, const_ne),
            Ult([a, b]) => cmp!(w, a, b, ult),
            Ule([a, b]) => cmp!(w, a, b, ule),
            Ilt([a, b]) => cmp!(w, a, b, ilt),
            Ile([a, b]) => cmp!(w, a, b, ile),
            RangeOr([a, b, c]) => range_op!(w, a, b, c, range_or_),
            RangeAnd([a, b, c]) => range_op!(w, a, b, c, range_and_),
            RangeXor([a, b, c]) => range_op!(w, a, b, c, range_xor_),
            Inc([a, b]) => {
                ceq_strict!(w, a.nzbw());
                cases!(b,
                    b => {
                        let b = cbool!(&b);
                        awi1!(a, {
                            a.inc_(b);
                            Valid(a)
                        })
                    },
                    b_w => {
                        cbool_w!(b_w);
                        Unevaluatable
                    },
                )
            }
            Dec([a, b]) => {
                ceq_strict!(w, a.nzbw());
                cases!(b,
                    b => {
                        let b = cbool!(&b);
                        awi1!(a, {
                            a.dec_(b);
                            Valid(a)
                        })
                    },
                    b_w => {
                        cbool_w!(b_w);
                        Unevaluatable
                    },
                )
            }
            Neg([a, b]) => {
                ceq_strict!(w, a.nzbw());
                cases!(b,
                    b => {
                        let b = cbool!(&b);
                        awi1!(a, {
                            a.neg_(b);
                            Valid(a)
                        })
                    },
                    b_w => {
                        cbool_w!(b_w);
                        Unevaluatable
                    },
                )
            }
            ZeroResizeOverflow([a], lhs_w) => {
                cbool_w!(w);
                awi1!(a, {
                    let mut tmp_awi = Awi::zero(lhs_w);
                    Valid(Awi::from_bool(tmp_awi.zero_resize_(&a)))
                })
            }
            SignResizeOverflow([a], lhs_w) => {
                cbool_w!(w);
                awi1!(a, {
                    let mut tmp_awi = Awi::zero(lhs_w);
                    Valid(Awi::from_bool(tmp_awi.sign_resize_(&a)))
                })
            }
            Get([a, b]) => {
                cbool_w!(w);
                cases!(b,
                    b => {
                        let b = cusize!(b);
                        cases!(a,
                            a => {
                                if let Some(res) = a.get(b) {
                                    Valid(Awi::from_bool(res))
                                } else {
                                    Noop
                                }
                            },
                            a_w => {
                                if b >= a_w.get() {
                                    Noop
                                } else {
                                    Unevaluatable
                                }
                            },
                        )
                    },
                    b_w => {
                        cusize_w!(b_w);
                        Unevaluatable
                    },
                )
            }
            Set([a, b, c]) => {
                ceq_strict!(w, a.nzbw());
                cbool_w!(c.nzbw());
                cusize_w!(b.nzbw());
                cases!(b,
                    b => {
                        let b = cusize!(b);
                        cases!(a,
                            a => {
                                cases!(c,
                                    c => {
                                        let c = cbool!(&c);
                                        if a.set(b, c).is_some() {
                                            Valid(a)
                                        } else {
                                            Pass(a)
                                        }
                                    },
                                    _c_w => {
                                        if b >= w.get() {
                                            Pass(a)
                                        } else {
                                            Unevaluatable
                                        }
                                    },
                                )
                            },
                            _a_w => {
                                if b >= w.get() {
                                    PassUnevaluatable
                                } else {
                                    Unevaluatable
                                }
                            },
                        )
                    },
                    _b_w => {
                        Unevaluatable
                    },
                )
            }
            Mux([a, b, c]) => {
                ceq_strict!(w, a.nzbw());
                ceq!(w, b.nzbw());
                cases!(c,
                    c => {
                        if cbool!(&c) {
                            awi1!(b, {
                                Valid(b)
                            })
                        } else {
                            awi1!(a, {
                                Valid(a)
                            })
                        }
                    },
                    c_w => {
                        cbool_w!(c_w);
                        Unevaluatable
                    },
                )
            }
            LutSet([a, b, c]) => {
                ceq_strict!(w, a.nzbw());
                let mut res = false;
                if c.bw() < USIZE_BITS {
                    if let Some(lut_len) = (1usize << c.bw()).checked_mul(b.bw()) {
                        if lut_len == a.bw() {
                            res = w == a.nzbw();
                        }
                    }
                }
                if !res {
                    return Noop
                }
                awi3!(a, b, c, {
                    a.lut_set(&b, &c).unwrap();
                    Valid(a)
                })
            }
            FieldWidth([a, b, c]) => {
                ceq_strict!(w, a.nzbw());
                cases!(c,
                    c => {
                        let c = cusize!(c);
                        let o = (c > a.bw()) || (c > b.bw());
                        cases!(a,
                            a => {
                                cases!(b,
                                    b => {
                                        if a.field_width(&b, c).is_some() {
                                            Valid(a)
                                        } else {
                                            Pass(a)
                                        }
                                    },
                                    _b_w => {
                                        if o {
                                            Pass(a)
                                        } else {
                                            Unevaluatable
                                        }
                                    },
                                )
                            },
                            _a_w => {
                                if o {
                                    PassUnevaluatable
                                } else {
                                    Unevaluatable
                                }
                            },
                        )
                    },
                    c_w => {
                        cusize_w!(c_w);
                        Unevaluatable
                    },
                )
            }
            FieldFrom([a, b, c, d]) => {
                ceq_strict!(w, a.nzbw());
                let mut o = false;
                cases!(d,
                    d => {
                        let d = cusize!(d);
                        o |= (d > a.bw()) || (d > b.bw());
                        cases!(c,
                            c => {
                                let c = cusize!(c);
                                o |= c > b.bw().saturating_sub(d);

                                return if o {
                                    cases!(a,
                                        a => {
                                            Pass(a)
                                        },
                                        _a_w => {
                                            PassUnevaluatable
                                        },
                                    )
                                } else {
                                    awi2!(a, b, {
                                        a.field_from(&b, c, d).unwrap();
                                        Valid(a)
                                    })
                                };
                            },
                            c_w => {
                                cusize_w!(c_w);
                            },
                        )
                    },
                    d_w => {
                        cusize_w!(d_w);
                        cases!(c,
                            c => {
                                let c = cusize!(c);
                                o |= c > b.bw();
                            },
                            c_w => {
                                cusize_w!(c_w);
                            },
                        )
                    },
                );
                if o {
                    PassUnevaluatable
                } else {
                    Unevaluatable
                }
            }
            FieldTo([a, b, c, d]) => {
                ceq_strict!(w, a.nzbw());
                let mut o = false;
                cases!(d,
                    d => {
                        let d = cusize!(d);
                        o |= (d > a.bw()) || (d > c.bw());
                        cases!(b,
                            b => {
                                let b = cusize!(b);
                                o |= b > a.bw().saturating_sub(d);

                                return if o {
                                    cases!(a,
                                        a => {
                                            Pass(a)
                                        },
                                        _a_w => {
                                            PassUnevaluatable
                                        },
                                    )
                                } else {
                                    awi2!(a, c, {
                                        a.field_to(b, &c, d).unwrap();
                                        Valid(a)
                                    })
                                };
                            },
                            b_w => {
                                cusize_w!(b_w);
                            },
                        )
                    },
                    d_w => {
                        cusize_w!(d_w);
                        cases!(b,
                            b => {
                                let b = cusize!(b);
                                o |= b > a.bw();
                            },
                            b_w => {
                                cusize_w!(b_w);
                            },
                        )
                    },
                );
                if o {
                    PassUnevaluatable
                } else {
                    Unevaluatable
                }
            }
            Field([a, b, c, d, e]) => {
                ceq_strict!(w, a.nzbw());
                let mut o = false;
                cases!(e,
                    e => {
                        let e = cusize!(e);
                        o |= (e > a.bw()) || (e > c.bw());
                        cases!(b,
                            b => {
                                let b = cusize!(b);
                                o |= b > a.bw().saturating_sub(e);
                                cases!(d,
                                    d => {
                                        let d = cusize!(&d);
                                        o |= d > c.bw().saturating_sub(e);
                                        return if o {
                                            cases!(a,
                                                a => {
                                                    Pass(a)
                                                },
                                                _a_w => {
                                                    PassUnevaluatable
                                                },
                                            )
                                        } else {
                                            awi2!(a, c, {
                                                a.field(b, &c, d, e).unwrap();
                                                Valid(a)
                                            })
                                        };
                                    },
                                    d_w => {
                                        cusize_w!(d_w);
                                    },
                                );
                            },
                            b_w => {
                                cusize_w!(b_w);
                                cases!(d,
                                    d => {
                                        let d = cusize!(&d);
                                        o |= d > c.bw().saturating_sub(e);
                                    },
                                    d_w => {
                                        cusize_w!(d_w);
                                    },
                                );
                            },
                        )
                    },
                    e_w => {
                        cusize_w!(e_w);
                        cases!(b,
                            b => {
                                let b = cusize!(&b);
                                o |= b > a.bw();
                            },
                            b_w => {
                                cusize_w!(b_w);
                            },
                        );
                        cases!(d,
                            d => {
                                let d = cusize!(&d);
                                o |= d > c.bw();
                            },
                            d_w => {
                                cusize_w!(d_w);
                            },
                        );
                    },
                );
                if o {
                    PassUnevaluatable
                } else {
                    Unevaluatable
                }
            }
            FieldBit([a, b, c, d]) => {
                ceq_strict!(w, a.nzbw());
                let mut o = false;
                cases!(b,
                    b => {
                        let b = cusize!(b);
                        o |= b > a.bw().wrapping_sub(1);
                        cases!(d,
                            d => {
                                let d = cusize!(&d);
                                o |= d > c.bw().wrapping_sub(1);
                                return if o {
                                    cases!(a,
                                        a => {
                                            Pass(a)
                                        },
                                        _a_w => {
                                            PassUnevaluatable
                                        },
                                    )
                                } else {
                                    awi2!(a, c, {
                                        a.field_bit(b, &c, d).unwrap();
                                        Valid(a)
                                    })
                                };
                            },
                            d_w => {
                                cusize_w!(d_w);
                            },
                        );
                    },
                    b_w => {
                        cusize_w!(b_w);
                        cases!(d,
                            d => {
                                let d = cusize!(&d);
                                o |= d > c.bw().wrapping_sub(1);
                            },
                            d_w => {
                                cusize_w!(d_w);
                            },
                        );
                    },
                );
                if o {
                    PassUnevaluatable
                } else {
                    Unevaluatable
                }
            }
            ArbMulAdd([a, b, c]) => {
                ceq_strict!(w, a.nzbw());
                awi3!(a, b, c, {
                    a.arb_umul_add_(&b, &c);
                    Valid(a)
                })
            }
            UnsignedOverflow([a, b, c]) => {
                cbool_w!(w);
                cases!(a,
                    a => {
                        let a = cbool!(&a);
                        awi2!(b, c, {
                            let mut t = Awi::zero(b.nzbw());
                            if let Some((o, _)) = t.cin_sum_(a, &b, &c) {
                                Valid(Awi::from_bool(o))
                            } else {
                                Noop
                            }
                        })
                    },
                    a_w => {
                        cbool_w!(a_w);
                        ceq!(b.nzbw(), c.nzbw());
                        Unevaluatable
                    },
                )
            }
            SignedOverflow([a, b, c]) => {
                cbool_w!(w);
                cases!(a,
                    a => {
                        let a = cbool!(&a);
                        awi2!(b, c, {
                            let mut t = Awi::zero(b.nzbw());
                            if let Some((_, o)) = t.cin_sum_(a, &b, &c) {
                                Valid(Awi::from_bool(o))
                            } else {
                                Noop
                            }
                        })
                    },
                    a_w => {
                        cbool_w!(a_w);
                        ceq!(b.nzbw(), c.nzbw());
                        Unevaluatable
                    },
                )
            }
            IncCout([a, b]) => {
                cbool_w!(w);
                cases!(b,
                    b => {
                        let b = cbool!(&b);
                        awi1!(a, {
                            Valid(Awi::from_bool(a.inc_(b)))
                        })
                    },
                    b_w => {
                        cbool_w!(b_w);
                        Unevaluatable
                    },
                )
            }
            DecCout([a, b]) => {
                cbool_w!(w);
                cases!(b,
                    b => {
                        let b = cbool!(&b);
                        awi1!(a, {
                            Valid(Awi::from_bool(a.dec_(b)))
                        })
                    },
                    b_w => {
                        cbool_w!(b_w);
                        Unevaluatable
                    },
                )
            }
            UQuo([a, b]) => {
                // Noop needs to take precedence
                ceq!(w, a.nzbw());
                ceq!(w, b.nzbw());
                cases!(a,
                    a => {
                        awi1!(b, {
                            if b.is_zero() {
                                Pass(a)
                            } else {
                                let mut r = Awi::zero(w);
                                let mut t = Awi::zero(w);
                                Bits::udivide(&mut r, &mut t, &a, &b).unwrap();
                                Valid(r)
                            }
                        })
                    },
                    _a_w => {
                        awi1!(b, {
                            if b.is_zero() {
                                PassUnevaluatable
                            } else {
                                Unevaluatable
                            }
                        })
                    },
                )
            }
            URem([a, b]) => {
                ceq!(w, a.nzbw());
                ceq!(w, b.nzbw());
                cases!(a,
                    a => {
                        awi1!(b, {
                            if b.is_zero() {
                                Pass(a)
                            } else {
                                let mut t = Awi::zero(w);
                                let mut r = Awi::zero(w);
                                Bits::udivide(&mut t, &mut r, &a, &b).unwrap();
                                Valid(r)
                            }
                        })
                    },
                    _a_w => {
                        awi1!(b, {
                            if b.is_zero() {
                                PassUnevaluatable
                            } else {
                                Unevaluatable
                            }
                        })
                    },
                )
            }
            IQuo([a, b]) => {
                ceq!(w, a.nzbw());
                ceq!(w, b.nzbw());
                cases!(a,
                    a => {
                        awi1!(b, {
                            if b.is_zero() {
                                Pass(a)
                            } else {
                                let mut r = Awi::zero(w);
                                let mut t = Awi::zero(w);
                                Bits::idivide(&mut r, &mut t, &mut a, &mut b).unwrap();
                                Valid(r)
                            }
                        })
                    },
                    _a_w => {
                        awi1!(b, {
                            if b.is_zero() {
                                PassUnevaluatable
                            } else {
                                Unevaluatable
                            }
                        })
                    },
                )
            }
            IRem([a, b]) => {
                ceq!(w, a.nzbw());
                ceq!(w, b.nzbw());
                cases!(a,
                    a => {
                        awi1!(b, {
                            if b.is_zero() {
                                Pass(a)
                            } else {
                                let mut t = Awi::zero(w);
                                let mut r = Awi::zero(w);
                                Bits::idivide(&mut t, &mut r, &mut a, &mut b).unwrap();
                                Valid(r)
                            }
                        })
                    },
                    _a_w => {
                        awi1!(b, {
                            if b.is_zero() {
                                PassUnevaluatable
                            } else {
                                Unevaluatable
                            }
                        })
                    },
                )
            }
        }
    }
}
